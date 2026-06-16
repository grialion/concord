use std::collections::{HashMap, HashSet};

use image::DynamicImage;
use ratatui_image::{picker::Picker, protocol::Protocol};

use crate::{
    discord::{AppCommand, AppEvent},
    tui::ui::EmojiImage,
};

use super::{
    EmojiImageTarget,
    decode::{MediaImageDecodeJob, MediaImageDecodeKey},
    emoji_protocol,
    lru::{self, TrackedCacheEntry},
    query_image_picker,
};

/// Cap on the URL-keyed emoji image cache. Each entry is a small terminal
/// protocol payload, so 256 or 128 fits realistic loads and bounds worst-case
/// memory if many unique emoji ids arrive.
pub(super) const MAX_EMOJI_IMAGE_CACHE_ENTRIES: usize = 128;

pub(in crate::tui) struct EmojiImageCache {
    pub(super) picker: Option<Picker>,
    pub(super) entries: HashMap<String, EmojiImageEntry>,
    pub(super) tick: u64,
    pub(super) decode_generation: u64,
    pub(super) protocol_generation: u64,
}

pub(super) enum EmojiImageEntry {
    Loading {
        last_used: u64,
    },
    Decoding {
        generation: u64,
        last_used: u64,
    },
    Ready {
        image: DynamicImage,
        protocol: Protocol,
        protocol_generation: u64,
        last_used: u64,
    },
    Failed {
        last_used: u64,
    },
}

impl TrackedCacheEntry for EmojiImageEntry {
    fn last_used(&self) -> u64 {
        match self {
            EmojiImageEntry::Loading { last_used }
            | EmojiImageEntry::Decoding { last_used, .. }
            | EmojiImageEntry::Ready { last_used, .. }
            | EmojiImageEntry::Failed { last_used } => *last_used,
        }
    }

    fn touch(&mut self, tick: u64) {
        match self {
            EmojiImageEntry::Loading { last_used }
            | EmojiImageEntry::Decoding { last_used, .. }
            | EmojiImageEntry::Ready { last_used, .. }
            | EmojiImageEntry::Failed { last_used } => *last_used = tick,
        }
    }
}

impl EmojiImageCache {
    pub(in crate::tui) fn new() -> Self {
        Self {
            picker: query_image_picker("emoji", "emoji image picker unavailable"),
            entries: HashMap::new(),
            tick: 0,
            decode_generation: 0,
            protocol_generation: 0,
        }
    }

    pub(in crate::tui) fn refresh_protocols(&mut self) {
        self.protocol_generation = self.protocol_generation.saturating_add(1);
    }

    /// Returns decoded protocols for visible targets and refreshes their
    /// LRU timestamps so they survive the next pruning pass.
    pub(in crate::tui) fn render_state(
        &mut self,
        targets: &[EmojiImageTarget],
    ) -> Vec<EmojiImage<'_>> {
        let touch_tick = lru::next_tick(&mut self.tick);
        let picker = self.picker.clone();
        let protocol_generation = self.protocol_generation;
        for target in targets {
            if let Some(entry) = self.entries.get_mut(&target.url) {
                entry.touch(touch_tick);
                if let EmojiImageEntry::Ready {
                    image,
                    protocol,
                    protocol_generation: entry_protocol_generation,
                    ..
                } = entry
                    && *entry_protocol_generation != protocol_generation
                    && let Some(picker) = picker.as_ref()
                    && let Some(updated_protocol) = emoji_protocol(picker, image.clone())
                {
                    *protocol = updated_protocol;
                    *entry_protocol_generation = protocol_generation;
                }
            }
        }
        targets
            .iter()
            .filter_map(|target| {
                let EmojiImageEntry::Ready { protocol, .. } = self.entries.get(&target.url)? else {
                    return None;
                };
                Some(EmojiImage {
                    url: target.url.clone(),
                    protocol,
                })
            })
            .collect()
    }

    pub(in crate::tui) fn next_requests(
        &mut self,
        targets: &[EmojiImageTarget],
    ) -> Vec<AppCommand> {
        if self.picker.is_none() {
            return Vec::new();
        }

        let mut intents = Vec::new();
        for target in targets.iter().take(MAX_EMOJI_IMAGE_CACHE_ENTRIES) {
            if let Some(intent) = lru::insert_loading_request(
                &mut self.entries,
                &mut self.tick,
                target.url.clone(),
                |last_used| EmojiImageEntry::Loading { last_used },
                |url| AppCommand::LoadAttachmentPreview { url },
            ) {
                intents.push(intent);
            }
        }
        self.prune_to_limit(targets);
        intents
    }

    pub(in crate::tui) fn record_event(&mut self, event: &AppEvent) -> Option<MediaImageDecodeJob> {
        match event {
            AppEvent::AttachmentPreviewLoaded { url, bytes } => self.store_loaded(url, bytes),
            AppEvent::AttachmentPreviewLoadFailed { url, .. } => {
                self.store_failed(url);
                None
            }
            _ => None,
        }
    }

    /// Drops LRU entries while protecting URLs in the current frame's
    /// targets so a flood of unique ids can never evict what is on screen.
    pub(super) fn prune_to_limit(&mut self, targets: &[EmojiImageTarget]) {
        let protected: HashSet<&str> = targets
            .iter()
            .take(MAX_EMOJI_IMAGE_CACHE_ENTRIES)
            .map(|target| target.url.as_str())
            .collect();
        lru::prune_to_limit(
            &mut self.entries,
            MAX_EMOJI_IMAGE_CACHE_ENTRIES,
            |url| protected.contains(url.as_str()),
            TrackedCacheEntry::last_used,
        );
    }

    fn store_loaded(&mut self, url: &str, bytes: &[u8]) -> Option<MediaImageDecodeJob> {
        lru::start_url_decode_job(
            (
                &mut self.entries,
                &mut self.tick,
                &mut self.decode_generation,
            ),
            (url, bytes),
            self.picker.is_some(),
            |entry| matches!(entry, EmojiImageEntry::Loading { .. }),
            |generation, last_used| EmojiImageEntry::Decoding {
                generation,
                last_used,
            },
            |last_used| EmojiImageEntry::Failed { last_used },
            MediaImageDecodeKey::Emoji,
        )
    }

    pub(in crate::tui) fn store_decoded(
        &mut self,
        url: String,
        result_generation: u64,
        result: std::result::Result<DynamicImage, String>,
    ) {
        let Some(generation) = self.entries.get(&url).and_then(|entry| {
            if let EmojiImageEntry::Decoding { generation, .. } = entry {
                Some(*generation)
            } else {
                None
            }
        }) else {
            return;
        };

        if generation != result_generation {
            return;
        }

        let last_used = lru::next_tick(&mut self.tick);
        match result {
            Ok(image) => {
                let Some(picker) = self.picker.as_ref() else {
                    self.entries
                        .insert(url, EmojiImageEntry::Failed { last_used });
                    return;
                };
                let Some(protocol) = emoji_protocol(picker, image.clone()) else {
                    self.entries
                        .insert(url, EmojiImageEntry::Failed { last_used });
                    return;
                };
                self.entries.insert(
                    url,
                    EmojiImageEntry::Ready {
                        image,
                        protocol,
                        protocol_generation: self.protocol_generation,
                        last_used,
                    },
                );
            }
            Err(_) => {
                self.entries
                    .insert(url, EmojiImageEntry::Failed { last_used });
            }
        }
    }

    fn store_failed(&mut self, url: &str) {
        if self.entries.contains_key(url) {
            let last_used = lru::next_tick(&mut self.tick);
            self.entries
                .insert(url.to_owned(), EmojiImageEntry::Failed { last_used });
        }
    }
}

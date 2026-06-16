use std::{collections::HashMap, hash::Hash, sync::Arc};

use crate::discord::AppCommand;

use super::decode::{MediaImageDecodeJob, MediaImageDecodeKey};

pub(super) fn next_tick(tick: &mut u64) -> u64 {
    *tick = tick.saturating_add(1);
    *tick
}

pub(super) fn next_generation(generation: &mut u64) -> u64 {
    *generation = generation.saturating_add(1);
    *generation
}

pub(super) trait TrackedCacheEntry {
    fn last_used(&self) -> u64;

    fn touch(&mut self, tick: u64);
}

pub(super) fn touch_entry(entry: &mut impl TrackedCacheEntry, tick: &mut u64) {
    entry.touch(next_tick(tick));
}

pub(super) fn insert_loading_request<E>(
    entries: &mut HashMap<String, E>,
    tick: &mut u64,
    url: String,
    make_loading: impl FnOnce(u64) -> E,
    make_command: impl FnOnce(String) -> AppCommand,
) -> Option<AppCommand> {
    if entries.contains_key(&url) {
        return None;
    }
    let last_used = next_tick(tick);
    entries.insert(url.clone(), make_loading(last_used));
    Some(make_command(url))
}

pub(super) fn start_url_decode_job<E>(
    cache: (&mut HashMap<String, E>, &mut u64, &mut u64),
    payload: (&str, &[u8]),
    picker_available: bool,
    is_loading: impl FnOnce(&E) -> bool,
    make_decoding: impl FnOnce(u64, u64) -> E,
    make_failed: impl FnOnce(u64) -> E,
    make_key: impl FnOnce(String) -> MediaImageDecodeKey,
) -> Option<MediaImageDecodeJob> {
    let (entries, tick, decode_generation) = cache;
    let (url, bytes) = payload;
    if !entries.get(url).is_some_and(is_loading) {
        return None;
    }
    let last_used = next_tick(tick);
    if !picker_available {
        entries.insert(url.to_owned(), make_failed(last_used));
        return None;
    }

    let generation = next_generation(decode_generation);
    entries.insert(url.to_owned(), make_decoding(generation, last_used));
    Some(MediaImageDecodeJob {
        key: make_key(url.to_owned()),
        generation,
        bytes: Arc::from(bytes.to_vec()),
    })
}

pub(super) fn prune_to_limit<K, V>(
    entries: &mut HashMap<K, V>,
    limit: usize,
    is_protected: impl Fn(&K) -> bool,
    last_used: impl Fn(&V) -> u64,
) where
    K: Clone + Eq + Hash,
{
    if entries.len() <= limit {
        return;
    }

    let mut removable = entries
        .iter()
        .filter(|(key, _)| !is_protected(key))
        .map(|(key, entry)| (key.clone(), last_used(entry)))
        .collect::<Vec<_>>();
    removable.sort_by_key(|(_, last_used)| *last_used);

    for (key, _) in removable {
        if entries.len() <= limit {
            break;
        }
        entries.remove(&key);
    }
}

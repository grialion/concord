use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use crate::discord::ids::{
    Id,
    marker::{ChannelMarker, GuildMarker, MessageMarker, UserMarker},
};

use crate::discord::{AppEvent, ForumPostArchiveState};

#[derive(Default)]
pub(super) struct HistoryRequests {
    requests: HashMap<Id<ChannelMarker>, HistoryRequestState>,
    last_channel: Option<Id<ChannelMarker>>,
}

#[derive(Default)]
pub(super) struct ForumPostRequests {
    requests: HashMap<Id<ChannelMarker>, ForumPostRequestState>,
    last_channel: Option<Id<ChannelMarker>>,
}

#[derive(Default)]
pub(super) struct PinnedMessageRequests {
    requests: HashMap<Id<ChannelMarker>, PinnedMessageRequestState>,
    last_channel: Option<Id<ChannelMarker>>,
}

pub(super) struct ForumPostRequestTarget {
    pub(super) guild_id: Id<GuildMarker>,
    pub(super) channel_id: Id<ChannelMarker>,
    pub(super) should_load_more: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MentionMemberSearchTarget {
    pub(super) guild_id: Id<GuildMarker>,
    pub(super) query: String,
}

#[derive(Default)]
pub(super) struct MessageAuthorMemberRequests {
    requested: HashMap<MessageAuthorMemberRequestKey, Instant>,
    requested_order: VecDeque<MessageAuthorMemberRequestKey>,
}

pub(super) struct MemberListSubscriptionTarget {
    pub(super) guild_id: Id<GuildMarker>,
    pub(super) channel_id: Id<ChannelMarker>,
    pub(super) bucket: u32,
    pub(super) ranges: Vec<(u32, u32)>,
}

#[derive(Default)]
pub(super) struct MemberListSubscriptionRequests {
    last_sent: Option<MemberListSubscriptionKey>,
    pending: Option<PendingMemberListSubscription>,
}

#[derive(Default)]
pub(super) struct MentionMemberSearchRequests {
    requested: HashMap<MentionMemberSearchKey, Instant>,
    requested_order: VecDeque<MentionMemberSearchKey>,
    pending: Option<PendingMentionMemberSearch>,
}

impl HistoryRequests {
    pub(super) fn record_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::MessageHistoryLoaded { channel_id, .. } => {
                self.requests
                    .insert(*channel_id, HistoryRequestState::Loaded);
            }
            AppEvent::MessageHistoryLoadFailed { channel_id, .. } => {
                self.mark_failed(*channel_id);
            }
            _ => {}
        }
    }

    pub(super) fn next(
        &mut self,
        channel_id: Option<Id<ChannelMarker>>,
        force_reload: bool,
    ) -> Option<Id<ChannelMarker>> {
        let Some(channel_id) = channel_id else {
            self.last_channel = None;
            return None;
        };
        let channel_changed = self.last_channel != Some(channel_id);
        self.last_channel = Some(channel_id);

        match self.requests.get(&channel_id).copied() {
            None => {
                self.requests
                    .insert(channel_id, HistoryRequestState::Requested);
                Some(channel_id)
            }
            Some(HistoryRequestState::Failed) if channel_changed => {
                self.requests
                    .insert(channel_id, HistoryRequestState::Requested);
                Some(channel_id)
            }
            Some(HistoryRequestState::Loaded) if force_reload && channel_changed => {
                self.requests
                    .insert(channel_id, HistoryRequestState::Requested);
                Some(channel_id)
            }
            Some(
                HistoryRequestState::Requested
                | HistoryRequestState::Loaded
                | HistoryRequestState::Failed,
            ) => None,
        }
    }

    pub(super) fn mark_failed(&mut self, channel_id: Id<ChannelMarker>) {
        self.requests
            .insert(channel_id, HistoryRequestState::Failed);
    }
}

impl ForumPostRequests {
    pub(super) fn record_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::ForumPostsLoaded {
                channel_id,
                archive_state,
                offset: _,
                next_offset,
                has_more,
                ..
            } => {
                self.requests.entry(*channel_id).or_default().set_loaded(
                    *archive_state,
                    *next_offset,
                    *has_more,
                );
            }
            AppEvent::ForumPostsLoadFailed {
                channel_id,
                archive_state,
                offset,
                ..
            } => {
                self.mark_failed(*channel_id, *archive_state, *offset);
            }
            _ => {}
        }
    }

    pub(super) fn next(
        &mut self,
        target: Option<ForumPostRequestTarget>,
    ) -> Option<(
        Id<GuildMarker>,
        Id<ChannelMarker>,
        ForumPostArchiveState,
        usize,
    )> {
        let Some(ForumPostRequestTarget {
            guild_id,
            channel_id,
            should_load_more,
        }) = target
        else {
            self.last_channel = None;
            return None;
        };
        let channel_changed = self.last_channel != Some(channel_id);
        self.last_channel = Some(channel_id);

        let state = self.requests.entry(channel_id).or_default();
        let next = state.next(channel_changed, should_load_more)?;
        Some((guild_id, channel_id, next.archive_state, next.offset))
    }

    pub(super) fn mark_failed(
        &mut self,
        channel_id: Id<ChannelMarker>,
        archive_state: ForumPostArchiveState,
        offset: usize,
    ) {
        self.requests
            .entry(channel_id)
            .or_default()
            .set_failed(archive_state, offset);
    }
}

impl PinnedMessageRequests {
    pub(super) fn record_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::PinnedMessagesLoaded { channel_id, .. } => {
                self.requests
                    .insert(*channel_id, PinnedMessageRequestState::Loaded);
            }
            AppEvent::PinnedMessagesLoadFailed { channel_id, .. } => {
                self.mark_failed(*channel_id);
            }
            _ => {}
        }
    }

    pub(super) fn next(
        &mut self,
        channel_id: Option<Id<ChannelMarker>>,
    ) -> Option<Id<ChannelMarker>> {
        let Some(channel_id) = channel_id else {
            self.last_channel = None;
            return None;
        };
        let channel_changed = self.last_channel != Some(channel_id);
        self.last_channel = Some(channel_id);

        match self.requests.get(&channel_id).copied() {
            None => {
                self.requests
                    .insert(channel_id, PinnedMessageRequestState::Requested);
                Some(channel_id)
            }
            Some(PinnedMessageRequestState::Failed) if channel_changed => {
                self.requests
                    .insert(channel_id, PinnedMessageRequestState::Requested);
                Some(channel_id)
            }
            Some(
                PinnedMessageRequestState::Requested
                | PinnedMessageRequestState::Loaded
                | PinnedMessageRequestState::Failed,
            ) => None,
        }
    }

    pub(super) fn mark_failed(&mut self, channel_id: Id<ChannelMarker>) {
        self.requests
            .insert(channel_id, PinnedMessageRequestState::Failed);
    }
}

impl MessageAuthorMemberRequests {
    const REQUEST_TTL: Duration = Duration::from_secs(30);
    const MAX_REQUESTED: usize = 4096;

    pub(super) fn record_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::GuildMemberUpsert { guild_id, member }
            | AppEvent::GuildMemberAdd { guild_id, member } => {
                self.remove((*guild_id, member.user_id));
            }
            _ => {}
        }
    }

    pub(super) fn next(
        &mut self,
        missing: Vec<(Id<GuildMarker>, Vec<Id<UserMarker>>)>,
        now: Instant,
    ) -> Vec<(Id<GuildMarker>, Vec<Id<UserMarker>>)> {
        self.prune_requested(now);

        let mut requests = Vec::new();
        for (guild_id, user_ids) in missing {
            let fresh_user_ids = user_ids
                .into_iter()
                .filter(|user_id| self.insert_requested((guild_id, *user_id), now))
                .collect::<Vec<_>>();
            if !fresh_user_ids.is_empty() {
                requests.push((guild_id, fresh_user_ids));
            }
        }
        requests
    }

    fn insert_requested(&mut self, key: MessageAuthorMemberRequestKey, now: Instant) -> bool {
        if self.requested.contains_key(&key) {
            return false;
        }
        self.requested.insert(key, now);
        self.requested_order.push_back(key);
        self.prune_requested(now);
        true
    }

    fn remove(&mut self, key: MessageAuthorMemberRequestKey) {
        self.requested.remove(&key);
        self.requested_order
            .retain(|requested_key| requested_key != &key);
    }

    fn prune_requested(&mut self, now: Instant) {
        self.requested.retain(|_, requested_at| {
            now.checked_duration_since(*requested_at)
                .is_none_or(|age| age <= Self::REQUEST_TTL)
        });
        self.requested_order
            .retain(|key| self.requested.contains_key(key));
        while self.requested.len() > Self::MAX_REQUESTED {
            let Some(oldest) = self.requested_order.pop_front() else {
                break;
            };
            self.requested.remove(&oldest);
        }
    }
}

impl MemberListSubscriptionRequests {
    const DEBOUNCE: Duration = Duration::from_millis(100);

    pub(super) fn set_target(
        &mut self,
        target: Option<MemberListSubscriptionTarget>,
        now: Instant,
    ) {
        let Some(target) = target else {
            self.pending = None;
            self.last_sent = None;
            return;
        };
        let key = target.key();

        // The initial guild subscription already covers bucket 0. Only send a
        // bucket-0 update when it resets a previously wider subscription.
        if self.last_sent.is_none() && key.bucket == 0 {
            self.pending = None;
            return;
        }
        if self.last_sent.as_ref() == Some(&key) {
            self.pending = None;
            return;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.target.key() == key)
        {
            return;
        }
        self.pending = Some(PendingMemberListSubscription {
            target,
            ready_at: now + Self::DEBOUNCE,
        });
    }

    pub(super) fn pending_deadline(&self) -> Option<Instant> {
        self.pending.as_ref().map(|pending| pending.ready_at)
    }

    pub(super) fn next_due(&mut self, now: Instant) -> Option<MemberListSubscriptionTarget> {
        let pending = self.pending.as_ref()?;
        if pending.ready_at > now {
            return None;
        }
        let pending = self.pending.take()?;
        self.last_sent = Some(pending.target.key());
        Some(pending.target)
    }
}

#[derive(Default)]
pub(super) struct MemberRequests {
    requests: HashSet<Id<GuildMarker>>,
}

#[derive(Default)]
pub(super) struct ThreadPreviewRequests {
    requested: HashSet<(Id<ChannelMarker>, Id<MessageMarker>)>,
    failed: HashSet<(Id<ChannelMarker>, Id<MessageMarker>)>,
}

impl MemberRequests {
    pub(super) fn next(&mut self, guild_id: Option<Id<GuildMarker>>) -> Option<Id<GuildMarker>> {
        let guild_id = guild_id?;
        self.requests.insert(guild_id).then_some(guild_id)
    }

    pub(super) fn remove(&mut self, guild_id: Id<GuildMarker>) {
        self.requests.remove(&guild_id);
    }
}

impl ThreadPreviewRequests {
    pub(super) fn record_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::ThreadPreviewLoaded {
                channel_id,
                message,
            } => {
                let key = (*channel_id, message.message_id);
                self.requested.remove(&key);
            }
            AppEvent::ThreadPreviewLoadFailed {
                channel_id,
                message_id,
            } => {
                let key = (*channel_id, *message_id);
                self.requested.remove(&key);
                self.failed.insert(key);
            }
            _ => {}
        }
    }

    pub(super) fn next(
        &mut self,
        missing: Vec<(Id<ChannelMarker>, Id<MessageMarker>)>,
    ) -> Vec<(Id<ChannelMarker>, Id<MessageMarker>)> {
        let visible = missing.iter().copied().collect::<HashSet<_>>();
        self.failed.retain(|key| visible.contains(key));

        missing
            .into_iter()
            .filter(|key| !self.failed.contains(key))
            .filter(|key| self.requested.insert(*key))
            .collect()
    }

    pub(super) fn remove(&mut self, key: (Id<ChannelMarker>, Id<MessageMarker>)) {
        self.requested.remove(&key);
    }
}

impl MentionMemberSearchRequests {
    const MIN_QUERY_CHARS: usize = 2;
    const MAX_QUERY_CHARS: usize = 64;
    const DEBOUNCE: Duration = Duration::from_millis(250);
    const REQUEST_TTL: Duration = Duration::from_secs(30);
    const MAX_REQUESTED: usize = 128;

    pub(super) fn set_target(&mut self, target: Option<MentionMemberSearchTarget>, now: Instant) {
        self.prune_requested(now);
        let Some(target) = target.and_then(normalize_mention_member_search_target) else {
            self.pending = None;
            return;
        };
        if self.requested.contains_key(&target.key()) {
            self.pending = None;
            return;
        }
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.target.key() == target.key())
        {
            return;
        }
        self.pending = Some(PendingMentionMemberSearch {
            target,
            ready_at: now + Self::DEBOUNCE,
        });
    }

    pub(super) fn pending_deadline(&self) -> Option<Instant> {
        self.pending.as_ref().map(|pending| pending.ready_at)
    }

    pub(super) fn next_due(&mut self, now: Instant) -> Option<MentionMemberSearchTarget> {
        self.prune_requested(now);
        let pending = self.pending.as_ref()?;
        if pending.ready_at > now {
            return None;
        }
        let pending = self.pending.take()?;
        let key = pending.target.key();
        if self.requested.contains_key(&key) {
            return None;
        }
        self.insert_requested(key, now);
        Some(pending.target)
    }

    fn insert_requested(&mut self, key: MentionMemberSearchKey, now: Instant) {
        self.requested_order.retain(|existing| existing != &key);
        self.requested.insert(key.clone(), now);
        self.requested_order.push_back(key);
        self.prune_requested(now);
    }

    fn prune_requested(&mut self, now: Instant) {
        self.requested.retain(|_, requested_at| {
            now.checked_duration_since(*requested_at)
                .is_none_or(|age| age <= Self::REQUEST_TTL)
        });
        self.requested_order
            .retain(|key| self.requested.contains_key(key));
        while self.requested.len() > Self::MAX_REQUESTED {
            let Some(oldest) = self.requested_order.pop_front() else {
                break;
            };
            self.requested.remove(&oldest);
        }
    }
}

type MentionMemberSearchKey = (Id<GuildMarker>, String);
type MessageAuthorMemberRequestKey = (Id<GuildMarker>, Id<UserMarker>);

#[derive(PartialEq)]
struct MemberListSubscriptionKey {
    guild_id: Id<GuildMarker>,
    channel_id: Id<ChannelMarker>,
    bucket: u32,
}

struct PendingMentionMemberSearch {
    target: MentionMemberSearchTarget,
    ready_at: Instant,
}

struct PendingMemberListSubscription {
    target: MemberListSubscriptionTarget,
    ready_at: Instant,
}

impl MentionMemberSearchTarget {
    fn key(&self) -> MentionMemberSearchKey {
        (self.guild_id, self.query.clone())
    }
}

impl MemberListSubscriptionTarget {
    fn key(&self) -> MemberListSubscriptionKey {
        MemberListSubscriptionKey {
            guild_id: self.guild_id,
            channel_id: self.channel_id,
            bucket: self.bucket,
        }
    }
}

fn normalize_mention_member_search_target(
    target: MentionMemberSearchTarget,
) -> Option<MentionMemberSearchTarget> {
    let query = normalize_mention_member_search_query(&target.query);
    (query.chars().count() >= MentionMemberSearchRequests::MIN_QUERY_CHARS).then_some(
        MentionMemberSearchTarget {
            guild_id: target.guild_id,
            query,
        },
    )
}

fn normalize_mention_member_search_query(query: &str) -> String {
    let mut normalized = String::new();
    let mut count = 0usize;
    for ch in query.trim().chars() {
        for lowered in ch.to_lowercase() {
            if count >= MentionMemberSearchRequests::MAX_QUERY_CHARS {
                return normalized;
            }
            normalized.push(lowered);
            count += 1;
        }
    }
    normalized
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HistoryRequestState {
    Requested,
    Loaded,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ForumPostRequestCursor {
    archive_state: ForumPostArchiveState,
    offset: usize,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ForumPostRequestState {
    active: ForumPostPageRequestState,
    archived: ForumPostPageRequestState,
}

impl ForumPostRequestState {
    fn next(
        &mut self,
        channel_changed: bool,
        should_load_more: bool,
    ) -> Option<ForumPostRequestCursor> {
        if let Some(offset) = self.active.next(channel_changed, true, should_load_more) {
            return Some(ForumPostRequestCursor {
                archive_state: ForumPostArchiveState::Active,
                offset,
            });
        }
        if let Some(offset) =
            self.archived
                .next(channel_changed, should_load_more, should_load_more)
        {
            return Some(ForumPostRequestCursor {
                archive_state: ForumPostArchiveState::Archived,
                offset,
            });
        }
        None
    }

    fn set_loaded(
        &mut self,
        archive_state: ForumPostArchiveState,
        next_offset: usize,
        has_more: bool,
    ) {
        self.page_mut(archive_state)
            .set_loaded(next_offset, has_more);
    }

    fn set_failed(&mut self, archive_state: ForumPostArchiveState, offset: usize) {
        self.page_mut(archive_state).set_failed(offset);
    }

    fn page_mut(&mut self, archive_state: ForumPostArchiveState) -> &mut ForumPostPageRequestState {
        match archive_state {
            ForumPostArchiveState::Active => &mut self.active,
            ForumPostArchiveState::Archived => &mut self.archived,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ForumPostPageRequestState {
    #[default]
    NotRequested,
    Requested {
        offset: usize,
    },
    Loaded {
        next_offset: usize,
        has_more: bool,
    },
    Failed {
        offset: usize,
    },
}

impl ForumPostPageRequestState {
    fn next(
        &mut self,
        channel_changed: bool,
        allow_initial: bool,
        should_load_more: bool,
    ) -> Option<usize> {
        match *self {
            Self::NotRequested if allow_initial => {
                *self = Self::Requested { offset: 0 };
                Some(0)
            }
            Self::Failed { offset } if channel_changed => {
                *self = Self::Requested { offset };
                Some(offset)
            }
            Self::Loaded {
                next_offset,
                has_more: true,
            } if should_load_more => {
                *self = Self::Requested {
                    offset: next_offset,
                };
                Some(next_offset)
            }
            Self::NotRequested
            | Self::Requested { .. }
            | Self::Loaded { .. }
            | Self::Failed { .. } => None,
        }
    }

    fn set_loaded(&mut self, next_offset: usize, has_more: bool) {
        *self = Self::Loaded {
            next_offset,
            has_more,
        };
    }

    fn set_failed(&mut self, offset: usize) {
        *self = Self::Failed { offset };
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PinnedMessageRequestState {
    Requested,
    Loaded,
    Failed,
}

#[cfg(test)]
mod tests {
    use crate::discord::ids::Id;

    use crate::discord::{AppEvent, ChannelInfo, ForumPostArchiveState, MemberInfo};

    use super::{
        ForumPostRequestTarget, ForumPostRequests, HistoryRequests, MemberListSubscriptionRequests,
        MemberListSubscriptionTarget, MemberRequests, MentionMemberSearchRequests,
        MentionMemberSearchTarget, MessageAuthorMemberRequests, ThreadPreviewRequests,
    };

    #[test]
    fn history_request_is_sent_once_and_retries_failed_channel_after_reselect() {
        let mut requests = HistoryRequests::default();
        let first = Id::new(1);
        let second = Id::new(2);

        assert_eq!(requests.next(None, false), None);
        assert_eq!(requests.next(Some(first), false), Some(first));
        assert_eq!(requests.next(Some(first), false), None);
        requests.record_event(&AppEvent::MessageHistoryLoadFailed {
            channel_id: first,
            message: "temporary failure".to_owned(),
        });
        assert_eq!(requests.next(Some(first), false), None);
        assert_eq!(requests.next(Some(second), false), Some(second));
        assert_eq!(requests.next(Some(first), false), Some(first));

        let mut requests = HistoryRequests::default();
        let first = Id::new(1);
        let second = Id::new(2);

        assert_eq!(requests.next(Some(first), false), Some(first));
        requests.record_event(&AppEvent::MessageHistoryLoaded {
            channel_id: first,
            before: None,
            messages: Vec::new(),
        });
        assert_eq!(requests.next(Some(first), true), None);
        assert_eq!(requests.next(Some(second), false), Some(second));
        assert_eq!(requests.next(Some(first), true), Some(first));
    }

    #[test]
    fn forum_post_request_is_sent_once_per_channel() {
        let mut requests = ForumPostRequests::default();
        let guild = Id::new(100);
        let first = Id::new(1);
        let second = Id::new(2);

        assert_eq!(requests.next(None), None);
        assert_eq!(
            requests.next(Some(target(guild, first, false))),
            Some((guild, first, ForumPostArchiveState::Active, 0))
        );
        assert_eq!(requests.next(Some(target(guild, first, false))), None);
        assert_eq!(
            requests.next(Some(target(guild, second, false))),
            Some((guild, second, ForumPostArchiveState::Active, 0))
        );
    }

    #[test]
    fn forum_post_request_retries_failed_channel_after_reselect() {
        let mut requests = ForumPostRequests::default();
        let guild = Id::new(100);
        let first = Id::new(1);
        let second = Id::new(2);

        assert_eq!(
            requests.next(Some(target(guild, first, false))),
            Some((guild, first, ForumPostArchiveState::Active, 0))
        );
        requests.record_event(&AppEvent::ForumPostsLoadFailed {
            channel_id: first,
            archive_state: ForumPostArchiveState::Active,
            offset: 0,
            message: "temporary failure".to_owned(),
        });
        assert_eq!(requests.next(Some(target(guild, first, false))), None);
        assert_eq!(
            requests.next(Some(target(guild, second, false))),
            Some((guild, second, ForumPostArchiveState::Active, 0))
        );
        assert_eq!(
            requests.next(Some(target(guild, first, false))),
            Some((guild, first, ForumPostArchiveState::Active, 0))
        );
    }

    #[test]
    fn forum_post_request_tracks_active_archived_and_server_offsets() {
        let mut requests = ForumPostRequests::default();
        let guild = Id::new(100);
        let channel = Id::new(1);

        assert_eq!(
            requests.next(Some(target(guild, channel, false))),
            Some((guild, channel, ForumPostArchiveState::Active, 0))
        );
        requests.record_event(&AppEvent::ForumPostsLoaded {
            channel_id: channel,
            archive_state: ForumPostArchiveState::Active,
            offset: 0,
            next_offset: 2,
            posts: vec![forum_post(channel, 10), forum_post(channel, 11)],
            preview_messages: Vec::new(),
            has_more: true,
        });

        assert_eq!(requests.next(Some(target(guild, channel, false))), None);
        assert_eq!(
            requests.next(Some(target(guild, channel, true))),
            Some((guild, channel, ForumPostArchiveState::Active, 2))
        );
        requests.record_event(&AppEvent::ForumPostsLoaded {
            channel_id: channel,
            archive_state: ForumPostArchiveState::Active,
            offset: 2,
            next_offset: 3,
            posts: vec![forum_post(channel, 12)],
            preview_messages: Vec::new(),
            has_more: false,
        });

        assert_eq!(requests.next(Some(target(guild, channel, false))), None);
        assert_eq!(
            requests.next(Some(target(guild, channel, true))),
            Some((guild, channel, ForumPostArchiveState::Archived, 0))
        );
        requests.record_event(&AppEvent::ForumPostsLoaded {
            channel_id: channel,
            archive_state: ForumPostArchiveState::Archived,
            offset: 0,
            next_offset: 2,
            posts: vec![forum_post(channel, 11), forum_post(channel, 12)],
            preview_messages: Vec::new(),
            has_more: true,
        });

        assert_eq!(
            requests.next(Some(target(guild, channel, true))),
            Some((guild, channel, ForumPostArchiveState::Archived, 2))
        );

        let mut requests = ForumPostRequests::default();
        let channel = Id::new(2);

        assert_eq!(
            requests.next(Some(target(guild, channel, false))),
            Some((guild, channel, ForumPostArchiveState::Active, 0))
        );
        requests.record_event(&AppEvent::ForumPostsLoaded {
            channel_id: channel,
            archive_state: ForumPostArchiveState::Active,
            offset: 0,
            next_offset: 25,
            posts: vec![forum_post(channel, 10), forum_post(channel, 11)],
            preview_messages: Vec::new(),
            has_more: true,
        });

        assert_eq!(
            requests.next(Some(target(guild, channel, true))),
            Some((guild, channel, ForumPostArchiveState::Active, 25))
        );
    }

    fn target(
        guild_id: Id<crate::discord::ids::marker::GuildMarker>,
        channel_id: Id<crate::discord::ids::marker::ChannelMarker>,
        should_load_more: bool,
    ) -> ForumPostRequestTarget {
        ForumPostRequestTarget {
            guild_id,
            channel_id,
            should_load_more,
        }
    }

    fn forum_post(
        forum_id: Id<crate::discord::ids::marker::ChannelMarker>,
        channel_id: u64,
    ) -> ChannelInfo {
        ChannelInfo {
            guild_id: Some(Id::new(100)),
            channel_id: Id::new(channel_id),
            parent_id: Some(forum_id),
            position: None,
            last_message_id: None,
            name: format!("post {channel_id}"),
            kind: "GuildPublicThread".to_owned(),
            message_count: None,
            total_message_sent: None,
            thread_archived: Some(false),
            thread_locked: Some(false),
            thread_pinned: None,
            recipients: None,
            permission_overwrites: Vec::new(),
        }
    }

    fn subscription_target(bucket: u32) -> MemberListSubscriptionTarget {
        let ranges = if bucket == 0 {
            vec![(0, 99)]
        } else {
            vec![(0, 99), (bucket * 100, bucket * 100 + 99)]
        };
        MemberListSubscriptionTarget {
            guild_id: Id::new(1),
            channel_id: Id::new(2),
            bucket,
            ranges,
        }
    }

    #[test]
    fn member_request_is_sent_once_per_active_guild() {
        let mut requests = MemberRequests::default();
        let first = Id::new(1);
        let second = Id::new(2);

        assert_eq!(requests.next(None), None);
        assert_eq!(requests.next(Some(first)), Some(first));
        assert_eq!(requests.next(Some(first)), None);
        assert_eq!(requests.next(Some(second)), Some(second));
        assert_eq!(requests.next(Some(first)), None);
    }

    #[test]
    fn member_request_can_retry_after_remove() {
        let mut requests = MemberRequests::default();
        let guild_id = Id::new(1);

        assert_eq!(requests.next(Some(guild_id)), Some(guild_id));
        requests.remove(guild_id);

        assert_eq!(requests.next(Some(guild_id)), Some(guild_id));
    }

    #[test]
    fn message_author_member_request_dedupes_until_member_arrives_or_ttl_expires() {
        let mut requests = MessageAuthorMemberRequests::default();
        let guild_id = Id::new(1);
        let user_id = Id::new(10);
        let other_user_id = Id::new(20);
        let now = std::time::Instant::now();

        assert_eq!(
            requests.next(vec![(guild_id, vec![user_id, other_user_id])], now),
            vec![(guild_id, vec![user_id, other_user_id])]
        );
        assert_eq!(
            requests.next(vec![(guild_id, vec![user_id, other_user_id])], now),
            Vec::new()
        );

        requests.record_event(&AppEvent::GuildMemberUpsert {
            guild_id,
            member: MemberInfo {
                user_id,
                display_name: "neo".to_owned(),
                username: Some("neo".to_owned()),
                is_bot: false,
                avatar_url: None,
                role_ids: Vec::new(),
            },
        });
        assert_eq!(
            requests.next(vec![(guild_id, vec![user_id, other_user_id])], now),
            vec![(guild_id, vec![user_id])]
        );

        let retry_at =
            now + MessageAuthorMemberRequests::REQUEST_TTL + std::time::Duration::from_millis(1);
        assert_eq!(
            requests.next(vec![(guild_id, vec![other_user_id])], retry_at),
            vec![(guild_id, vec![other_user_id])]
        );
    }

    #[test]
    fn member_list_subscription_debounces_and_coalesces_bucket_updates() {
        let mut requests = MemberListSubscriptionRequests::default();
        let now = std::time::Instant::now();

        requests.set_target(Some(subscription_target(0)), now);
        assert_eq!(requests.pending_deadline(), None);

        requests.set_target(Some(subscription_target(1)), now);
        let first_deadline = requests
            .pending_deadline()
            .expect("bucket one should arm debounce");
        assert!(
            requests
                .next_due(first_deadline - std::time::Duration::from_millis(1))
                .is_none()
        );

        requests.set_target(
            Some(subscription_target(2)),
            now + std::time::Duration::from_millis(1),
        );
        let second_deadline = requests
            .pending_deadline()
            .expect("latest bucket should stay pending");
        let target = requests
            .next_due(second_deadline)
            .expect("latest bucket should be sent after debounce");
        assert_eq!(target.bucket, 2);
        assert_eq!(target.ranges, vec![(0, 99), (200, 299)]);

        requests.set_target(Some(subscription_target(2)), second_deadline);
        assert_eq!(requests.pending_deadline(), None);

        requests.set_target(Some(subscription_target(0)), second_deadline);
        assert!(requests.pending_deadline().is_some());
    }

    #[test]
    fn mention_member_search_debounces_bounds_and_retries_queries() {
        let mut requests = MentionMemberSearchRequests::default();
        let guild_id = Id::new(1);
        let now = std::time::Instant::now();

        requests.set_target(
            Some(MentionMemberSearchTarget {
                guild_id,
                query: "A".to_owned(),
            }),
            now,
        );
        assert_eq!(requests.pending_deadline(), None);

        requests.set_target(
            Some(MentionMemberSearchTarget {
                guild_id,
                query: " Alice ".to_owned(),
            }),
            now,
        );
        let deadline = requests
            .pending_deadline()
            .expect("valid query should arm debounce");
        assert_eq!(
            requests.next_due(deadline - std::time::Duration::from_millis(1)),
            None
        );
        assert_eq!(
            requests.next_due(deadline),
            Some(MentionMemberSearchTarget {
                guild_id,
                query: "alice".to_owned(),
            })
        );

        requests.set_target(
            Some(MentionMemberSearchTarget {
                guild_id,
                query: "ALICE".to_owned(),
            }),
            now + std::time::Duration::from_secs(1),
        );
        assert_eq!(requests.pending_deadline(), None);

        let retry_at = deadline
            + MentionMemberSearchRequests::REQUEST_TTL
            + std::time::Duration::from_millis(1);
        requests.set_target(
            Some(MentionMemberSearchTarget {
                guild_id,
                query: "alice".to_owned(),
            }),
            retry_at,
        );
        assert!(requests.pending_deadline().is_some());

        let long_query = "A".repeat(MentionMemberSearchRequests::MAX_QUERY_CHARS + 10);
        requests.set_target(
            Some(MentionMemberSearchTarget {
                guild_id,
                query: long_query,
            }),
            retry_at + std::time::Duration::from_millis(1),
        );
        let deadline = requests
            .pending_deadline()
            .expect("long query should still search by capped prefix");
        let target = requests
            .next_due(deadline)
            .expect("capped query should be due");
        assert_eq!(
            target.query.chars().count(),
            MentionMemberSearchRequests::MAX_QUERY_CHARS
        );
        assert!(target.query.chars().all(|ch| ch == 'a'));

        let expanding_query = "İ".repeat(MentionMemberSearchRequests::MAX_QUERY_CHARS + 10);
        requests.set_target(
            Some(MentionMemberSearchTarget {
                guild_id,
                query: expanding_query,
            }),
            retry_at + std::time::Duration::from_millis(2),
        );
        let deadline = requests
            .pending_deadline()
            .expect("expanding query should still search by capped prefix");
        let target = requests
            .next_due(deadline)
            .expect("expanded lowercase query should be due");
        assert_eq!(
            target.query.chars().count(),
            MentionMemberSearchRequests::MAX_QUERY_CHARS
        );
    }

    #[test]
    fn thread_preview_request_retries_after_failed_card_is_revisited() {
        let mut requests = ThreadPreviewRequests::default();
        let key = (Id::new(10), Id::new(30));

        assert_eq!(requests.next(vec![key]), vec![key]);
        requests.record_event(&AppEvent::ThreadPreviewLoadFailed {
            channel_id: key.0,
            message_id: key.1,
        });

        assert_eq!(requests.next(vec![key]), Vec::new());
        assert_eq!(requests.next(Vec::new()), Vec::new());
        assert_eq!(requests.next(vec![key]), vec![key]);
    }
}

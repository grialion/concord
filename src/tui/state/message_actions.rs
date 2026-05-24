use crate::discord::{EmbedInfo, MessageState, ReactionEmoji};
use crate::tui::format::detected_urls;

use super::scroll::{clamp_selected_index, move_index_down, move_index_up};
use super::{
    DashboardState, FocusPane, MessageActionItem, MessageActionKind, MessageActionMenuPhase,
    MessageActionMenuState, MessageUrlItem, popups,
};
use crate::discord::AppCommand;

impl DashboardState {
    pub fn activate_selected_message_pane_item(&mut self) -> Option<AppCommand> {
        if self.selected_channel_is_forum() {
            return self.activate_selected_forum_post();
        }
        self.open_selected_message_actions();
        None
    }

    pub fn is_message_action_menu_open(&self) -> bool {
        self.popups.message_action_menu.is_some()
    }

    pub fn open_selected_message_actions(&mut self) {
        if self.navigation.focus == FocusPane::Messages && self.selected_message_state().is_some() {
            self.popups.message_action_menu = Some(MessageActionMenuState::default());
        }
    }

    pub fn close_message_action_menu(&mut self) {
        self.popups.message_action_menu = None;
    }

    pub fn close_or_back_message_action_menu(&mut self) {
        if let Some(menu) = &mut self.popups.message_action_menu
            && menu.phase == MessageActionMenuPhase::Urls
        {
            menu.phase = MessageActionMenuPhase::Actions;
            menu.selected = 0;
            return;
        }

        self.close_message_action_menu();
    }

    pub fn move_message_action_down(&mut self) {
        let actions_len = self.current_message_action_menu_len();
        if let Some(menu) = &mut self.popups.message_action_menu {
            move_index_down(&mut menu.selected, actions_len);
        }
    }

    pub fn move_message_action_up(&mut self) {
        if let Some(menu) = &mut self.popups.message_action_menu {
            move_index_up(&mut menu.selected);
        }
    }

    pub fn select_message_action_row(&mut self, row: usize) -> bool {
        if row >= self.current_message_action_menu_len() {
            return false;
        }
        if let Some(menu) = &mut self.popups.message_action_menu {
            menu.selected = row;
            return true;
        }
        false
    }

    pub fn selected_message_action_items(&self) -> Vec<MessageActionItem> {
        let Some(message) = self.selected_message_state() else {
            return Vec::new();
        };
        let mut actions = Vec::new();

        actions.push(MessageActionItem {
            kind: MessageActionKind::OpenThread,
            label: "Open thread".to_owned(),
            enabled: self.thread_summary_for_message(message).is_some(),
        });

        let mut has_download_action = false;
        if message.message_kind.is_regular_or_reply() {
            // Image attachments already have a direct `d` download path in the
            // image viewer, so the message-level menu only surfaces downloads
            // for the file/video kinds that have no other entry point.
            for (index, attachment) in message.attachments_in_display_order().enumerate() {
                if attachment.is_image() && attachment.preferred_url().is_some() {
                    continue;
                }
                if attachment.preferred_url().is_none() {
                    continue;
                }
                has_download_action = true;
                actions.push(MessageActionItem {
                    kind: MessageActionKind::DownloadAttachment(index),
                    label: format!("Download {}", attachment.filename),
                    enabled: true,
                });
            }
        }
        if !has_download_action {
            actions.push(MessageActionItem {
                kind: MessageActionKind::DownloadAttachment(0),
                label: "Download attachment".to_owned(),
                enabled: false,
            });
        }

        actions.push(MessageActionItem {
            kind: MessageActionKind::ShowReactionUsers,
            label: "Show reacted users".to_owned(),
            enabled: !message.reactions.is_empty()
                && self.can_show_reaction_users_for_message(message),
        });

        let poll_voting_enabled = message
            .poll
            .as_ref()
            .is_some_and(|poll| !poll.results_finalized.unwrap_or(false));
        actions.push(MessageActionItem {
            kind: MessageActionKind::OpenPollVotePicker,
            label: "Choose poll votes".to_owned(),
            enabled: poll_voting_enabled,
        });
        actions
    }

    pub fn selected_message_action_index(&self) -> Option<usize> {
        self.popups
            .message_action_menu
            .as_ref()
            .filter(|menu| menu.phase == MessageActionMenuPhase::Actions)
            .map(|menu| {
                clamp_selected_index(menu.selected, self.selected_message_action_items().len())
            })
    }

    pub fn is_message_url_picker_open(&self) -> bool {
        self.popups
            .message_action_menu
            .as_ref()
            .is_some_and(|menu| menu.phase == MessageActionMenuPhase::Urls)
    }

    pub fn selected_message_url_items(&self) -> Vec<MessageUrlItem> {
        self.selected_message_state()
            .map(message_url_items)
            .unwrap_or_default()
    }

    pub fn selected_message_url_index(&self) -> Option<usize> {
        self.popups
            .message_action_menu
            .as_ref()
            .filter(|menu| menu.phase == MessageActionMenuPhase::Urls)
            .map(|menu| {
                clamp_selected_index(menu.selected, self.selected_message_url_items().len())
            })
    }

    pub fn selected_message_action(&self) -> Option<MessageActionItem> {
        let index = self.selected_message_action_index()?;
        self.selected_message_action_items().get(index).cloned()
    }

    pub fn activate_selected_message_action(&mut self) -> Option<AppCommand> {
        let action = self.selected_message_action()?;
        if !action.enabled {
            return None;
        }

        match action.kind {
            MessageActionKind::OpenThread => {
                let channel_id = self
                    .selected_message_state()
                    .and_then(|message| self.thread_summary_for_message(message))?
                    .channel_id;
                self.record_thread_return_target(channel_id);
                self.activate_channel(channel_id);
                self.close_message_action_menu();
                None
            }
            MessageActionKind::DownloadAttachment(index) => {
                let message = self.selected_message_state()?;
                let attachment = message.attachments_in_display_order().nth(index)?;
                let url = attachment.preferred_url()?.to_owned();
                let filename = attachment.filename.clone();
                self.close_message_action_menu();
                Some(AppCommand::DownloadAttachment {
                    url,
                    filename,
                    source: crate::discord::DownloadAttachmentSource::MessageAction,
                })
            }
            MessageActionKind::ShowReactionUsers => {
                let message = self.selected_message_state()?;
                if !self.can_show_reaction_users_for_message(message) {
                    self.close_message_action_menu();
                    return None;
                }
                let channel_id = message.channel_id;
                let message_id = message.id;
                let reactions = message
                    .reactions
                    .iter()
                    .map(|reaction| reaction.emoji.clone())
                    .collect::<Vec<_>>();
                if reactions.is_empty() {
                    self.close_message_action_menu();
                    return None;
                }
                self.close_message_action_menu();
                Some(AppCommand::LoadReactionUsers {
                    channel_id,
                    message_id,
                    reactions,
                })
            }
            MessageActionKind::OpenPollVotePicker => {
                self.open_poll_vote_picker();
                self.close_message_action_menu();
                None
            }
        }
    }

    pub(super) fn can_add_reaction_to_message(
        &self,
        message: &MessageState,
        emoji: &ReactionEmoji,
    ) -> bool {
        let Some(channel) = self.discord.cache.channel(message.channel_id) else {
            return true;
        };
        if !self
            .discord
            .cache
            .can_read_message_history_in_channel(channel)
        {
            return false;
        }
        message
            .reactions
            .iter()
            .any(|reaction| &reaction.emoji == emoji)
            || self.discord.cache.can_add_reactions_in_channel(channel)
    }

    pub(super) fn can_open_reaction_picker(&self, message: &MessageState) -> bool {
        let Some(channel) = self.discord.cache.channel(message.channel_id) else {
            return true;
        };
        self.discord
            .cache
            .can_read_message_history_in_channel(channel)
            && (self.discord.cache.can_add_reactions_in_channel(channel)
                || !message.reactions.is_empty())
    }

    pub(super) fn can_add_new_reaction_for_message(&self, message: &MessageState) -> bool {
        let Some(channel) = self.discord.cache.channel(message.channel_id) else {
            return true;
        };
        self.discord.cache.can_add_reactions_in_channel(channel)
    }

    fn can_show_reaction_users_for_message(&self, message: &MessageState) -> bool {
        let Some(channel) = self.discord.cache.channel(message.channel_id) else {
            return true;
        };
        self.discord
            .cache
            .can_read_message_history_in_channel(channel)
    }

    fn can_delete_message(&self, message: &MessageState) -> bool {
        if Some(message.author_id) == self.discord.current_user_id {
            return true;
        }
        let Some(channel) = self.discord.cache.channel(message.channel_id) else {
            return true;
        };
        self.discord.cache.can_manage_messages_in_channel(channel)
    }

    fn can_pin_messages_for_message(&self, message: &MessageState) -> bool {
        let Some(channel) = self.discord.cache.channel(message.channel_id) else {
            return true;
        };
        self.discord.cache.can_pin_messages_in_channel(channel)
    }

    pub fn activate_message_action_shortcut(&mut self, shortcut: char) -> Option<AppCommand> {
        if self.is_message_url_picker_open() {
            return self.activate_message_url_shortcut(shortcut);
        }

        let actions = self.selected_message_action_items();
        let index = actions.iter().enumerate().position(|(index, action)| {
            action.enabled
                && self
                    .options
                    .key_bindings()
                    .message_action_shortcuts(&actions, index)
                    .contains(&shortcut)
        })?;
        self.select_message_action_row(index);
        self.activate_selected_message_action()
    }

    pub fn activate_selected_message_url_action(&mut self) -> Option<AppCommand> {
        let urls = self.selected_message_url_items();
        match urls.as_slice() {
            [] => None,
            [item] => {
                let url = item.url.clone();
                self.close_message_action_menu();
                Some(AppCommand::OpenUrl { url })
            }
            _ => {
                if let Some(menu) = &mut self.popups.message_action_menu {
                    menu.phase = MessageActionMenuPhase::Urls;
                    menu.selected = 0;
                }
                None
            }
        }
    }

    pub fn activate_selected_message_url(&mut self) -> Option<AppCommand> {
        let index = self.selected_message_url_index()?;
        let url = self.selected_message_url_items().get(index)?.url.clone();
        self.close_message_action_menu();
        Some(AppCommand::OpenUrl { url })
    }

    fn activate_message_url_shortcut(&mut self, shortcut: char) -> Option<AppCommand> {
        let urls = self.selected_message_url_items();
        let index = urls.iter().enumerate().position(|(index, _)| {
            self.options
                .key_bindings()
                .indexed_shortcut(index)
                .is_some_and(|candidate| candidate == shortcut)
        })?;
        self.select_message_action_row(index);
        self.activate_selected_message_url()
    }

    fn current_message_action_menu_len(&self) -> usize {
        if self.is_message_url_picker_open() {
            self.selected_message_url_items().len()
        } else {
            self.selected_message_action_items().len()
        }
    }

    pub fn direct_copy_selected_message_content(&mut self) {
        let Some(content) = self
            .selected_message_state()
            .and_then(|message| message.content.as_ref())
        else {
            return;
        };
        self.runtime.copy_message_content_requested = Some(content.clone());
    }

    pub(in crate::tui) fn take_copy_message_content_request(&mut self) -> Option<String> {
        self.runtime.copy_message_content_requested.take()
    }

    pub fn direct_open_selected_message_reaction_picker(&mut self) {
        self.open_emoji_reaction_picker();
    }

    pub fn direct_reply_to_selected_message(&mut self) {
        self.start_reply_composer();
    }

    pub fn direct_edit_selected_message(&mut self) {
        self.start_edit_composer();
    }

    pub fn direct_open_selected_message_image_viewer(&mut self) {
        self.open_image_viewer_for_selected_message();
    }

    pub fn direct_open_selected_message_url(&mut self) -> Option<AppCommand> {
        let message = self.selected_message_state()?;
        let urls = message_url_items(message);

        match urls.as_slice() {
            [] => None,
            [item] => Some(AppCommand::OpenUrl {
                url: item.url.clone(),
            }),
            _ => {
                self.open_selected_message_actions();
                self.activate_selected_message_url_action()
            }
        }
    }

    pub fn direct_show_selected_message_profile(&mut self) -> Option<AppCommand> {
        let message = self.selected_message_state()?;
        self.open_user_profile_popup(message.author_id, message.guild_id)
    }

    pub fn direct_open_selected_message_pin_confirmation(&mut self) {
        let Some(message) = self.selected_message_state() else {
            return;
        };
        self.open_selected_message_pin_confirmation(!message.pinned);
    }

    pub fn open_selected_message_delete_confirmation(&mut self) {
        let Some(message) = self.selected_message_state() else {
            return;
        };
        if !self.can_delete_message(message) {
            return;
        }
        self.popups.message_delete_confirmation = Some(popups::MessageDeleteConfirmationState {
            channel_id: message.channel_id,
            message_id: message.id,
            author: message.author.clone(),
            content: message.content.clone(),
        });
    }

    pub fn is_message_delete_confirmation_open(&self) -> bool {
        self.popups.message_delete_confirmation.is_some()
    }

    pub fn close_message_delete_confirmation(&mut self) {
        self.popups.message_delete_confirmation = None;
    }

    pub fn confirm_message_delete(&mut self) -> Option<AppCommand> {
        let confirmation = self.popups.message_delete_confirmation.take()?;
        Some(AppCommand::DeleteMessage {
            channel_id: confirmation.channel_id,
            message_id: confirmation.message_id,
        })
    }

    pub fn message_delete_confirmation_lines(&self) -> Option<(String, Option<String>)> {
        let confirmation = self.popups.message_delete_confirmation.as_ref()?;
        Some((confirmation.author.clone(), confirmation.content.clone()))
    }

    pub fn open_selected_message_pin_confirmation(&mut self, pinned: bool) {
        let Some(message) = self.selected_message_state() else {
            return;
        };
        if !self.can_pin_messages_for_message(message) {
            return;
        }
        self.popups.message_pin_confirmation = Some(popups::MessagePinConfirmationState {
            channel_id: message.channel_id,
            message_id: message.id,
            pinned,
            author: message.author.clone(),
            content: message.content.clone(),
        });
    }

    pub fn is_message_pin_confirmation_open(&self) -> bool {
        self.popups.message_pin_confirmation.is_some()
    }

    pub fn close_message_pin_confirmation(&mut self) {
        self.popups.message_pin_confirmation = None;
    }

    pub fn confirm_message_pin(&mut self) -> Option<AppCommand> {
        let confirmation = self.popups.message_pin_confirmation.take()?;
        Some(AppCommand::SetMessagePinned {
            channel_id: confirmation.channel_id,
            message_id: confirmation.message_id,
            pinned: confirmation.pinned,
        })
    }

    pub fn message_pin_confirmation_lines(&self) -> Option<(bool, String, Option<String>)> {
        let confirmation = self.popups.message_pin_confirmation.as_ref()?;
        Some((
            confirmation.pinned,
            confirmation.author.clone(),
            confirmation.content.clone(),
        ))
    }
}

fn message_url_items(message: &MessageState) -> Vec<MessageUrlItem> {
    message_urls(message)
        .into_iter()
        .map(|url| MessageUrlItem {
            label: url.clone(),
            url,
        })
        .collect()
}

fn message_urls(message: &MessageState) -> Vec<String> {
    let mut urls = Vec::new();
    if let Some(content) = &message.content {
        urls.extend(detected_urls(content));
    }
    urls.extend(embed_urls(&message.embeds));
    // URLs in a reply quote or a forwarded message are shown to the user too.
    if let Some(reply) = &message.reply
        && let Some(content) = &reply.content
    {
        urls.extend(detected_urls(content));
    }
    for snapshot in &message.forwarded_snapshots {
        if let Some(content) = &snapshot.content {
            urls.extend(detected_urls(content));
        }
        urls.extend(embed_urls(&snapshot.embeds));
    }
    dedupe_urls(urls)
}

fn embed_urls(embeds: &[EmbedInfo]) -> Vec<String> {
    embeds
        .iter()
        .filter_map(|embed| embed.url.clone())
        .collect()
}

fn dedupe_urls(urls: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for url in urls {
        if !unique.contains(&url) {
            unique.push(url);
        }
    }
    unique
}

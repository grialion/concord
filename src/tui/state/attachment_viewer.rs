use crate::discord::{
    AppCommand, AttachmentInfo, DownloadAttachmentSource, InlinePreviewInfo,
    ids::{Id, marker::MessageMarker},
};

use super::scroll::clamp_selected_index;
use super::{AttachmentViewerItem, DashboardState};
use crate::tui::state::popups::AttachmentViewerState;

impl DashboardState {
    pub fn is_attachment_viewer_open(&self) -> bool {
        self.popups.attachment_viewer.is_some()
    }

    pub fn open_attachment_viewer_for_selected_message(&mut self) -> bool {
        let Some(message) = self.selected_message_state() else {
            return false;
        };
        if message.attachments_in_display_order().next().is_none() {
            return false;
        }

        self.popups.attachment_viewer = Some(AttachmentViewerState {
            message_id: message.id,
            selected: 0,
            download_message: None,
        });
        true
    }

    pub fn close_attachment_viewer(&mut self) {
        self.popups.attachment_viewer = None;
    }

    pub fn move_attachment_viewer_previous(&mut self) {
        if let Some(viewer) = &mut self.popups.attachment_viewer {
            viewer.selected = viewer.selected.saturating_sub(1);
        }
    }

    pub fn move_attachment_viewer_next(&mut self) {
        let Some((message_id, selected)) = self
            .popups
            .attachment_viewer
            .as_ref()
            .map(|viewer| (viewer.message_id, viewer.selected))
        else {
            return;
        };
        let count = self.attachment_viewer_attachment_count(message_id);
        if count == 0 {
            self.close_attachment_viewer();
            return;
        }
        if let Some(viewer) = &mut self.popups.attachment_viewer {
            viewer.selected = selected.saturating_add(1).min(count.saturating_sub(1));
        }
    }

    pub fn selected_attachment_viewer_item(&self) -> Option<AttachmentViewerItem> {
        let viewer = self.popups.attachment_viewer.as_ref()?;
        let attachments = self.attachment_viewer_attachments(viewer.message_id)?;
        let selected = clamp_selected_index(viewer.selected, attachments.len());
        let attachment = attachments.get(selected)?;
        Some(AttachmentViewerItem {
            index: selected.saturating_add(1),
            total: attachments.len(),
            filename: attachment.filename.clone(),
            url: attachment.preferred_url().map(str::to_owned),
            size_bytes: attachment.size,
            is_image: attachment.is_image(),
        })
    }

    pub(in crate::tui) fn selected_attachment_viewer_preview(
        &self,
    ) -> Option<(Id<MessageMarker>, usize, InlinePreviewInfo<'_>)> {
        let viewer = self.popups.attachment_viewer.as_ref()?;
        let attachments = self.attachment_viewer_attachments(viewer.message_id)?;
        let selected = clamp_selected_index(viewer.selected, attachments.len());
        let attachment = *attachments.get(selected)?;
        let preview = attachment.inline_preview_info()?;
        Some((viewer.message_id, selected, preview))
    }

    pub fn attachment_viewer_download_message(&self) -> Option<&str> {
        self.popups
            .attachment_viewer
            .as_ref()
            .and_then(|viewer| viewer.download_message.as_deref())
    }

    pub fn record_attachment_viewer_download_completed(&mut self, path: &str) {
        if let Some(viewer) = &mut self.popups.attachment_viewer {
            viewer.download_message = Some(format!("Downloaded to {path}"));
        }
    }

    pub fn download_selected_attachment_viewer_attachment(&mut self) -> Option<AppCommand> {
        let item = self.selected_attachment_viewer_item()?;
        let url = item.url?;
        if let Some(viewer) = &mut self.popups.attachment_viewer {
            viewer.download_message = Some("Downloading attachment...".to_owned());
        }
        Some(AppCommand::DownloadAttachment {
            url,
            filename: item.filename,
            source: DownloadAttachmentSource::AttachmentViewer,
        })
    }

    fn attachment_viewer_attachments(
        &self,
        message_id: Id<MessageMarker>,
    ) -> Option<Vec<&AttachmentInfo>> {
        self.messages()
            .into_iter()
            .find(|message| message.id == message_id)
            .map(|message| message.attachments_in_display_order().collect())
    }

    fn attachment_viewer_attachment_count(&self, message_id: Id<MessageMarker>) -> usize {
        match self.attachment_viewer_attachments(message_id) {
            Some(attachments) => attachments.len(),
            None => 0,
        }
    }
}

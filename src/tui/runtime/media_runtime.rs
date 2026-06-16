use ratatui::layout::Rect;
use tokio::sync::mpsc;

use crate::{
    discord::{AppCommand, DiscordClient},
    tui::{
        commands as command_helpers,
        media::{
            AvatarImageCache, AvatarTarget, EmojiImageCache, EmojiImageTarget, ImagePreviewCache,
            ImagePreviewTarget, MediaImageDecodeKey, MediaImageDecodeResult,
            visible_avatar_targets_from_plan, visible_emoji_image_targets,
            visible_image_preview_targets_from_plan,
        },
        message::layout::MessageViewportPlan,
        state::DashboardState,
        ui::{self, ImagePreviewLayout},
    },
};

use super::{effects as effect_helpers, redraw::image_surfaces_visible};

pub(super) struct DashboardMediaRuntime {
    image_previews: ImagePreviewCache,
    avatar_images: AvatarImageCache,
    emoji_images: EmojiImageCache,
    image_targets: Vec<ImagePreviewTarget>,
    avatar_targets: Vec<AvatarTarget>,
    emoji_targets: Vec<EmojiImageTarget>,
}

impl DashboardMediaRuntime {
    pub(super) fn new() -> Self {
        Self {
            image_previews: ImagePreviewCache::new(),
            avatar_images: AvatarImageCache::new(),
            emoji_images: EmojiImageCache::new(),
            image_targets: Vec::new(),
            avatar_targets: Vec::new(),
            emoji_targets: Vec::new(),
        }
    }

    pub(super) fn refresh_protocols(&mut self) {
        self.image_previews.refresh_protocols();
        self.avatar_images.refresh_protocols();
        self.emoji_images.refresh_protocols();
    }

    pub(super) fn image_surfaces_visible(&self, state: &DashboardState) -> bool {
        image_surfaces_visible(
            state,
            !self.image_targets.is_empty(),
            !self.avatar_targets.is_empty(),
            !self.emoji_targets.is_empty(),
        )
    }

    pub(super) fn effect_context<'a>(
        &'a mut self,
        state: &'a mut DashboardState,
        client: &'a DiscordClient,
        media_decode_tx: &'a mpsc::UnboundedSender<MediaImageDecodeResult>,
    ) -> effect_helpers::EffectContext<'a> {
        effect_helpers::EffectContext {
            state,
            client,
            image_previews: &mut self.image_previews,
            avatar_images: &mut self.avatar_images,
            emoji_images: &mut self.emoji_images,
            media_decode_tx,
        }
    }

    pub(super) fn store_media_decode(&mut self, result: MediaImageDecodeResult) {
        let MediaImageDecodeResult {
            key,
            generation,
            result,
        } = result;
        match key {
            MediaImageDecodeKey::Preview(key) => {
                self.image_previews.store_decoded(key, generation, result);
            }
            MediaImageDecodeKey::Avatar(key) => {
                self.avatar_images.store_decoded(key, generation, result);
            }
            MediaImageDecodeKey::Emoji(url) => {
                self.emoji_images.store_decoded(url, generation, result);
            }
        }
    }

    fn preview_layout_for_draw(
        &self,
        state: &mut DashboardState,
        area: Rect,
    ) -> ImagePreviewLayout {
        let mut preview_layout = ui::image_preview_layout(area, state);
        preview_layout.font_size = self.image_previews.font_size();
        if !state.show_images() {
            preview_layout.preview_width = 0;
            preview_layout.max_preview_height = 0;
            preview_layout.viewer_preview_width = 0;
            preview_layout.viewer_max_preview_height = 0;
        }
        state.clamp_message_viewport_for_image_previews(
            preview_layout.content_width,
            preview_layout.preview_width,
            preview_layout.max_preview_height,
        );
        preview_layout
    }

    fn compute_targets_for_draw(
        &mut self,
        state: &DashboardState,
        layout: ImagePreviewLayout,
        plan: &MessageViewportPlan<'_>,
    ) {
        self.image_targets = visible_image_preview_targets_from_plan(state, layout, plan);
        self.avatar_targets = visible_avatar_targets_from_plan(state, layout, plan);
        self.emoji_targets = visible_emoji_image_targets(state);
    }
}

pub(super) fn clear_image_surfaces_frame(
    frame: &mut ratatui::Frame<'_>,
    state: &mut DashboardState,
) -> Rect {
    let area = frame.area();
    ui::sync_view_heights(area, state);
    ui::render(frame, state, Vec::new(), Vec::new(), Vec::new(), None);
    area
}

pub(super) fn draw_dashboard_frame(
    frame: &mut ratatui::Frame<'_>,
    state: &mut DashboardState,
    media_runtime: &mut DashboardMediaRuntime,
) -> Rect {
    let area = frame.area();
    ui::sync_view_heights(area, state);
    let preview_layout = media_runtime.preview_layout_for_draw(state, area);
    let messages = state.visible_messages();
    let selected = state.focused_message_selection();
    let viewport_plan = MessageViewportPlan::new(
        &messages,
        selected,
        state,
        preview_layout.content_width,
        preview_layout.preview_width,
        preview_layout.max_preview_height,
    );
    media_runtime.compute_targets_for_draw(state, preview_layout, &viewport_plan);

    let image_previews = media_runtime
        .image_previews
        .render_state(&media_runtime.image_targets);
    let rendered_emojis = media_runtime
        .emoji_images
        .render_state(&media_runtime.emoji_targets);
    let pending_popup_avatar_key = state.user_profile_popup_pending_avatar_preview_key();
    let popup_avatar_url = state
        .show_avatars()
        .then(|| pending_popup_avatar_key.or_else(|| state.user_profile_popup_avatar_url()))
        .flatten();
    let (rendered_avatars, popup_avatar) = media_runtime.avatar_images.render_state_with_popup(
        &media_runtime.avatar_targets,
        popup_avatar_url,
        state.circular_avatars(),
    );

    ui::render_with_message_viewport_plan(
        frame,
        state,
        image_previews,
        rendered_avatars,
        rendered_emojis,
        popup_avatar,
        Some(&viewport_plan),
    );
    area
}

pub(super) async fn drain_pending_commands_after_draw(
    state: &mut DashboardState,
    commands: &mpsc::Sender<AppCommand>,
) -> bool {
    let pending_commands = state.drain_pending_commands();
    send_commands_until_closed(state, commands, pending_commands).await
}

pub(super) async fn schedule_media_loads_after_draw(
    state: &mut DashboardState,
    media_runtime: &mut DashboardMediaRuntime,
    commands: &mpsc::Sender<AppCommand>,
) -> bool {
    let mut dirty = false;
    send_media_request_commands(
        state,
        commands,
        media_runtime
            .image_previews
            .next_requests(&media_runtime.image_targets),
        &mut dirty,
    )
    .await;
    send_media_request_commands(
        state,
        commands,
        media_runtime
            .avatar_images
            .next_requests(&media_runtime.avatar_targets),
        &mut dirty,
    )
    .await;

    // Profile popup avatar isn't part of the message-pane targets, so schedule
    // its fetch separately. It uses a larger avatar CDN size than message-pane
    // avatars, so it may have its own cache entry.
    if state.show_avatars() {
        let command = if let Some(key) = state.user_profile_popup_pending_avatar_preview_key() {
            media_runtime
                .avatar_images
                .next_request_for_profile_upload(key, || {
                    state.user_profile_popup_pending_avatar_upload()
                })
        } else if let Some(url) = state.user_profile_popup_avatar_url().map(str::to_owned) {
            media_runtime.avatar_images.next_request_for_url(&url)
        } else {
            None
        };
        if let Some(command) = command {
            send_media_request_commands(state, commands, [command], &mut dirty).await;
        }
    }

    send_media_request_commands(
        state,
        commands,
        media_runtime
            .emoji_images
            .next_requests(&media_runtime.emoji_targets),
        &mut dirty,
    )
    .await;
    dirty
}

async fn send_media_request_commands(
    state: &mut DashboardState,
    commands: &mpsc::Sender<AppCommand>,
    media_commands: impl IntoIterator<Item = AppCommand>,
    dirty: &mut bool,
) {
    for command in media_commands {
        *dirty = true;
        if command_helpers::send_or_record_closed(state, commands, command)
            .await
            .is_channel_closed()
        {
            break;
        }
    }
}

async fn send_commands_until_closed(
    state: &mut DashboardState,
    commands: &mpsc::Sender<AppCommand>,
    pending_commands: impl IntoIterator<Item = AppCommand>,
) -> bool {
    for command in pending_commands {
        if command_helpers::send_or_record_closed(state, commands, command)
            .await
            .is_channel_closed()
        {
            return true;
        }
    }
    false
}

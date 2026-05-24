use super::*;
use crate::discord::AppCommand;

#[test]
fn message_action_items_reflect_selected_message_capabilities() {
    let mut state = state_with_image_messages(1, &[1]);
    state.focus_pane(FocusPane::Messages);

    let actions = state.selected_message_action_items();

    assert_eq!(actions.len(), 4);
    assert!(actions.iter().all(|action| !action.enabled));
    assert_eq!(actions[0].label, "Open thread");
    assert_eq!(actions[1].label, "Download attachment");
    assert_eq!(actions[2].label, "Show reacted users");
    assert_eq!(actions[3].label, "Choose poll votes");
}

#[test]
fn disabled_image_previews_hide_view_image_action() {
    let mut state = state_with_image_messages(1, &[1]);
    state.open_options_popup();
    state.toggle_selected_display_option();
    state.focus_pane(FocusPane::Messages);

    let actions = state.selected_message_action_items();

    assert!(!actions.iter().any(|action| action.label == "View image"));
}

#[test]
fn direct_image_message_action_opens_image_viewer() {
    let mut state = state_with_messages(1);
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some("https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_owned()),
            embeds: vec![youtube_embed()],
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);

    state.direct_open_selected_message_image_viewer();

    assert!(state.is_image_viewer_open());
    assert_eq!(
        state.selected_image_viewer_item(),
        Some(super::ImageViewerItem {
            index: 1,
            total: 1,
            filename: "embed-thumbnail".to_owned(),
            url: "https://i.ytimg.com/vi/dQw4w9WgXcQ/hqdefault.jpg".to_owned(),
        })
    );
}

#[test]
fn image_viewer_navigation_clamps_and_downloads_current_image() {
    let mut state = state_with_messages(1);
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some(String::new()),
            attachments: vec![image_attachment(10), image_attachment(11)],
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);
    state.direct_open_selected_message_image_viewer();

    state.move_image_viewer_previous();
    assert_eq!(
        state.selected_image_viewer_item().map(|item| item.index),
        Some(1)
    );

    state.move_image_viewer_next();
    state.move_image_viewer_next();
    assert_eq!(
        state.selected_image_viewer_item().map(|item| item.index),
        Some(2)
    );

    let command = state.download_selected_image_viewer_image();

    assert_eq!(
        command,
        Some(AppCommand::DownloadAttachment {
            url: "https://cdn.discordapp.com/image-11.png".to_owned(),
            filename: "image-11.png".to_owned(),
            source: DownloadAttachmentSource::ImageViewer,
        })
    );
    assert!(state.is_image_viewer_open());
    assert_eq!(
        state.image_viewer_download_message(),
        Some("Downloading image...")
    );
}

#[test]
fn image_viewer_download_uses_original_url_not_preview_proxy() {
    let mut state = state_with_messages(1);
    let mut attachment = image_attachment(10);
    attachment.url = "https://cdn.discordapp.com/original/photo.png".to_owned();
    attachment.proxy_url = concat!(
        "https://media.discordapp.net/attachments/1/10/photo.png",
        "?format=webp&width=160&height=90"
    )
    .to_owned();
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some(String::new()),
            attachments: vec![attachment],
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);
    state.direct_open_selected_message_image_viewer();

    let command = state.download_selected_image_viewer_image();

    assert_eq!(
        command,
        Some(AppCommand::DownloadAttachment {
            url: "https://cdn.discordapp.com/original/photo.png".to_owned(),
            filename: "image-10.png".to_owned(),
            source: DownloadAttachmentSource::ImageViewer,
        })
    );
}

#[test]
fn image_viewer_download_completed_event_updates_viewer_message() {
    let mut state = state_with_messages(1);
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some(String::new()),
            attachments: vec![image_attachment(10)],
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);
    state.direct_open_selected_message_image_viewer();

    state.push_event(AppEvent::AttachmentDownloadCompleted {
        path: "/tmp/cat.png".to_owned(),
        source: DownloadAttachmentSource::ImageViewer,
    });

    assert_eq!(
        state.image_viewer_download_message(),
        Some("Downloaded to /tmp/cat.png")
    );
}

#[test]
fn message_action_download_completed_event_does_not_open_image_feedback() {
    let mut state = DashboardState::new();

    state.push_event(AppEvent::AttachmentDownloadCompleted {
        path: "/tmp/clip.mp4".to_owned(),
        source: DownloadAttachmentSource::MessageAction,
    });

    assert_eq!(state.image_viewer_download_message(), None);
}

#[test]
fn normal_message_actions_show_disabled_dynamic_actions() {
    let mut state = state_with_messages(1);
    state.focus_pane(FocusPane::Messages);

    let actions = state.selected_message_action_items();

    assert_eq!(actions.len(), 4);
    assert!(actions.iter().all(|action| !action.enabled));
}

#[test]
fn own_regular_message_actions_show_disabled_dynamic_actions() {
    let mut state = state_with_messages(1);
    state.push_event(AppEvent::Ready {
        user: "neo".to_owned(),
        user_id: Some(Id::new(99)),
    });
    state.focus_pane(FocusPane::Messages);

    let actions = state.selected_message_action_items();

    assert_eq!(actions.len(), 4);
    assert!(actions.iter().all(|action| !action.enabled));
}

#[test]
fn own_reply_message_actions_show_disabled_dynamic_actions() {
    let mut state = state_with_message_ids([]);
    state.push_event(AppEvent::Ready {
        user: "neo".to_owned(),
        user_id: Some(Id::new(99)),
    });
    push_reply_message_with_attachments(&mut state, 1, 99, Some("reply body"), Vec::new());
    state.focus_pane(FocusPane::Messages);

    let actions = state.selected_message_action_items();

    assert_eq!(actions.len(), 4);
    assert!(actions.iter().all(|action| !action.enabled));
}

#[test]
fn edit_reply_action_prefills_composer_without_reply_target_and_submits_edit_command() {
    let mut state = state_with_message_ids([]);
    state.push_event(AppEvent::Ready {
        user: "neo".to_owned(),
        user_id: Some(Id::new(99)),
    });
    push_reply_message_with_attachments(&mut state, 1, 99, Some("reply body"), Vec::new());
    state.focus_pane(FocusPane::Messages);

    state.direct_edit_selected_message();

    assert_eq!(state.composer_input(), "reply body");
    assert!(state.reply_target_message_state().is_none());
    state.push_composer_char('!');

    assert_eq!(
        state.submit_composer(),
        Some(AppCommand::EditMessage {
            channel_id: Id::new(2),
            message_id: Id::new(1),
            content: "reply body!".to_owned(),
        })
    );
}

#[test]
fn other_user_direct_edit_does_not_start_composer() {
    let mut state = state_with_messages(1);
    state.push_event(AppEvent::Ready {
        user: "me".to_owned(),
        user_id: Some(Id::new(10)),
    });
    state.focus_pane(FocusPane::Messages);

    state.direct_edit_selected_message();

    assert!(!state.is_composing());
}

#[test]
fn unhydrated_guild_permissions_keep_other_user_delete_available() {
    let mut state =
        state_with_other_user_message_permissions_hydrating_member(PERM_VIEW_CHANNEL, Vec::new());
    state.focus_pane(FocusPane::Messages);

    state.open_selected_message_delete_confirmation();

    assert!(state.is_message_delete_confirmation_open());
}

#[test]
fn other_user_message_actions_include_delete_with_manage_messages() {
    let mut state = state_with_other_user_message_permissions(
        PERM_VIEW_CHANNEL | PERM_READ_MESSAGE_HISTORY | PERM_MANAGE_MESSAGES,
        Vec::new(),
    );
    state.focus_pane(FocusPane::Messages);

    state.direct_edit_selected_message();
    assert!(!state.is_composing());

    state.open_selected_message_delete_confirmation();

    assert!(state.is_message_delete_confirmation_open());
    assert_eq!(
        state.confirm_message_delete(),
        Some(AppCommand::DeleteMessage {
            channel_id: Id::new(2),
            message_id: Id::new(1),
        })
    );
}

#[test]
fn other_user_delete_requires_manage_messages() {
    let mut state = state_with_other_user_message_permissions(
        PERM_VIEW_CHANNEL | PERM_READ_MESSAGE_HISTORY,
        Vec::new(),
    );
    state.focus_pane(FocusPane::Messages);

    state.open_selected_message_delete_confirmation();

    assert!(!state.is_message_delete_confirmation_open());
}

#[test]
fn direct_edit_message_prefills_composer_and_submits_edit_command() {
    let mut state = state_with_messages(1);
    state.push_event(AppEvent::Ready {
        user: "neo".to_owned(),
        user_id: Some(Id::new(99)),
    });
    state.focus_pane(FocusPane::Messages);

    state.direct_edit_selected_message();

    assert_eq!(state.composer_input(), "msg 1");
    state.push_composer_char('!');

    assert_eq!(
        state.submit_composer(),
        Some(AppCommand::EditMessage {
            channel_id: Id::new(2),
            message_id: Id::new(1),
            content: "msg 1!".to_owned(),
        })
    );
    assert!(!state.is_composing());
}

#[test]
fn direct_delete_message_submits_delete_command_for_own_message() {
    let mut state = state_with_messages(1);
    state.push_event(AppEvent::Ready {
        user: "neo".to_owned(),
        user_id: Some(Id::new(99)),
    });
    state.focus_pane(FocusPane::Messages);

    state.open_selected_message_delete_confirmation();

    assert!(state.is_message_delete_confirmation_open());
    assert_eq!(
        state.confirm_message_delete(),
        Some(AppCommand::DeleteMessage {
            channel_id: Id::new(2),
            message_id: Id::new(1),
        })
    );
}

#[test]
fn own_attachment_only_message_can_be_deleted_but_not_edited() {
    let mut state = state_with_message_ids([]);
    state.push_event(AppEvent::Ready {
        user: "neo".to_owned(),
        user_id: Some(Id::new(99)),
    });
    state.push_event(message_create_event(MessageCreateFixture {
        guild_id: Some(Id::new(1)),
        channel_id: Id::new(2),
        message_id: Id::new(1),
        author_id: Id::new(99),
        content: None,
        attachments: vec![image_attachment(1)],
        ..MessageCreateFixture::default()
    }));
    state.focus_pane(FocusPane::Messages);

    state.direct_edit_selected_message();
    assert!(!state.is_composing());

    state.open_selected_message_delete_confirmation();

    assert!(state.is_message_delete_confirmation_open());
    assert_eq!(
        state.confirm_message_delete(),
        Some(AppCommand::DeleteMessage {
            channel_id: Id::new(2),
            message_id: Id::new(1),
        })
    );
}

#[test]
fn direct_pin_message_requires_pin_messages_permission() {
    let mut without_pin = state_with_other_user_message_permissions(
        PERM_VIEW_CHANNEL | PERM_READ_MESSAGE_HISTORY,
        Vec::new(),
    );
    without_pin.focus_pane(FocusPane::Messages);

    without_pin.direct_open_selected_message_pin_confirmation();

    assert!(!without_pin.is_message_pin_confirmation_open());

    let mut with_pin = state_with_other_user_message_permissions(
        PERM_VIEW_CHANNEL | PERM_READ_MESSAGE_HISTORY | PERM_PIN_MESSAGES,
        Vec::new(),
    );
    with_pin.focus_pane(FocusPane::Messages);

    with_pin.direct_open_selected_message_pin_confirmation();

    assert!(with_pin.is_message_pin_confirmation_open());
}

#[test]
fn non_image_attachment_action_downloads_with_proxy_url_fallback() {
    let mut state = state_with_message_ids([]);
    let mut attachment = video_attachment(1);
    attachment.url.clear();
    state.push_event(message_create_event(MessageCreateFixture {
        guild_id: Some(Id::new(1)),
        channel_id: Id::new(2),
        message_id: Id::new(1),
        author_id: Id::new(99),
        content: Some("clip".to_owned()),
        attachments: vec![attachment],
        ..MessageCreateFixture::default()
    }));
    state.focus_pane(FocusPane::Messages);
    state.open_selected_message_actions();

    let actions = state.selected_message_action_items();
    assert!(actions.iter().any(|action| {
        action.kind == MessageActionKind::DownloadAttachment(0)
            && action.label == "Download clip-1.mp4"
    }));
    assert!(state.select_message_action_row(1));

    assert_eq!(
        state.activate_selected_message_action(),
        Some(AppCommand::DownloadAttachment {
            url: "https://media.discordapp.net/clip-1.mp4".to_owned(),
            filename: "clip-1.mp4".to_owned(),
            source: DownloadAttachmentSource::MessageAction,
        })
    );
}

#[test]
fn reply_image_attachment_action_can_open_image_viewer() {
    let mut state = state_with_message_ids([]);
    push_reply_message_with_attachments(
        &mut state,
        1,
        99,
        Some("reply image"),
        vec![image_attachment(1)],
    );
    state.focus_pane(FocusPane::Messages);
    let actions = state.selected_message_action_items();
    assert_eq!(actions.len(), 4);
    assert!(actions.iter().all(|action| !action.enabled));

    state.direct_open_selected_message_image_viewer();

    assert!(state.is_image_viewer_open());
    assert_eq!(
        state.selected_image_viewer_item(),
        Some(super::ImageViewerItem {
            index: 1,
            total: 1,
            filename: "image-1.png".to_owned(),
            url: "https://cdn.discordapp.com/image-1.png".to_owned(),
        })
    );
}

#[test]
fn reply_non_image_attachment_action_downloads_with_proxy_url_fallback() {
    let mut state = state_with_message_ids([]);
    let mut attachment = video_attachment(1);
    attachment.url.clear();
    push_reply_message_with_attachments(&mut state, 1, 99, Some("reply clip"), vec![attachment]);
    state.focus_pane(FocusPane::Messages);
    state.open_selected_message_actions();

    let actions = state.selected_message_action_items();
    assert!(actions.iter().any(|action| {
        action.kind == MessageActionKind::DownloadAttachment(0)
            && action.label == "Download clip-1.mp4"
    }));
    assert!(state.select_message_action_row(1));

    assert_eq!(
        state.activate_selected_message_action(),
        Some(AppCommand::DownloadAttachment {
            url: "https://media.discordapp.net/clip-1.mp4".to_owned(),
            filename: "clip-1.mp4".to_owned(),
            source: DownloadAttachmentSource::MessageAction,
        })
    );
}

#[test]
fn direct_message_url_opens_single_url_from_message_content() {
    let mut state = state_with_messages(1);
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some("read https://example.com/docs.".to_owned()),
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);
    assert_eq!(
        state.direct_open_selected_message_url(),
        Some(AppCommand::OpenUrl {
            url: "https://example.com/docs".to_owned(),
        })
    );
    assert!(!state.is_message_action_menu_open());
}

#[test]
fn direct_message_url_opens_url_picker_for_multiple_urls() {
    let mut state = state_with_messages(1);
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some("one https://one.example two <https://two.example/path>,".to_owned()),
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);
    assert_eq!(state.direct_open_selected_message_url(), None);
    assert!(state.is_message_url_picker_open());
    assert_eq!(state.selected_message_url_index(), Some(0));

    assert_eq!(
        state.activate_message_action_shortcut('2'),
        Some(AppCommand::OpenUrl {
            url: "https://two.example/path".to_owned(),
        })
    );
    assert!(!state.is_message_action_menu_open());
}

#[test]
fn message_action_detects_markdown_link_urls() {
    let mut state = state_with_messages(1);
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some(
                "[Tweet](<https://x.com/i/status/2055068765671305537>) • [@steelers](<https://x.com/steelers>) • [FxTwitter](https://fxtwitter.com/i/status/2055068765671305537)"
                    .to_owned(),
            ),
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);
    state.open_selected_message_actions();

    let urls = state.selected_message_url_items();

    assert_eq!(
        urls.into_iter().map(|item| item.url).collect::<Vec<_>>(),
        vec![
            "https://x.com/i/status/2055068765671305537",
            "https://x.com/steelers",
            "https://fxtwitter.com/i/status/2055068765671305537",
        ]
    );
}

#[test]
fn message_action_detects_embed_urls() {
    let mut state = state_with_messages(1);
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some("embed below".to_owned()),
            embeds: vec![EmbedInfo {
                color: None,
                provider_name: None,
                author_name: None,
                title: Some("Release notes".to_owned()),
                description: Some("Read [docs](<https://docs.example/release>)".to_owned()),
                timestamp: None,
                fields: vec![EmbedFieldInfo {
                    name: "Links".to_owned(),
                    value: "Status https://status.example".to_owned(),
                }],
                footer_text: None,
                url: Some("https://app.example/releases/1".to_owned()),
                thumbnail_url: Some("https://media.example/thumb.jpg".to_owned()),
                thumbnail_proxy_url: None,
                thumbnail_width: None,
                thumbnail_height: None,
                image_url: Some("https://media.example/image.jpg".to_owned()),
                image_proxy_url: None,
                image_width: None,
                image_height: None,
                video_url: Some("https://media.example/video.mp4".to_owned()),
            }],
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);
    state.open_selected_message_actions();

    let urls = state.selected_message_url_items();

    assert_eq!(
        urls.into_iter().map(|item| item.url).collect::<Vec<_>>(),
        vec!["https://app.example/releases/1"]
    );
}

#[test]
fn message_action_detects_urls_in_reply_quote_and_forwarded_snapshot() {
    let mut state = state_with_messages(1);
    state.push_event(latest_history_loaded(
        Id::new(2),
        vec![MessageInfo {
            content: Some("see above".to_owned()),
            reply: Some(ReplyInfo {
                author_id: None,
                author: "alice".to_owned(),
                content: Some("check https://reply.example/page".to_owned()),
                sticker_names: Vec::new(),
                mentions: Vec::new(),
            }),
            forwarded_snapshots: vec![MessageSnapshotInfo {
                content: Some("forwarded https://forward.example/doc".to_owned()),
                sticker_names: Vec::new(),
                mentions: Vec::new(),
                attachments: Vec::new(),
                embeds: vec![youtube_embed()],
                source_channel_id: None,
                timestamp: None,
            }],
            ..message_info(Id::new(2), 1)
        }],
    ));
    state.focus_pane(FocusPane::Messages);
    state.open_selected_message_actions();

    let urls = state.selected_message_url_items();

    assert_eq!(
        urls.into_iter().map(|item| item.url).collect::<Vec<_>>(),
        vec![
            "https://reply.example/page",
            "https://forward.example/doc",
            "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        ]
    );
}

#[test]
fn non_regular_message_actions_do_not_include_attachment_downloads() {
    let mut state = state_with_message_ids([]);
    state.push_event(message_create_event(MessageCreateFixture {
        guild_id: Some(Id::new(1)),
        channel_id: Id::new(2),
        message_id: Id::new(1),
        author_id: Id::new(99),
        message_kind: MessageKind::new(7),
        content: None,
        attachments: vec![video_attachment(1)],
        ..MessageCreateFixture::default()
    }));
    state.focus_pane(FocusPane::Messages);

    let action = state
        .selected_message_action_items()
        .into_iter()
        .find(|action| matches!(action.kind, MessageActionKind::DownloadAttachment(_)))
        .expect("download action placeholder should be visible");
    assert_eq!(action.label, "Download attachment");
    assert!(!action.enabled);
}

#[test]
fn message_action_items_keep_poll_actions_for_image_messages() {
    let mut state = state_with_image_messages(1, &[1]);
    state.focus_pane(FocusPane::Messages);
    state.push_event(message_create_event(MessageCreateFixture {
        guild_id: Some(Id::new(1)),
        channel_id: Id::new(2),
        message_id: Id::new(1),
        author_id: Id::new(99),
        poll: Some(poll_info(false)),
        content: Some(String::new()),
        attachments: vec![image_attachment(1)],
        ..MessageCreateFixture::default()
    }));

    let actions = state.selected_message_action_items();

    assert_eq!(
        actions.iter().map(|action| action.kind).collect::<Vec<_>>(),
        vec![
            MessageActionKind::OpenThread,
            MessageActionKind::DownloadAttachment(0),
            MessageActionKind::ShowReactionUsers,
            MessageActionKind::OpenPollVotePicker,
        ]
    );
    assert!(!actions[0].enabled);
    assert!(!actions[1].enabled);
    assert!(!actions[2].enabled);
    assert!(actions[3].enabled);
}

#[test]
fn single_select_poll_action_opens_picker_and_submits_one_answer() {
    let mut state = state_with_messages(1);
    state.focus_pane(FocusPane::Messages);
    state.push_event(message_create_event(MessageCreateFixture {
        guild_id: Some(Id::new(1)),
        channel_id: Id::new(2),
        message_id: Id::new(1),
        author_id: Id::new(99),
        poll: Some(poll_info(false)),
        content: Some(String::new()),
        ..MessageCreateFixture::default()
    }));
    state.open_selected_message_actions();

    assert!(state.select_message_action_row(3));
    assert_eq!(state.activate_selected_message_action(), None);
    assert!(state.is_poll_vote_picker_open());

    state.move_poll_vote_picker_down();
    state.toggle_selected_poll_vote_answer();
    let command = state.activate_poll_vote_picker();

    assert_eq!(
        command,
        Some(AppCommand::VotePoll {
            channel_id: Id::new(2),
            message_id: Id::new(1),
            answer_ids: vec![2],
        })
    );
}

#[test]
fn single_select_poll_picker_normalizes_multiple_initial_votes() {
    let mut state = state_with_messages(1);
    state.focus_pane(FocusPane::Messages);
    let mut poll = poll_info(false);
    poll.answers[1].me_voted = true;
    state.push_event(message_create_event(MessageCreateFixture {
        guild_id: Some(Id::new(1)),
        channel_id: Id::new(2),
        message_id: Id::new(1),
        author_id: Id::new(99),
        poll: Some(poll),
        content: Some(String::new()),
        ..MessageCreateFixture::default()
    }));
    state.open_selected_message_actions();
    assert!(state.select_message_action_row(3));
    assert_eq!(state.activate_selected_message_action(), None);

    assert_eq!(
        state.poll_vote_picker_items().map(|items| {
            items
                .iter()
                .map(|item| (item.answer_id, item.selected))
                .collect::<Vec<_>>()
        }),
        Some(vec![(1, true), (2, false)])
    );
    assert_eq!(
        state.activate_poll_vote_picker(),
        Some(AppCommand::VotePoll {
            channel_id: Id::new(2),
            message_id: Id::new(1),
            answer_ids: vec![1],
        })
    );
}

#[test]
fn multi_select_poll_action_opens_picker_and_submits_selected_answers() {
    let mut state = state_with_messages(1);
    state.focus_pane(FocusPane::Messages);
    state.push_event(message_create_event(MessageCreateFixture {
        guild_id: Some(Id::new(1)),
        channel_id: Id::new(2),
        message_id: Id::new(1),
        author_id: Id::new(99),
        poll: Some(poll_info(true)),
        content: Some(String::new()),
        ..MessageCreateFixture::default()
    }));

    let actions = state.selected_message_action_items();
    assert_eq!(actions[3].kind, MessageActionKind::OpenPollVotePicker);
    assert_eq!(actions[3].label, "Choose poll votes");

    state.open_selected_message_actions();
    assert!(state.select_message_action_row(3));
    assert_eq!(state.activate_selected_message_action(), None);
    assert!(state.is_poll_vote_picker_open());
    assert_eq!(
        state.poll_vote_picker_items().map(|items| {
            items
                .iter()
                .map(|item| (item.answer_id, item.selected))
                .collect::<Vec<_>>()
        }),
        Some(vec![(1, true), (2, false)])
    );

    state.move_poll_vote_picker_down();
    state.toggle_selected_poll_vote_answer();
    let command = state.activate_poll_vote_picker();

    assert_eq!(
        command,
        Some(AppCommand::VotePoll {
            channel_id: Id::new(2),
            message_id: Id::new(1),
            answer_ids: vec![1, 2],
        })
    );
}

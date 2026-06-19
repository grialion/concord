use super::*;

pub(in crate::tui::ui) fn render_guilds(frame: &mut Frame, area: Rect, state: &DashboardState) {
    let dashboard = state;
    let focused = state.focus() == FocusPane::Guilds;
    let filter_query = state.guild_pane_filter_query();
    let block = panel_block("Servers", focused);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (list_area, filter_area) = split_pane_filter_area(inner, filter_query.is_some());

    let entry_count = state.guild_pane_filtered_entries().len();
    let entries = state.visible_guild_pane_entries();
    let max_width = list_area.width.saturating_sub(6) as usize;
    let horizontal_scroll = state.guild_horizontal_scroll();
    let selected = state.focused_guild_selection();
    let rename_target = state.folder_rename_target_id();
    let rename_value = state.folder_rename_value();
    let rename_cursor = state.folder_rename_cursor_byte_index();
    let mut rename_cursor_position = None;
    let items: Vec<ListItem> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let is_selected = selected == Some(index);
            let is_active = state.is_active_guild_entry(entry);
            styled_list_item(
                match entry {
                    GuildPaneEntry::DirectMessages => {
                        let base_style = active_text_style(
                            is_active,
                            Style::default()
                                .fg(Color::Magenta)
                                .add_modifier(Modifier::BOLD),
                        );
                        let unread_count = state.direct_message_unread_count();
                        let badge = (unread_count > 0).then(|| {
                            notification_count_badge(ChannelUnreadState::Notified(
                                u32::try_from(unread_count).unwrap_or(u32::MAX),
                            ))
                        });
                        let badge_width =
                            badge.as_ref().map(|span| span.content.width()).unwrap_or(0);
                        let label_width = max_width.saturating_sub(badge_width);
                        let mut spans = vec![selection_marker(is_selected)];
                        if let Some(badge) = badge {
                            spans.push(badge);
                        }
                        spans.push(Span::styled(
                            truncate_display_width_from(
                                entry.label(),
                                horizontal_scroll,
                                label_width,
                            ),
                            base_style,
                        ));
                        ListItem::new(Line::from(spans))
                    }
                    GuildPaneEntry::FolderHeader { folder, collapsed } => {
                        let arrow = if *collapsed { "▶ " } else { "▼ " };
                        let icon = if *collapsed { "📁" } else { "📂" };
                        let color = folder_color(folder.color);
                        let is_renaming = folder.id.is_some() && folder.id == rename_target;
                        let label = if is_renaming {
                            rename_value.unwrap_or_default()
                        } else {
                            folder.name.as_deref().unwrap_or_default()
                        };
                        let title = if is_renaming {
                            format!("{icon} {label}")
                        } else if label.is_empty() {
                            icon.to_owned()
                        } else {
                            format!("{icon} {label}")
                        };
                        let label_width = max_width.saturating_sub(arrow.width());
                        if focused && is_renaming {
                            let cursor = rename_cursor.unwrap_or(label.len()).min(label.len());
                            let title_before_cursor = format!("{icon} {}", &label[..cursor]);
                            let title_cursor_col = title_before_cursor
                                .width()
                                .saturating_sub(horizontal_scroll)
                                .min(label_width);
                            let cursor_col = selection_marker(false)
                                .content
                                .width()
                                .saturating_add(arrow.width())
                                .saturating_add(title_cursor_col);
                            rename_cursor_position = Some((index as u16, cursor_col as u16));
                        }
                        ListItem::new(Line::from(vec![
                            selection_marker(is_selected),
                            Span::styled(arrow, Style::default().fg(color)),
                            Span::styled(
                                truncate_display_width_from(&title, horizontal_scroll, label_width),
                                Style::default().fg(color).add_modifier(Modifier::BOLD),
                            ),
                        ]))
                    }
                    GuildPaneEntry::Guild {
                        state: guild,
                        branch,
                    } => {
                        let prefix = branch.prefix();
                        let base_style = active_text_style(is_active, Style::default());
                        let is_muted = dashboard.guild_notification_muted(guild.id);
                        let unread = dashboard.sidebar_guild_unread(guild.id);
                        let (badge, mut name_style) = if is_active {
                            let (badge, _) = channel_unread_decoration(unread, base_style, false);
                            (badge, base_style)
                        } else if unread == ChannelUnreadState::Seen {
                            (None, base_style)
                        } else {
                            channel_unread_decoration(unread, base_style, false)
                        };
                        if is_muted {
                            name_style = name_style.add_modifier(Modifier::DIM);
                        }
                        let badge_width =
                            badge.as_ref().map(|span| span.content.width()).unwrap_or(0);
                        let label_width = max_width
                            .saturating_sub(prefix.width())
                            .saturating_sub(badge_width);
                        let mut spans = vec![
                            selection_marker(is_selected),
                            Span::styled(prefix, Style::default().fg(DIM)),
                        ];
                        if let Some(badge) = badge {
                            spans.push(badge);
                        }
                        spans.push(Span::styled(
                            truncate_display_width_from(
                                guild.name.as_str(),
                                horizontal_scroll,
                                label_width,
                            ),
                            name_style,
                        ));
                        ListItem::new(Line::from(spans))
                    }
                },
                is_selected,
            )
        })
        .collect();

    let list = List::new(items).highlight_style(highlight_style());
    frame.render_widget(list, list_area);
    if let Some((row, col)) = rename_cursor_position {
        frame.set_cursor_position(Position {
            x: list_area
                .x
                .saturating_add(col.min(list_area.width.saturating_sub(1))),
            y: list_area
                .y
                .saturating_add(row.min(list_area.height.saturating_sub(1))),
        });
    }

    render_pane_filter_bar_with_cursor(
        frame,
        filter_area,
        filter_query,
        state.guild_pane_filter_cursor(),
        focused,
    );

    render_vertical_scrollbar(
        frame,
        list_area,
        state.guild_scroll(),
        list_area.height as usize,
        entry_count,
    );
}

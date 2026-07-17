use ratatui::{
    prelude::*,
    widgets::{Paragraph, Block, Borders, List, ListItem, Wrap},
};
use ratatui_image::Image;
use crate::state::{AppState, InputMode};

pub fn draw_ui(frame: &mut Frame, state: &mut AppState) {
    let size = frame.area();

    // 縦長（画面分割など）の場合の判定
    let is_vertical = size.width < 90;

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),      // Player
            Constraint::Length(if is_vertical { 8 } else { 10 }), // Playlist
            Constraint::Length(1),   // Help Footer
        ])
        .split(size);

    render_player_area(frame, state, main_chunks[0], is_vertical);
    render_playlist_and_search(frame, state, main_chunks[1]);
    render_help(frame, state, main_chunks[2]);
}

fn render_player_area(frame: &mut Frame, state: &mut AppState, area: Rect, is_vertical: bool) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Art/Info & Lyrics
            Constraint::Length(3),  // Controls
        ])
        .split(area);

    if is_vertical {
        // --- 縦長レイアウト ---
        let vertical_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(10),  // Art & Info (高さを固定して確保)
                Constraint::Min(0),     // Lyrics
            ])
            .split(chunks[0]);
        
        // アートと情報を「横並び」に配置 (ご要望: 右横に大きく配置)
        let art_info_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(12), // Art (正方形に近いサイズ)
                Constraint::Min(0),     // Info (残り全部)
            ])
            .split(vertical_chunks[0]);

        render_art(frame, state, art_info_chunks[0]);
        render_large_info(frame, state, art_info_chunks[1]);
        
        let lyric_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(60, 60, 60))).title(" Lyrics ");
        frame.render_widget(lyric_block, vertical_chunks[1]);
        let lyric_inner = vertical_chunks[1].inner(Margin { horizontal: 1, vertical: 1 });
        state.lyric_area = Some(lyric_inner);
        state.video_area = Some(lyric_inner);
        render_lyrics(frame, state, lyric_inner);

    } else {
        // --- 横長（通常）レイアウト ---
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35), // Album Art & Info
                Constraint::Percentage(65), // Lyrics
            ])
            .split(chunks[0]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),     // Art
                Constraint::Length(6),  // Info
            ])
            .split(top_chunks[0]);

        render_art(frame, state, left_chunks[0]);
        render_large_info(frame, state, left_chunks[1]);

        let lyric_block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Rgb(60, 60, 60))).title(" Lyrics ");
        frame.render_widget(lyric_block, top_chunks[1]);
        let lyric_inner = top_chunks[1].inner(Margin { horizontal: 1, vertical: 1 });
        state.lyric_area = Some(lyric_inner);
        state.video_area = Some(lyric_inner);
        render_lyrics(frame, state, lyric_inner);
    }

    render_controls(frame, state, chunks[1]);
}

fn render_art(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let art_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)))
        .title(" Art ");
    frame.render_widget(art_block, area);
    
    let art_inner = area.inner(Margin { horizontal: 1, vertical: 1 });
    if state.album_art.is_some() {
        if let Some(picker) = &state.picker {
            // protocolが未生成またはエリア変更の場合のみ再生成
            if state.album_art_protocol.is_none() {
                if let Some(img) = &state.album_art {
                    match picker.new_protocol(img.clone(), art_inner, ratatui_image::Resize::Fit(None)) {
                        Ok(protocol) => { state.album_art_protocol = Some(protocol); }
                        Err(_) => {}
                    }
                }
            }
            if let Some(protocol) = &state.album_art_protocol {
                let image_widget = Image::new(protocol);
                frame.render_widget(image_widget, art_inner);
                return;
            }
        }
    }
    frame.render_widget(
        Paragraph::new("\n 🎵").alignment(Alignment::Center).style(Style::default().fg(Color::DarkGray)),
        art_inner
    );
}

fn render_large_info(frame: &mut Frame, state: &AppState, area: Rect) {
    let playing_track = state.playing_id.as_ref().and_then(|path| {
        state.tracks.iter().find(|t| &t.path == path)
    });

    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)))
        .title(" Now Playing ");

    if let Some(t) = playing_track {
        let video_icon = if t.video.is_some() { " 🎬" } else { "" };
        let fav_icon = if state.favorites.contains(&t.path) { " ⭐" } else { "" };

        let info_text = vec![
            Line::from(vec![
                Span::styled(format!("{}{}{}", t.title, video_icon, fav_icon), Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Artist: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&t.artist, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Album:  ", Style::default().fg(Color::DarkGray)),
                Span::styled(&t.album, Style::default().fg(Color::Gray)),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(info_text)
                .block(info_block)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left),
            area
        );
    } else {
        frame.render_widget(info_block, area);
    }
}

fn render_controls(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Seek Bar
            Constraint::Length(1), // Buttons
            Constraint::Min(0),
        ])
        .split(area);

    let playing_track = state.playing_id.as_ref().and_then(|path| {
        state.tracks.iter().find(|t| &t.path == path)
    });

    if let Some(t) = playing_track {
        let pos = if state.is_playing_video { state.video_playback_pos } else { state.playback_pos };
        let track_duration = t.duration.max(1.0);
        let duration = if state.is_playing_video && state.video_duration > 0.0 { state.video_duration } else { track_duration };
        let percent = ((pos / duration) * 100.0).min(100.0) as u16;
        
        // --- 可視化された再生バー ---
        let seek_bar_area = chunks[0];
        state.seek_bar_area = Some(seek_bar_area);
        
        let symbols = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];
        let width = seek_bar_area.width as usize;
        let filled_width = (width * percent as usize) / 100;
        let remainder = (width * percent as usize) % 100;
        let partial_idx = (remainder * (symbols.len() - 1)) / 100;

        let mut bar_str = String::with_capacity(width * 3);
        for i in 0..width {
            if i < filled_width {
                bar_str.push_str("█");
            } else if i == filled_width && partial_idx > 0 {
                bar_str.push_str(symbols[partial_idx]);
            } else {
                bar_str.push_str(" ");
            }
        }

        let progress_label = format!(" {:.0}:{:02} / {:.0}:{:02} ", pos / 60.0, (pos as i32) % 60, duration / 60.0, (duration as i32) % 60);
        
        frame.render_widget(
            Paragraph::new(bar_str).style(Style::default().fg(Color::Cyan).bg(Color::Rgb(30, 30, 30))),
            seek_bar_area
        );
        frame.render_widget(
            Paragraph::new(progress_label).alignment(Alignment::Right).style(Style::default().fg(Color::White)),
            seek_bar_area
        );

        // --- ボタン ---
        let btn_area = chunks[1];
        let center_x = btn_area.x + btn_area.width / 2;
        let prev_btn = " [⏮ Prev] ";
        let play_btn = if state.is_paused { " [▶ Play] " } else { " [⏸ Pause] " };
        let next_btn = " [⏭ Next] ";

        let prev_rect = Rect::new(center_x.saturating_sub(15), btn_area.y, 11, 1);
        let play_rect = Rect::new(center_x.saturating_sub(4), btn_area.y, 11, 1);
        let next_rect = Rect::new(center_x + 7, btn_area.y, 11, 1);

        state.prev_button_area = Some(prev_rect);
        state.play_button_area = Some(play_rect);
        state.next_button_area = Some(next_rect);

        frame.render_widget(Paragraph::new(prev_btn).style(Style::default().fg(Color::White)), prev_rect);
        frame.render_widget(Paragraph::new(play_btn).style(Style::default().fg(if state.is_paused { Color::Yellow } else { Color::Green })), play_rect);
        frame.render_widget(Paragraph::new(next_btn).style(Style::default().fg(Color::White)), next_rect);
    }
}

pub struct OdinVideoWidget<'a> {
    frame: &'a crate::renderer::OdinVideoFrame,
}

impl<'a> OdinVideoWidget<'a> {
    pub fn new(frame: &'a crate::renderer::OdinVideoFrame) -> Self {
        Self { frame }
    }
}

impl<'a> ratatui::widgets::Widget for OdinVideoWidget<'a> {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::prelude::Buffer) {
        let frame = self.frame;

        let render_w = std::cmp::min(area.width, frame.width);
        let render_h = std::cmp::min(area.height, frame.height);

        let start_x = area.x + (area.width.saturating_sub(render_w)) / 2;
        let start_y = area.y + (area.height.saturating_sub(render_h)) / 2;

        let data = &frame.data;

        for y in 0..render_h {
            for x in 0..render_w {
                let idx = ((y as usize) * (frame.width as usize) + (x as usize)) * 8;
                if idx + 7 < data.len() {
                    let char_code = u32::from_le_bytes([data[idx], data[idx+1], data[idx+2], data[idx+3]]);
                    let r = data[idx + 4];
                    let g = data[idx + 5];
                    let b = data[idx + 6];

                    if let Some(cell) = buf.cell_mut((start_x + x, start_y + y)) {
                        if let Some(c) = char::from_u32(char_code) {
                            cell.set_char(c);
                            cell.set_fg(ratatui::style::Color::Rgb(r, g, b));
                            cell.set_bg(ratatui::style::Color::Black);
                        }
                    }
                }
            }
        }
    }
}

fn render_lyrics(frame: &mut ratatui::Frame, state: &AppState, area: Rect) {
    if state.is_playing_video {
        if let Ok(mut size) = state.video_area_size.write() {
            if *size != (area.width, area.height) {
                *size = (area.width, area.height);
            }
        }

        if let Some(img) = &state.video_frame {
            let widget = OdinVideoWidget::new(img);
            frame.render_widget(widget, area);
            return;
        }
        frame.render_widget(Paragraph::new("🎬 Loading video...").alignment(Alignment::Center), area);
        return;
    }

    if state.parsed_lyrics.is_empty() {
        frame.render_widget(Paragraph::new(state.current_lyric.clone()).alignment(Alignment::Center), area);
        return;
    }

    let pos = state.playback_pos;
    let mut current_idx = 0;
    for (i, (time, _)) in state.parsed_lyrics.iter().enumerate() {
        if pos >= *time { current_idx = i; } else { break; }
    }

    let h = area.height as i32;
    let center_line = h / 2;
    
    if state.lyric_scroll_offset != 0 {
        let indicator = format!(" 📜 Scrolling ({:+} lines) ", -state.lyric_scroll_offset);
        frame.render_widget(Paragraph::new(indicator).style(Style::default().fg(Color::Yellow)).alignment(Alignment::Right), area);
    }

    for (i, (_time, text)) in state.parsed_lyrics.iter().enumerate() {
        let relative_idx = i as i32 - current_idx as i32 + state.lyric_scroll_offset;
        let y = center_line + relative_idx;

        if y >= 0 && y < h {
            let mut style = Style::default().fg(Color::Rgb(100, 100, 100));
            if i == current_idx {
                style = Style::default().fg(Color::White).add_modifier(Modifier::BOLD);
            } else if i < current_idx {
                style = Style::default().fg(Color::Rgb(60, 60, 60));
            }

            let line_area = Rect {
                x: area.x,
                y: area.y + y as u16,
                width: area.width,
                height: 1,
            };
            frame.render_widget(Paragraph::new(text.as_str()).alignment(Alignment::Center).style(style), line_area);
        }
    }
}

fn render_playlist_and_search(frame: &mut Frame, state: &AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Playlist
            Constraint::Percentage(30), // Search
        ])
        .split(area);

    let list_items: Vec<ListItem> = state.filtered_indices.iter().enumerate().map(|(i, &idx)| {
        let track = &state.tracks[idx];
        let is_selected = i == state.current;
        let is_playing = state.playing_id.as_ref().map_or(false, |id| id == &track.path);
        let is_fav = state.favorites.contains(&track.path);
        
        let video_indicator = if track.video.is_some() { " 🎬" } else { "" };
        let fav_indicator = if is_fav { " ⭐" } else { "" };
        
        let mut style = Style::default();
        if is_selected { style = style.bg(Color::Rgb(40, 40, 80)).fg(Color::White); }
        if is_playing { style = style.fg(Color::Cyan); }
        
        let prefix = if is_playing { "▶ " } else { "  " };
        ListItem::new(format!("{}{} - {}{}{}", prefix, track.title, track.artist, video_indicator, fav_indicator)).style(style)
    }).collect();

    let mut list_state = state.list_state.clone();
    frame.render_stateful_widget(
        List::new(list_items).block(Block::default().borders(Borders::ALL).title(" Playlist ").border_style(Style::default().fg(Color::Rgb(50, 50, 50)))),
        chunks[0],
        &mut list_state
    );

    let search_label = if matches!(state.input_mode, InputMode::Editing) { " Searching... " } else { " Search [/] " };
    let search_style = if matches!(state.input_mode, InputMode::Editing) { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::DarkGray) };
    frame.render_widget(
        Paragraph::new(format!(" > {} ", state.search))
            .block(Block::default().borders(Borders::ALL).title(search_label).border_style(search_style))
            .style(if matches!(state.input_mode, InputMode::Editing) { Style::default().fg(Color::White) } else { Style::default().fg(Color::Gray) }),
        chunks[1],
    );
}

fn render_help(frame: &mut Frame, state: &AppState, area: Rect) {
    let help_text = " q:Quit | /:Search | f:Fav | Shift+F:Toggle View | Space:Play/Pause | v:TUI Video | Shift+V:MPV ";
    let action_text = format!(" [{}] ", state.last_action);
    let help_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(action_text.len() as u16),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(help_text).style(Style::default().fg(Color::Rgb(80, 80, 80))), help_chunks[0]);
    frame.render_widget(Paragraph::new(action_text).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)), help_chunks[1]);
}

use ratatui::{
    prelude::*,
    widgets::{Paragraph, Block, Borders, List, ListItem, Gauge, Wrap},
};
use ratatui_image::{Image, Resize};
use crate::state::{AppState, InputMode};

pub fn draw_ui(frame: &mut Frame, state: &mut AppState) {
    let size = frame.area();

    // 1. Help Footer (最下部)
    // 2. Playlist & Search (下部)
    // 3. Player Area (上部 - 残り全部)
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),      // Player
            Constraint::Length(8),   // Playlist & Search (少し高さを抑える)
            Constraint::Length(1),   // Help Footer
        ])
        .split(size);

    render_player_area(frame, state, main_chunks[0]);
    render_playlist_and_search(frame, state, main_chunks[1]);
    render_help(frame, state, main_chunks[2]);
}

fn render_player_area(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Art/Info & Lyrics
            Constraint::Length(3),  // Controls (Gauge & Buttons)
        ])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35), // Album Art & Info
            Constraint::Percentage(65), // Lyrics
        ])
        .split(chunks[0]);

    // --- Left Column: Art & Info ---
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),     // Album Art
            Constraint::Length(6),  // Track Info (Title, Artist, Album)
        ])
        .split(top_chunks[0]);

    // Album Art
    let art_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)))
        .title(" Album Art ");
    frame.render_widget(art_block, left_chunks[0]);
    
    let art_inner = left_chunks[0].inner(Margin { horizontal: 1, vertical: 1 });
    if let Some(img) = &state.album_art {
        if let Some(picker) = &state.picker {
            let dyn_img = img.clone();
            if let Ok(protocol) = picker.new_protocol(dyn_img, art_inner, Resize::Fit(None)) {
                let image_widget = Image::new(&protocol);
                frame.render_widget(image_widget, art_inner);
            }
        }
    } else {
        frame.render_widget(
            Paragraph::new("\n\n\n\n 🎵\n No Art").alignment(Alignment::Center).style(Style::default().fg(Color::DarkGray)),
            art_inner
        );
    }

    // Track Info (Artの下)
    let info_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)));
    
    let playing_track = state.playing_id.as_ref().and_then(|path| {
        state.tracks.iter().find(|t| &t.path == path)
    });

    if let Some(t) = playing_track {
        let info_text = vec![
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&t.title, Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("Artist: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&t.artist, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                Span::styled("Album: ", Style::default().fg(Color::DarkGray)),
                Span::styled(&t.album, Style::default().fg(Color::Gray)),
            ]),
        ];
        frame.render_widget(
            Paragraph::new(info_text)
                .block(info_block.clone())
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left),
            left_chunks[1].inner(Margin { horizontal: 1, vertical: 1 })
        );
        // ブロック自体を描画
        frame.render_widget(info_block, left_chunks[1]);
    } else {
        frame.render_widget(info_block, left_chunks[1]);
    }

    // --- Lyrics ---
    let lyric_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Rgb(60, 60, 60)))
        .title(" Lyrics ");
    frame.render_widget(lyric_block, top_chunks[1]);
    
    let lyric_inner = top_chunks[1].inner(Margin { horizontal: 1, vertical: 1 });
    state.lyric_area = Some(lyric_inner);
    render_lyrics(frame, state, lyric_inner);

    // --- Controls ---
    render_controls(frame, state, chunks[1]);
}

fn render_controls(frame: &mut Frame, state: &mut AppState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Gauge
            Constraint::Length(1), // Buttons
            Constraint::Min(0),
        ])
        .split(area);

    let playing_track = state.playing_id.as_ref().and_then(|path| {
        state.tracks.iter().find(|t| &t.path == path)
    });

    if let Some(t) = playing_track {
        // Progress Gauge
        let pos = state.playback_pos;
        let duration = t.duration.max(1.0);
        let percent = ((pos / duration) * 100.0).min(100.0) as u16;
        let progress_label = format!("{:.0}:{:02} / {:.0}:{:02}", pos / 60.0, (pos as i32) % 60, duration / 60.0, (duration as i32) % 60);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Rgb(20, 20, 20)))
            .percent(percent)
            .label(Span::styled(progress_label, Style::default().fg(Color::White)));
        frame.render_widget(gauge, chunks[0]);

        // Buttons
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

fn render_lyrics(frame: &mut Frame, state: &AppState, area: Rect) {
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
    
    for (i, (_time, text)) in state.parsed_lyrics.iter().enumerate() {
        let relative_idx = i as i32 - current_idx as i32;
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

    // Playlist
    let list_items: Vec<ListItem> = state.filtered_indices.iter().enumerate().map(|(i, &idx)| {
        let track = &state.tracks[idx];
        let is_selected = i == state.current;
        let is_playing = state.playing_id.as_ref().map_or(false, |id| id == &track.path);
        
        let mut style = Style::default();
        if is_selected { style = style.bg(Color::Rgb(40, 40, 80)).fg(Color::White); }
        if is_playing { style = style.fg(Color::Cyan); }
        
        let prefix = if is_playing { "▶ " } else { "  " };
        ListItem::new(format!("{}{} - {}", prefix, track.title, track.artist)).style(style)
    }).collect();

    let list_title = if state.show_favorites_only { " ⭐ Favorites " } else { " ☰ Tracks " };
    let mut list_state = state.list_state.clone();
    frame.render_stateful_widget(
        List::new(list_items).block(Block::default().borders(Borders::ALL).title(list_title).border_style(Style::default().fg(Color::Rgb(50, 50, 50)))),
        chunks[0],
        &mut list_state
    );

    // Search
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
    let help_text = " q:Quit | /:Search | f:Fav | Shift+F:Toggle View | Space:Play/Pause | ←/→:Prev/Next ";
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

use ratatui::{
    prelude::*,
    widgets::{Paragraph, Block, Borders, Wrap, List, ListItem, Gauge},
};
use crate::state::{AppState, InputMode};

pub fn draw_ui(frame: &mut Frame, state: &AppState) {
    let size = frame.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Top Status Bar
            Constraint::Length(8), // Track List
            Constraint::Length(9), // Rich Control Panel
            Constraint::Min(3),    // Lyrics
            Constraint::Length(3), // Search Bar
            Constraint::Length(1), // Help Footer
        ])
        .split(size);

    // 1. Top Status Bar
    let status_style = if state.is_paused { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Green) };
    let top_status = Line::from(vec![
        Span::styled(format!(" {} ", state.last_action), Style::default().bg(Color::White).fg(Color::Black).add_modifier(Modifier::BOLD)),
        Span::raw(" "),
        Span::styled(format!("Status: {} | Fetch: {:.1}%", state.status_msg, state.load_progress), status_style),
    ]);
    frame.render_widget(Paragraph::new(top_status).alignment(Alignment::Right), chunks[0]);

    // 2. Track List
    let list_items: Vec<ListItem> = if state.tracks.is_empty() && state.is_loading {
        vec![ListItem::new("Connecting...")]
    } else {
        state.filtered_indices.iter().enumerate().map(|(i, &idx)| {
            let track = &state.tracks[idx];
            let is_selected = i == state.current;
            let is_playing = state.playing_id.as_ref().map_or(false, |id| id == &track.path);
            
            let mut style = Style::default();
            if is_selected { style = style.bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD); }
            if is_playing && !state.is_paused { style = style.fg(Color::Cyan); }

            let prefix = if is_playing { ">> " } else { "   " };
            ListItem::new(format!("{}{} - {}", prefix, track.title, track.artist)).style(style)
        }).collect()
    };

    let list_block = Block::default().borders(Borders::ALL).title(" Track List ");
    let mut list_state = state.list_state.clone();
    frame.render_stateful_widget(List::new(list_items).block(list_block), chunks[1], &mut list_state);

    // 3. Control Panel
    let playing_track = state.playing_id.as_ref().and_then(|path| {
        state.tracks.iter().find(|t| &t.path == path)
    });

    let panel_block = Block::default().borders(Borders::ALL).title(" Now Playing ");
    let panel_inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Artist / Album
            Constraint::Length(1), // Progress Bar
            Constraint::Length(1), // Buttons
        ])
        .split(chunks[2]);

    if let Some(t) = playing_track {
        // --- 曲名 ---
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Cyan)),
                Span::styled(&t.title, Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
            ])).alignment(Alignment::Center),
            panel_inner[0]
        );

        // --- アーティスト / アルバム (白文字に変更) ---
        frame.render_widget(
            Paragraph::new(format!("Artist: {}  /  Album: {}", t.artist, t.album))
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center),
            panel_inner[1]
        );

        // --- プログレスバー ---
        let pos = state.playback_pos;
        let duration = t.duration.max(1.0);
        let percent = ((pos / duration) * 100.0).min(100.0) as u16;
        let progress_label = format!("{:.0}:{:02} / {:.0}:{:02}", 
            pos / 60.0, (pos as i32) % 60, duration / 60.0, (duration as i32) % 60);
        
        let gauge = Gauge::default()
            .block(Block::default())
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Rgb(30, 30, 30)))
            .percent(percent)
            .label(progress_label);
        frame.render_widget(gauge, panel_inner[2]);

        let controls = Line::from(vec![
            Span::styled(" [⏮ Prev] ", Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled(format!(" [{}] ", if state.is_paused { "▶ PLAY" } else { "⏸ PAUSE" }), 
                Style::default().fg(if state.is_paused { Color::Yellow } else { Color::Green }).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(" [⏭ Next] ", Style::default().fg(Color::White)),
        ]);
        frame.render_widget(Paragraph::new(controls).alignment(Alignment::Center), panel_inner[3]);

    } else {
        frame.render_widget(
            Paragraph::new("\n(Stopped - Select a song and press Enter)")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray)),
            chunks[2]
        );
    }
    frame.render_widget(panel_block, chunks[2]);

    // 4. Karaoke Lyrics
    let lyric_style = if state.is_paused { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) };
    frame.render_widget(
        Paragraph::new(state.current_lyric.clone())
            .block(Block::default().borders(Borders::TOP).title(" Lyrics "))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(lyric_style),
        chunks[3],
    );

    // 5. Search Bar
    frame.render_widget(
        Paragraph::new(format!(" > {} ", state.search))
            .block(Block::default().borders(Borders::ALL).title(" Search: [/] to type "))
            .style(if matches!(state.input_mode, InputMode::Editing) { Style::default().fg(Color::Yellow) } else { Style::default() }),
        chunks[4],
    );

    // 6. Help Footer
    frame.render_widget(
        Paragraph::new("Quit: q | Search: / | Select: Up/Down | Seek: Left/Right | Play: Enter | Pause: Space")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[5],
    );
}

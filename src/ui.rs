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
            Constraint::Length(5), // 1. Now Playing (高さを微調整)
            Constraint::Length(4), // 2. Lyrics (高さを微調整)
            Constraint::Min(10),   // 3. Track List (メイン)
            Constraint::Length(3), // 4. Search Bar
            Constraint::Length(1), // 5. Help Footer
        ])
        .split(size);

    // 1. Now Playing
    let playing_track = state.playing_id.as_ref().and_then(|path| {
        state.tracks.iter().find(|t| &t.path == path)
    });

    let panel_inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Artist / Album
            Constraint::Length(1), // Progress
            Constraint::Length(1), // Controls
            Constraint::Min(0),
        ])
        .split(chunks[0]);

    if let Some(t) = playing_track {
        frame.render_widget(Paragraph::new(Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Color::Cyan)),
            Span::styled(&t.title, Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
        ])).alignment(Alignment::Center), panel_inner[0]);

        frame.render_widget(Paragraph::new(format!("Artist: {}  /  Album: {}", t.artist, t.album))
            .style(Style::default().fg(Color::White)).alignment(Alignment::Center), panel_inner[1]);

        let pos = state.playback_pos;
        let duration = t.duration.max(1.0);
        let percent = ((pos / duration) * 100.0).min(100.0) as u16;
        let progress_label = format!("{:.0}:{:02} / {:.0}:{:02}", pos / 60.0, (pos as i32) % 60, duration / 60.0, (duration as i32) % 60);
        let gauge = Gauge::default().gauge_style(Style::default().fg(Color::Cyan).bg(Color::Rgb(30, 30, 30))).percent(percent).label(progress_label);
        frame.render_widget(gauge, panel_inner[2]);

        let controls = Line::from(vec![
            Span::styled(" [Prev: ←] ", Style::default().fg(Color::White)),
            Span::raw("  "),
            Span::styled(format!(" [{}] ", if state.is_paused { "PLAY: Space" } else { "PAUSE: Space" }), 
                Style::default().fg(if state.is_paused { Color::Yellow } else { Color::Green }).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(" [Next: →] ", Style::default().fg(Color::White)),
        ]);
        frame.render_widget(Paragraph::new(controls).alignment(Alignment::Center), panel_inner[3]);
    } else {
        frame.render_widget(Paragraph::new("\n(Stopped - Press Enter to Play)").alignment(Alignment::Center).style(Style::default().fg(Color::DarkGray)), chunks[0]);
    }

    // 2. Lyrics
    let lyric_style = if state.is_paused { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) };
    frame.render_widget(
        Paragraph::new(state.current_lyric.clone())
            .block(Block::default().borders(Borders::TOP).title(" Lyrics "))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(lyric_style),
        chunks[1],
    );

    // 3. Track List (Borders::ALL を復活)
    let list_items: Vec<ListItem> = if state.tracks.is_empty() && state.is_loading {
        vec![ListItem::new(format!("Connecting... ({:.1}%)", state.load_progress))]
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

    let list_block = Block::default()
        .borders(Borders::ALL) // ALLを復活
        .title(format!(" Track List ({}/{} songs) ", state.filtered_indices.len(), state.tracks.len()));
    
    let mut list_state = state.list_state.clone();
    frame.render_stateful_widget(List::new(list_items).block(list_block), chunks[2], &mut list_state);

    // 4. Search Bar
    let search_label = match state.input_mode {
        InputMode::Normal => " Search: [/] to type ",
        InputMode::Editing => " Searching... (Enter to finish) ",
    };
    frame.render_widget(
        Paragraph::new(format!(" > {} ", state.search))
            .block(Block::default().borders(Borders::ALL).title(search_label))
            .style(if matches!(state.input_mode, InputMode::Editing) { Style::default().fg(Color::Yellow) } else { Style::default() }),
        chunks[3],
    );

    // 5. Help Footer
    frame.render_widget(
        Paragraph::new("Quit: q | Search: / | Select: Up/Down | Seek: L/R (Hold) | Play: Enter | Pause: Space")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[4],
    );
}

use ratatui::{
    prelude::*,
    widgets::{Paragraph, Block, Borders, Wrap, List, ListItem, Gauge},
};
use ratatui_image::{Image, Resize};
use crate::state::{AppState, InputMode};

pub fn draw_ui(frame: &mut Frame, state: &AppState) {
    let size = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(7), // 1. Now Playing (高さを増やしてアートに対応)
            Constraint::Length(4), // 2. Lyrics
            Constraint::Min(10),   // 3. Track List
            Constraint::Length(3), // 4. Search Bar
            Constraint::Length(1), // 5. Help Footer
        ])
        .split(size);

    // 1. Now Playing
    let playing_track = state.playing_id.as_ref().and_then(|path| {
        state.tracks.iter().find(|t| &t.path == path)
    });

    let now_playing_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(10), // Album Art
            Constraint::Min(0),     // Track Info
        ])
        .split(chunks[0]);

    // Render Album Art
    if let Some(img) = &state.album_art {
        if let Some(picker) = &state.picker {
            let image_area = now_playing_chunks[0].inner(Margin { horizontal: 1, vertical: 0 });
            // プロトコルを作成してウィジェットをレンダリング
            // ratatui-image 10.x では Picker からプロトコルを作成する
            let dyn_img = img.clone();
            if let Ok(protocol) = picker.new_protocol(dyn_img, image_area, Resize::Fit(None)) {
                let image_widget = Image::new(&protocol);
                frame.render_widget(image_widget, image_area);
            }
        }
    } else {
        frame.render_widget(Paragraph::new("\n 🎵").alignment(Alignment::Center), now_playing_chunks[0]);
    }

    let panel_inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // Artist / Album
            Constraint::Length(1), // Progress
            Constraint::Length(1), // Controls
            Constraint::Min(0),
        ])
        .split(now_playing_chunks[1]);

    if let Some(t) = playing_track {
        let video_icon = if t.video.is_some() { " 🎬" } else { "" };
        let fav_icon = if state.favorites.contains(&t.path) { " ⭐" } else { "" };
        frame.render_widget(Paragraph::new(Line::from(vec![
            Span::styled("Title: ", Style::default().fg(Color::Cyan)),
            Span::styled(format!("{}{}{}", t.title, video_icon, fav_icon), Style::default().add_modifier(Modifier::BOLD).fg(Color::White)),
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
            Span::raw("  "),
            Span::styled(format!(" [Vol: {:.0}% (-/+)] ", state.volume * 100.0), Style::default().fg(Color::Magenta)),
        ]);
        frame.render_widget(Paragraph::new(controls).alignment(Alignment::Center), panel_inner[3]);
    } else {
        frame.render_widget(Paragraph::new("\n(Stopped - Press Enter to Play)").alignment(Alignment::Center).style(Style::default().fg(Color::DarkGray)), now_playing_chunks[1]);
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

    // 3. Track List
    let list_items: Vec<ListItem> = if state.tracks.is_empty() && state.is_loading {
        vec![ListItem::new(format!("Connecting... ({:.1}%)", state.load_progress))]
    } else {
        state.filtered_indices.iter().enumerate().map(|(i, &idx)| {
            let track = &state.tracks[idx];
            let is_selected = i == state.current;
            let is_playing = state.playing_id.as_ref().map_or(false, |id| id == &track.path);
            let is_fav = state.favorites.contains(&track.path);
            
            let video_indicator = if track.video.is_some() { " 🎬" } else { "" };
            let fav_indicator = if is_fav { " ⭐" } else { "" };
            
            let mut style = Style::default();
            if is_selected { style = style.bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD); }
            if is_playing && !state.is_paused { style = style.fg(Color::Cyan); }
            
            let prefix = if is_playing { ">> " } else { "   " };
            ListItem::new(format!("{}{} - {}{}{}", prefix, track.title, track.artist, video_indicator, fav_indicator)).style(style)
        }).collect()
    };

    let list_title = if state.show_favorites_only {
        format!(" Favorite Tracks ({}/{} songs) ", state.filtered_indices.len(), state.favorites.len())
    } else {
        format!(" All Tracks ({}/{} songs) ", state.filtered_indices.len(), state.tracks.len())
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(list_title);
    
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
    let help_text = if state.show_favorites_only {
        "Quit: q | Search: / | Fav: f | View All: Shift+F | Video: v | Vol: -/+ | Reorder: Alt+↑/↓ | Select: ↑/↓ | Play: Enter"
    } else {
        "Quit: q | Search: / | Fav: f | View Favorites: Shift+F | Video: v | Vol: -/+ | Reorder: Alt+↑/↓ | Select: ↑/↓ | Play: Enter"
    };
    frame.render_widget(
        Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray)),
        chunks[4],
    );
}

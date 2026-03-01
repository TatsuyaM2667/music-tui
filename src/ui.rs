use ratatui::{
    prelude::*,
    widgets::{Paragraph, Block, Borders, Wrap, List, ListItem},
};
use crate::state::{AppState, InputMode};

pub fn draw_ui(frame: &mut Frame, state: &AppState) {
    let size = frame.size();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Status Bar
            Constraint::Length(8), // Track List
            Constraint::Length(5), // Control Panel (ここを強化)
            Constraint::Min(3),    // Lyrics
            Constraint::Length(3), // Search
            Constraint::Length(1), // Help
        ])
        .split(size);

    // 1. Status Bar
    let status_style = if state.is_paused { Style::default().fg(Color::Yellow) } else { Style::default().fg(Color::Green) };
    frame.render_widget(
        Paragraph::new(format!(" ● {} Status: {}", state.last_action, state.status_msg))
            .alignment(Alignment::Right)
            .style(status_style),
        chunks[0],
    );

    // 2. Track List
    let list_items: Vec<ListItem> = if state.is_loading {
        vec![ListItem::new("Loading tracks...")]
    } else {
        state.filtered_indices.iter().enumerate().map(|(i, &idx)| {
            let track = &state.tracks[idx];
            let is_selected = i == state.current;
            let is_playing = state.playing_id.as_ref().map_or(false, |id| id == &AppState::id_from_path(&track.path));
            
            let mut style = Style::default();
            if is_selected {
                style = style.bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD);
            }
            if is_playing && !state.is_paused {
                style = style.fg(Color::Cyan);
            }

            let prefix = if is_playing { " 󰝚 " } else { "   " };
            ListItem::new(format!("{}{}- {}", prefix, track.title, track.artist)).style(style)
        }).collect()
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" 曲リスト ({}/{}曲) ", state.filtered_indices.len(), state.tracks.len()));
    
    let mut list_state = state.list_state.clone();
    frame.render_stateful_widget(List::new(list_items).block(list_block), chunks[1], &mut list_state);

    // 3. Control Panel (ここを強化して操作を表示)
    let selected_track = state.current_track();
    let playing_track = state.playing_id.as_ref().and_then(|id| {
        state.tracks.iter().find(|t| AppState::id_from_path(&t.path) == *id)
    });

    let panel_block = Block::default().borders(Borders::NONE);
    let mut panel_content = Vec::new();

    if let Some(t) = playing_track {
        let pos = state.playback_pos;
        let progress = format!("{:.0}:{:02} / {:.0}:{:02}",
            pos / 60.0, (pos as i32) % 60, t.duration as i32 / 60, (t.duration as i32) % 60);
        
        panel_content.push(Line::from(vec![
            Span::styled(format!(" {} ", if state.is_paused { "⏸ PAUSED" } else { "▶ PLAYING" }), 
                Style::default().bg(if state.is_paused { Color::Yellow } else { Color::Green }).fg(Color::Black).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(format!("{} - {}", t.title, t.artist), Style::default().add_modifier(Modifier::BOLD)),
        ]));
        panel_content.push(Line::from(vec![
            Span::styled(format!("  {}  ", progress), Style::default().fg(Color::Cyan)),
        ]));
    } else {
        panel_content.push(Line::from("  (STOPPED)"));
    }

    if let Some(t) = selected_track {
        panel_content.push(Line::from(vec![
            Span::styled("  SELECTING: ", Style::default().fg(Color::DarkGray)),
            Span::raw(format!("{} ", t.title)),
            Span::styled("[ENTER] to Play", Style::default().fg(Color::Yellow)),
        ]));
    }

    frame.render_widget(
        Paragraph::new(panel_content)
            .block(panel_block)
            .alignment(Alignment::Center),
        chunks[2],
    );

    // 4. Karaoke Lyrics
    frame.render_widget(
        Paragraph::new(state.current_lyric.clone())
            .block(Block::default().borders(Borders::TOP).title(" 歌詞 (Karaoke) "))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        chunks[3],
    );

    // 5. Search Bar
    frame.render_widget(
        Paragraph::new(format!(" > {} ", state.search))
            .block(Block::default().borders(Borders::ALL).title("  検索: [/]キーで入力 "))
            .style(if matches!(state.input_mode, InputMode::Editing) { Style::default().fg(Color::Yellow) } else { Style::default() }),
        chunks[4],
    );

    // 6. Help
    frame.render_widget(
        Paragraph::new("終了: q | 検索: / | 選択: ↑/↓ | 前後: ←/→ | 再生: Enter | 停止: Space")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[5],
    );
}

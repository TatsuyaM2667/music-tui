use ratatui::{
    prelude::*,
    widgets::{Paragraph, Block, Borders, Wrap},
};
use crate::state::AppState;

pub fn draw_ui(frame: &mut Frame, state: &AppState) {
    let size = frame.size();

    // 全体を囲うブロック
    let main_block = Block::default()
        .borders(Borders::ALL)
        .title(" Music TUI Player ");
    frame.render_widget(main_block, size);

    // 内側のエリアを計算
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Title / Status
            Constraint::Length(1), // Artist / Album
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Progress & Controls
            Constraint::Min(3),    // Lyrics
            Constraint::Length(1), // Search
            Constraint::Length(1), // Help
        ])
        .split(size);

    // エラー表示
    if let Some(ref err) = state.error_msg {
        frame.render_widget(
            Paragraph::new(format!("❌ Error: {}", err))
                .style(Style::default().fg(Color::Red))
                .block(Block::default().borders(Borders::ALL).title(" Error ")),
            size, // 画面全体に被せる
        );
        return;
    }

    // 読み込み中表示 (アニメーション)
    if state.is_loading {
        let spinners = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let frame_idx = (state.tick_count / 2) % spinners.len() as u64;
        frame.render_widget(
            Paragraph::new(format!("{} Loading tracks (Filtering 58MB JSON)...", spinners[frame_idx as usize]))
                .alignment(Alignment::Center),
            chunks[3],
        );
        return;
    }

    if state.tracks.is_empty() {
        frame.render_widget(Paragraph::new("No tracks found."), chunks[0]);
        return;
    }

    let track = state.current_track();

    // 1. Title
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Title: ", Style::default().fg(Color::Cyan)),
            Span::styled(&track.title, Style::default().add_modifier(Modifier::BOLD)),
        ])),
        chunks[0],
    );

    // 2. Artist / Album
    frame.render_widget(
        Paragraph::new(format!(" Artist: {} / Album: {}", track.artist, track.album)),
        chunks[1],
    );

    // 4. Progress & Controls
    let controls_area = chunks[3];
    let pos = state.playback_pos;
    let progress_text = format!("{:.0}:{:02} / {:.0}:{:02}",
        pos / 60.0, (pos as i32) % 60, track.duration as i32 / 60, (track.duration as i32) % 60);
    
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(progress_text),
            Line::from(vec![
                Span::styled(" [ Prev(←) ] ", Style::default().bg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(" [ Play(Space) ] ", Style::default().bg(Color::DarkGray)),
                Span::raw(" "),
                Span::styled(" [ Next(→) ] ", Style::default().bg(Color::DarkGray)),
            ]),
        ])
        .alignment(Alignment::Center),
        controls_area,
    );

    // 5. Lyrics
    frame.render_widget(
        Paragraph::new(state.current_lyric.clone())
            .block(Block::default().borders(Borders::TOP).title(" Lyrics "))
            .wrap(Wrap { trim: true }),
        chunks[4],
    );

    // 6. Search
    frame.render_widget(
        Paragraph::new(format!("Search: {}", state.search))
            .block(Block::default().borders(Borders::TOP)),
        chunks[5],
    );

    // 7. Help
    frame.render_widget(
        Paragraph::new("Quit: q | Play: Space | Prev: ← | Next: → | [Mouse Disabled for Stability]")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[6],
    );

    // ステータス表示 (Titleの横などに)
    frame.render_widget(
        Paragraph::new(format!(" Status: {}", state.status_msg))
            .alignment(Alignment::Right)
            .style(Style::default().fg(Color::Yellow)),
        chunks[0],
    );
}

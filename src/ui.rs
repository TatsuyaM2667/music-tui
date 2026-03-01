use ratatui::{
    prelude::*,
    widgets::Paragraph,
};
use crate::state::AppState;

pub fn draw_ui(frame: &mut Frame, state: &AppState) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
            Constraint::Min(1),
        ])
        .split(frame.size());

    let track = state.current_track();

    frame.render_widget(Paragraph::new(track.title.clone()), layout[0]);
    frame.render_widget(
        Paragraph::new(format!("{} / {}", track.artist, track.album)),
        layout[1],
    );

    frame.render_widget(
        Paragraph::new(format!("{:.0}:{:02} / {:.0}:{:02}",
            0, 0, track.duration as i32 / 60, (track.duration as i32) % 60)),
        layout[2],
    );

    frame.render_widget(Paragraph::new(state.current_lyric.clone()), layout[3]);

    frame.render_widget(
        Paragraph::new("[ ← Prev ]  [ ▶ ]  [ Next → ]"),
        layout[4],
    );

    frame.render_widget(
        Paragraph::new(format!("[ Search: {} ]", state.search)),
        layout[5],
    );
}

mod api;
mod state;
mod player;
mod ui;

use api::*;
use state::*;

use std::io::stdout;

use anyhow::Result;

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
    cursor::{Hide, Show},
    event::{self, Event, KeyCode},
};

#[tokio::main]
async fn main() -> Result<()> {
    let tracks = match fetch_tracks().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("❌ fetch_tracks() でエラー発生: {:?}", e);
            return Ok(());
        }
    };

    let mut state = AppState::new(tracks);

    let mut out = stdout();

    // GNOME Terminal で必要な強制初期化
    execute!(
        out,
        Clear(ClearType::All),
        EnterAlternateScreen,
        Hide
    )?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui::draw_ui(f, &state))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,

                    KeyCode::Left => {
                        if state.current > 0 {
                            state.current -= 1;
                        }
                    }

                    KeyCode::Right => {
                        if state.current < state.tracks.len() - 1 {
                            state.current += 1;
                        }
                    }

                    KeyCode::Char(' ') => {
                        let id = AppState::id_from_path(&state.current_track().path);
                        let url = stream_url(&id);
                        let _ = player::play_from_url(&url).await;
                    }

                    _ => {}
                }
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        Show,
        LeaveAlternateScreen
    )?;

    Ok(())
}

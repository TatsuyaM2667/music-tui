mod api;
mod player;
mod ui;
mod state;

use state::AppState;
use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{terminal, execute};
use std::io::stdout;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tracks = api::fetch_tracks().await?;
    let mut state = AppState::new(tracks);

    let mut stdout = stdout();
    execute!(stdout, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        terminal.draw(|f| ui::draw_ui(f, &state))?;
        // キー入力処理は後で追加
    }
}

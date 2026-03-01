mod api;
mod state;
mod player;
mod ui;

use api::*;
use state::*;

use std::io::stdout;
use std::time::Duration;
use std::sync::atomic::Ordering;

use anyhow::Result;

use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};

use crossterm::{
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen, Clear, ClearType},
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, DisableMouseCapture},
};

fn restore_terminal() {
    let _ = terminal::disable_raw_mode();
    let _ = execute!(stdout(), DisableMouseCapture, Show, LeaveAlternateScreen);
}

#[tokio::main]
async fn main() -> Result<()> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    let mut state = AppState::new(vec![]);
    let mut out = stdout();

    execute!(out, EnterAlternateScreen, Clear(ClearType::All), Hide, DisableMouseCapture)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    // 1. 曲受信用チャンネル
    let (tx_track, mut rx_track) = tokio::sync::mpsc::channel::<TrackInfo>(100);
    let (tx_progress, mut rx_progress) = tokio::sync::mpsc::channel::<f64>(100);
    
    let pause_signal = state.fetch_paused.clone();
    tokio::spawn(async move {
        let _ = fetch_tracks_streaming(tx_track, tx_progress, pause_signal).await;
    });

    // 2. 歌詞受信用
    let (tx_lyrics, mut rx_lyrics) = tokio::sync::mpsc::channel::<(String, Result<String>)>(10);

    // 3. プレイヤーエラー受信用
    let (tx_player_err, mut rx_player_err) = tokio::sync::mpsc::channel::<String>(10);

    let mut last_tick = std::time::Instant::now();
    let mut last_key: Option<(KeyCode, std::time::Instant)> = None;
    
    loop {
        terminal.draw(|f| ui::draw_ui(f, &state))?;

        // --- データの受信処理 ---
        while let Ok(msg) = rx_player_err.try_recv() {
            state.error_msg = Some(msg);
        }

        while let Ok(p) = rx_progress.try_recv() {
            state.load_progress = p;
        }

        while let Ok(track) = rx_track.try_recv() {
            state.tracks.push(track);
            state.update_search();
            if state.tracks.len() == 1 { state.list_state.select(Some(0)); }
            state.is_loading = state.load_progress < 99.9;
        }

        if let Ok((id, result)) = rx_lyrics.try_recv() {
            if state.playing_id.as_ref() == Some(&id) {
                if let Ok(lrc) = result { state.parsed_lyrics = parse_lrc(&lrc); }
            }
        }

        // --- キー入力処理 ---
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                let now = std::time::Instant::now();
                match state.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('/') => state.input_mode = InputMode::Editing,
                        KeyCode::Up => { if state.current > 0 { state.current -= 1; state.list_state.select(Some(state.current)); } }
                        KeyCode::Down => {
                            let max = state.filtered_indices.len();
                            if state.current < max.saturating_sub(1) { state.current += 1; state.list_state.select(Some(state.current)); }
                        }
                        KeyCode::Left => {
                            let is_repeat = last_key.map_or(false, |(code, time)| {
                                code == KeyCode::Left && now.duration_since(time) < Duration::from_millis(200)
                            });
                            if is_repeat {
                                player::seek_relative(-5.0);
                                state.last_action = "⏪".into();
                            } else {
                                if state.current > 0 {
                                    state.current -= 1;
                                    state.list_state.select(Some(state.current));
                                    state.last_action = "⏮".into();
                                    play_selected_track(&mut state, tx_lyrics.clone(), tx_player_err.clone());
                                }
                            }
                            last_key = Some((KeyCode::Left, now));
                        }
                        KeyCode::Right => {
                            let is_repeat = last_key.map_or(false, |(code, time)| {
                                code == KeyCode::Right && now.duration_since(time) < Duration::from_millis(200)
                            });
                            if is_repeat {
                                player::seek_relative(5.0);
                                state.last_action = "⏩".into();
                            } else {
                                let max = state.filtered_indices.len();
                                if state.current < max.saturating_sub(1) {
                                    state.current += 1;
                                    state.list_state.select(Some(state.current));
                                    state.last_action = "⏭".into();
                                    play_selected_track(&mut state, tx_lyrics.clone(), tx_player_err.clone());
                                }
                            }
                            last_key = Some((KeyCode::Right, now));
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if key.code == KeyCode::Char(' ') && state.playing_id.is_some() {
                                state.is_paused = player::toggle_pause();
                                state.last_action = if state.is_paused { "⏸".into() } else { "▶".into() };
                            } else {
                                state.last_action = "▶".into();
                                play_selected_track(&mut state, tx_lyrics.clone(), tx_player_err.clone());
                            }
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Char(c) => { state.search.push(c); state.update_search(); }
                        KeyCode::Backspace => { state.search.pop(); state.update_search(); }
                        KeyCode::Esc | KeyCode::Enter => state.input_mode = InputMode::Normal,
                        _ => {}
                    },
                }
            }
        }

        state.playback_pos = player::get_position();
        update_current_lyric(&mut state);

        if last_tick.elapsed() >= Duration::from_millis(100) {
            state.tick_count += 1;
            last_tick = std::time::Instant::now();
        }
    }

    restore_terminal();
    Ok(())
}

fn play_selected_track(state: &mut AppState, tx_lyrics: tokio::sync::mpsc::Sender<(String, Result<String>)>, tx_err: tokio::sync::mpsc::Sender<String>) {
    let (id, title, url) = if let Some(t) = state.current_track() {
        let id = AppState::id_from_path(&t.path);
        (id.clone(), t.title.clone(), stream_url(&id))
    } else { return };

    state.error_msg = None;
    state.fetch_paused.store(true, Ordering::SeqCst);
    state.playing_id = Some(id.clone());
    state.status_msg = format!("Playing: {}", title);
    state.current_lyric = "Buffering...".into();
    state.parsed_lyrics.clear();
    state.is_paused = false;

    let _ = player::play_from_url_streaming(url, tx_err);

    let id_for_task = id.clone();
    let pause_signal = state.fetch_paused.clone();
    tokio::spawn(async move {
        let lrc_res = fetch_lyrics(&id_for_task).await;
        let _ = tx_lyrics.send((id_for_task, lrc_res)).await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        pause_signal.store(false, Ordering::SeqCst);
    });
}

fn update_current_lyric(state: &mut AppState) {
    if state.parsed_lyrics.is_empty() { return; }
    let mut line = "";
    for (time, text) in &state.parsed_lyrics {
        if state.playback_pos >= *time { line = text; } else { break; }
    }
    state.current_lyric = line.to_string();
}

fn parse_lrc(lrc: &str) -> Vec<(f64, String)> {
    let mut result = Vec::new();
    for line in lrc.lines() {
        if let Some(pos) = line.find(']') {
            let time_str = &line[1..pos];
            let parts: Vec<&str> = time_str.split(':').collect();
            if parts.len() == 2 {
                let m: f64 = parts[0].parse().unwrap_or(0.0);
                let s: f64 = parts[1].parse().unwrap_or(0.0);
                result.push((m * 60.0 + s, line[pos + 1..].trim().to_string()));
            }
        }
    }
    result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    result
}

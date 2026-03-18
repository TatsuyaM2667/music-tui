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
    // .env ファイルを読み込む
    let _ = dotenvy::dotenv();

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

    let (tx_track, mut rx_track) = tokio::sync::mpsc::channel::<TrackInfo>(100);
    let (tx_progress, mut rx_progress) = tokio::sync::mpsc::channel::<f64>(100);
    
    let pause_signal = state.fetch_paused.clone();
    tokio::spawn(async move {
        let _ = fetch_tracks_streaming(tx_track, tx_progress, pause_signal).await;
    });

    let (tx_lyrics, mut rx_lyrics) = tokio::sync::mpsc::channel::<(String, Result<String>)>(10);
    let (tx_player_status, mut rx_player_status) = tokio::sync::mpsc::channel::<String>(10);
    let (tx_album_art, mut rx_album_art) = tokio::sync::mpsc::channel::<Vec<u8>>(10);

    let mut last_tick = std::time::Instant::now();
    let mut last_key: Option<(KeyCode, std::time::Instant)> = None;
    
    loop {
        terminal.draw(|f| ui::draw_ui(f, &state))?;

        while let Ok(msg) = rx_player_status.try_recv() {
            if msg == "Playing" {
                state.is_actually_playing = true;
                let title = state.tracks.iter().find(|t| Some(&t.path) == state.playing_id.as_ref())
                    .map(|t| t.title.clone()).unwrap_or_default();
                state.status_msg = format!("Playing: {}", title);
                if state.parsed_lyrics.is_empty() { state.current_lyric = "● Playing...".into(); }
            } else if msg.contains("Error") {
                state.error_msg = Some(msg.clone());
                state.current_lyric = format!("❌ {}", msg);
                state.is_actually_playing = false;
            } else {
                state.status_msg = msg.clone();
                state.current_lyric = format!(">> {}", msg);
            }
        }

        while let Ok(art_data) = rx_album_art.try_recv() {
            if let Ok(img) = image::load_from_memory(&art_data) {
                state.album_art = Some(img);
            }
        }

        while let Ok(p) = rx_progress.try_recv() { state.load_progress = p; }
        while let Ok(track) = rx_track.try_recv() {
            state.tracks.push(track);
            // サーバーからの順序を尊重
            state.update_search();
            if state.tracks.len() == 1 { state.list_state.select(Some(0)); }
            state.is_loading = state.load_progress < 99.9;
        }

        while let Ok((path, result)) = rx_lyrics.try_recv() {
            if state.playing_id.as_ref() == Some(&path) {
                match result {
                    Ok(lrc) => {
                        state.parsed_lyrics = parse_lrc(&lrc);
                        if state.parsed_lyrics.is_empty() { state.current_lyric = "(No time tags)".into(); }
                    }
                    Err(_) => { state.current_lyric = "(No lyrics found)".into(); }
                }
            }
        }

        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key) = event::read()? {
                let now = std::time::Instant::now();
                match state.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('v') => {
                            let video_info = state.current_track().and_then(|t| {
                                t.video.as_ref().map(|v| v.clone())
                            });

                            if let Some(v_path) = video_info {
                                let url = video_url_from_path(&v_path);
                                // 確実にオーディオを一時停止させる
                                player::pause();
                                state.is_paused = true;
                                state.last_action = "🎬".into();
                                
                                let mut cmd = std::process::Command::new("mpv");
                                cmd.arg("--ytdl=no");
                                cmd.arg("--force-window");
                                cmd.arg("--user-agent=Mozilla/5.0");
                                cmd.arg(url);
                                // 動画は最初から再生（長さやイントロが違う可能性があるため）
                                let _ = cmd.spawn();
                            }
                        }
                        KeyCode::Char('f') => {
                            state.toggle_favorite();
                            state.last_action = "⭐".into();
                        }
                        KeyCode::Char('F') => {
                            state.toggle_favorite_view();
                            state.last_action = if state.show_favorites_only { "⭐".into() } else { "☰".into() };
                        }
                        KeyCode::Char('q') => break,
                        KeyCode::Char('/') => state.input_mode = InputMode::Editing,
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            state.adjust_volume(0.05);
                            state.last_action = format!("Vol: {:.0}%", state.volume * 100.0);
                        }
                        KeyCode::Char('-') | KeyCode::Char('_') => {
                            state.adjust_volume(-0.05);
                            state.last_action = format!("Vol: {:.0}%", state.volume * 100.0);
                        }
                        KeyCode::Up => {
                            if key.modifiers.contains(event::KeyModifiers::ALT) {
                                state.move_track(true);
                                let tracks_copy = state.tracks.clone();
                                tokio::spawn(async move {
                                    let _ = update_track_order(&tracks_copy).await;
                                });
                            } else {
                                if state.current > 0 { state.current -= 1; state.list_state.select(Some(state.current)); }
                            }
                        }
                        KeyCode::Down => {
                            if key.modifiers.contains(event::KeyModifiers::ALT) {
                                state.move_track(false);
                                let tracks_copy = state.tracks.clone();
                                tokio::spawn(async move {
                                    let _ = update_track_order(&tracks_copy).await;
                                });
                            } else {
                                if state.current < state.filtered_indices.len().saturating_sub(1) { state.current += 1; state.list_state.select(Some(state.current)); }
                            }
                        }
                        KeyCode::Left => {
                            let is_repeat = last_key.map_or(false, |(c, t)| c == KeyCode::Left && now.duration_since(t) < Duration::from_millis(200));
                            if is_repeat { player::seek_relative(-5.0); state.last_action = "⏪".into(); }
                            else if state.current > 0 { state.current -= 1; state.list_state.select(Some(state.current)); state.last_action = "⏮".into(); play_selected_track(&mut state, tx_lyrics.clone(), tx_player_status.clone(), tx_album_art.clone()); }
                            last_key = Some((KeyCode::Left, now));
                        }
                        KeyCode::Right => {
                            let is_repeat = last_key.map_or(false, |(c, t)| c == KeyCode::Right && now.duration_since(t) < Duration::from_millis(200));
                            if is_repeat { player::seek_relative(5.0); state.last_action = "⏩".into(); }
                            else if state.current < state.filtered_indices.len().saturating_sub(1) { state.current += 1; state.list_state.select(Some(state.current)); state.last_action = "⏭".into(); play_selected_track(&mut state, tx_lyrics.clone(), tx_player_status.clone(), tx_album_art.clone()); }
                            last_key = Some((KeyCode::Right, now));
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            if key.code == KeyCode::Char(' ') && state.playing_id.is_some() {
                                state.is_paused = player::toggle_pause();
                                state.last_action = if state.is_paused { "⏸".into() } else { "▶".into() };
                            } else {
                                state.last_action = "▶".into();
                                play_selected_track(&mut state, tx_lyrics.clone(), tx_player_status.clone(), tx_album_art.clone());
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

        if state.playing_id.is_some() && !state.is_paused && state.is_actually_playing {
            let reached_end = player::is_finished();
            let duration = state.tracks.iter().find(|t| Some(&t.path) == state.playing_id.as_ref()).map(|t| t.duration).unwrap_or(0.0);
            let is_near_end = state.playback_pos >= duration - 1.0 && duration > 0.0;

            if reached_end || is_near_end {
                let current_playing_idx = state.filtered_indices.iter().position(|&idx| Some(&state.tracks[idx].path) == state.playing_id.as_ref());
                if let Some(idx_in_filtered) = current_playing_idx {
                    if idx_in_filtered < state.filtered_indices.len() - 1 {
                        state.current = idx_in_filtered + 1;
                        state.list_state.select(Some(state.current));
                        state.last_action = "⏭".into();
                        play_selected_track(&mut state, tx_lyrics.clone(), tx_player_status.clone(), tx_album_art.clone());
                    }
                }
            }
        }

        if last_tick.elapsed() >= Duration::from_millis(100) { state.tick_count += 1; last_tick = std::time::Instant::now(); }
    }
    restore_terminal();
    Ok(())
}

fn play_selected_track(
    state: &mut AppState, 
    tx_lyrics: tokio::sync::mpsc::Sender<(String, Result<String>)>, 
    tx_status: tokio::sync::mpsc::Sender<String>,
    tx_art: tokio::sync::mpsc::Sender<Vec<u8>>
) {
    let (path, lrc_path, _title) = if let Some(t) = state.current_track() {
        (t.path.clone(), t.lrc.clone(), t.title.clone())
    } else { return };

    state.error_msg = None;
    state.fetch_paused.store(true, Ordering::SeqCst);
    state.playing_id = Some(path.clone());
    state.status_msg = "Starting...".into();
    state.current_lyric = "Buffering...".into();
    state.parsed_lyrics.clear();
    state.is_paused = false;
    state.is_actually_playing = false;
    state.album_art = None; // Reset album art

    let url = stream_url_from_path(&path);
    let _ = player::play_from_url_streaming(url, tx_status, tx_art);

    if let Some(lp) = lrc_path {
        let tx = tx_lyrics.clone();
        let path_copy = path.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let res = fetch_lyrics_from_url(&lyrics_url_from_path(&lp)).await;
            let _ = tx.send((path_copy, res)).await;
        });
    }

    let pause_signal = state.fetch_paused.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
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

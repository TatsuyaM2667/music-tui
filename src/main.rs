mod api;
mod state;
mod player;
mod ui;

use api::*;
use state::*;
use std::io::stdout;
use std::time::Duration;

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
    // ... rest of main ...

    // パニック時（エラー落ち）でもターミナルを元の状態に戻す
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    let mut state = AppState::new(vec![]);
    let mut out = stdout();

    // 初期化: マウスキャプチャを明示的にOFFにするコマンドを送信
    execute!(out, EnterAlternateScreen, Clear(ClearType::All), Hide, DisableMouseCapture)?;
    terminal::enable_raw_mode()?;

    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    // バックグラウンドで曲情報を取得
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        let _ = tx.send(fetch_tracks().await).await;
    });

    let mut last_tick = std::time::Instant::now();
    
    // メインループ
    loop {
        terminal.draw(|f| ui::draw_ui(f, &state))?;

        // データの受信
        while let Ok(result) = rx.try_recv() {
            state.is_loading = false;
            match result {
                Ok(mut t) => {
                    // ここで再ソートを徹底
                    t.sort_by(|a, b| {
                        a.artist.to_lowercase().cmp(&b.artist.to_lowercase())
                            .then(a.album.to_lowercase().cmp(&b.album.to_lowercase()))
                            .then(a.title.to_lowercase().cmp(&b.title.to_lowercase()))
                    });
                    state.tracks = t;
                    state.update_search();
                    state.list_state.select(Some(0));
                    state.status_msg = "Ready.".into();
                }
                Err(e) => {
                    state.error_msg = Some(format!("API Error: {}", e));
                }
            }
        }

        // 2. キーボードイベントの処理 (50ms待機)
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match state.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('/') => {
                            state.input_mode = InputMode::Editing;
                        }
                        KeyCode::Up => {
                            if state.current > 0 { 
                                state.current -= 1; 
                                state.list_state.select(Some(state.current));
                            }
                        }
                        KeyCode::Down => {
                            let max = if state.search.is_empty() { state.tracks.len() } else { state.filtered_indices.len() };
                            if state.current < max - 1 { 
                                state.current += 1; 
                                state.list_state.select(Some(state.current));
                            }
                        }
                        KeyCode::Left => {
                            if state.current > 0 { 
                                state.current -= 1; 
                                state.list_state.select(Some(state.current));
                                state.last_action = "⏮".into();
                                play_selected_track(&mut state).await;
                            }
                        }
                        KeyCode::Right => {
                            let max = if state.search.is_empty() { state.tracks.len() } else { state.filtered_indices.len() };
                            if state.current < max - 1 { 
                                state.current += 1; 
                                state.list_state.select(Some(state.current));
                                state.last_action = "⏭".into();
                                play_selected_track(&mut state).await;
                            }
                        }
                        KeyCode::Enter => {
                            state.last_action = "▶".into();
                            play_selected_track(&mut state).await;
                        }
                        KeyCode::Char(' ') => {
                            state.last_action = if state.is_paused { "▶".into() } else { "⏸".into() };
                            play_selected_track(&mut state).await;
                        }
                        _ => {}
                    },
                    InputMode::Editing => match key.code {
                        KeyCode::Char(c) => {
                            state.search.push(c);
                            state.update_search();
                            state.current = 0; // 検索時は先頭に戻す
                            state.list_state.select(Some(0));
                        }
                        KeyCode::Backspace => {
                            state.search.pop();
                            state.update_search();
                            state.list_state.select(Some(0));
                        }
                        KeyCode::Esc | KeyCode::Enter => {
                            state.input_mode = InputMode::Normal;
                        }
                        _ => {}
                    },
                }
            }
        }
// 3. 状態の更新
state.playback_pos = player::get_position();

// カラオケ表示の更新
if !state.parsed_lyrics.is_empty() {
    let mut current_line = "";
    for (time, text) in &state.parsed_lyrics {
        if state.playback_pos >= *time {
            current_line = text;
        } else {
            break;
        }
    }
    state.current_lyric = current_line.to_string();
}

if last_tick.elapsed() >= Duration::from_millis(100) {
    state.tick_count += 1;
    last_tick = std::time::Instant::now();
}
}

// 正常終了時の復旧
restore_terminal();

Ok(())
}

async fn play_selected_track(state: &mut AppState) {
    let selected = state.current_track();
    if selected.is_none() { return; }
    
    // 必要な情報を先にコピーして、借用を解除する
    let (id, title, url) = {
        let track = selected.unwrap(); // ここではis_noneチェック済みなので安全
        let id = AppState::id_from_path(&track.path);
        (id.clone(), track.title.clone(), stream_url(&id))
    };

    // 同じ曲が選択されている場合は一時停止/再開
    if state.playing_id.as_ref() == Some(&id) {
        state.is_paused = player::toggle_pause();
        state.status_msg = if state.is_paused { "Paused".into() } else { format!("Playing: {}", title) };
        return;
    }

    // 違う曲、または初回再生の場合
    state.status_msg = format!("Loading: {}...", title);
    state.current_lyric = "(Streaming...)".into();
    state.parsed_lyrics.clear();
    state.playing_id = Some(id.clone());
    state.is_paused = false;

    // --- ストリーミング再生開始 (非同期) ---
    // ここで .await しないのが重要！
    if let Err(e) = player::play_from_url_streaming(url) {
        state.error_msg = Some(format!("Player Error: {}", e));
        return;
    }

    // 歌詞取得をバックグラウンドで行う
    let _id_copy = id.clone();
    tokio::spawn(async move {
        // ... (省略)
    });
    
    // 歌詞取得 (簡易的にここで行うが、本当はこれも spawn すべき)
    if let Ok(lrc) = fetch_lyrics(&id).await {
        state.parsed_lyrics = parse_lrc(&lrc);
        if state.parsed_lyrics.is_empty() {
            state.current_lyric = "(No time-synced lyrics)".into();
        }
    }
}

// LRC [mm:ss.xx] 形式のパース
fn parse_lrc(lrc: &str) -> Vec<(f64, String)> {
let mut result = Vec::new();
for line in lrc.lines() {
if let Some(pos) = line.find(']') {
    let time_str = &line[1..pos]; // "00:12.34"
    let content = &line[pos + 1..];

    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() == 2 {
        let min: f64 = parts[0].parse().unwrap_or(0.0);
        let sec: f64 = parts[1].parse().unwrap_or(0.0);
        let total_sec = min * 60.0 + sec;
        result.push((total_sec, content.trim().to_string()));
    }
}
}
result.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
result
}


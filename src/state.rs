use crate::api::TrackInfo;
use ratatui::widgets::ListState;
use ratatui::layout::Rect;
use std::collections::HashSet;
use std::fs;
use std::time::Instant;
use image::DynamicImage;
use ratatui_image::picker::Picker;
use souvlaki::{MediaControls, MediaControlEvent};
use tokio::sync::mpsc;

pub enum InputMode {
    Normal,
    Editing,
}

const FAV_FILE: &str = "favorites.json";

pub struct AppState {
    pub tracks: Vec<TrackInfo>,
    pub filtered_indices: Vec<usize>,
    pub current: usize,
    pub list_state: ListState,
    pub input_mode: InputMode,
    pub current_lyric: String,
    pub parsed_lyrics: Vec<(f64, String)>,
    pub lyric_area: Option<Rect>,
    pub lyric_scroll_offset: i32,
    pub last_lyric_interaction: Instant,
    pub prev_button_area: Option<Rect>,
    pub play_button_area: Option<Rect>,
    pub next_button_area: Option<Rect>,
    pub search: String,
    pub is_loading: bool,
    pub load_progress: f64,
    pub fetch_paused: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub error_msg: Option<String>,
    pub status_msg: String,
    pub last_action: String,
    pub tick_count: u64,
    pub playback_pos: f64,
    pub playing_id: Option<String>,
    pub is_paused: bool,
    pub is_actually_playing: bool,
    pub favorites: HashSet<String>,
    pub show_favorites_only: bool,
    pub volume: f32,
    pub album_art: Option<DynamicImage>,
    pub art_temp_path: Option<String>,
    pub picker: Option<Picker>,
    // Media controls for OS integration
    pub media_controls: Option<MediaControls>,
    pub rx_media_events: mpsc::Receiver<MediaControlEvent>,
}

impl AppState {
    pub fn new(tracks: Vec<TrackInfo>) -> Self {
        let favorites = Self::load_favorites().unwrap_or_default();
        let filtered_indices = (0..tracks.len()).collect();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        let picker = Picker::from_query_stdio().ok();
        let (tx, rx) = mpsc::channel(32);

        #[cfg(target_os = "linux")]
        let media_controls = {
            use souvlaki::PlatformConfig;
            let config = PlatformConfig {
                dbus_name: "org.mpris.MediaPlayer2.music_tui",
                display_name: "Music TUI",
                hwnd: None,
            };
            if let Ok(mut mc) = MediaControls::new(config) {
                let tx_clone = tx.clone();
                let _ = mc.attach(move |event| {
                    let _ = tx_clone.blocking_send(event);
                });
                Some(mc)
            } else {
                None
            }
        };
        #[cfg(not(target_os = "linux"))]
        let media_controls = None;

        Self {
            tracks,
            filtered_indices,
            current: 0,
            list_state,
            input_mode: InputMode::Normal,
            current_lyric: "".into(),
            parsed_lyrics: vec![],
            lyric_area: None,
            lyric_scroll_offset: 0,
            last_lyric_interaction: Instant::now(),
            prev_button_area: None,
            play_button_area: None,
            next_button_area: None,
            search: "".into(),
            is_loading: true,
            load_progress: 0.0,
            fetch_paused: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            error_msg: None,
            status_msg: "Waiting for tracks...".into(),
            last_action: "■".into(),
            tick_count: 0,
            playback_pos: 0.0,
            playing_id: None,
            is_paused: false,
            is_actually_playing: false,
            favorites,
            show_favorites_only: false,
            volume: 1.0,
            album_art: None,
            art_temp_path: None,
            picker,
            media_controls,
            rx_media_events: rx,
        }
    }

    fn load_favorites() -> Option<HashSet<String>> {
        if let Ok(data) = fs::read_to_string(FAV_FILE) {
            serde_json::from_str(&data).ok()
        } else {
            None
        }
    }

    fn save_favorites(&self) {
        if let Ok(data) = serde_json::to_string(&self.favorites) {
            let _ = fs::write(FAV_FILE, data);
        }
    }

    pub fn current_track(&self) -> Option<&TrackInfo> {
        if self.tracks.is_empty() || self.filtered_indices.is_empty() { return None; }
        let idx = if self.current >= self.filtered_indices.len() { self.filtered_indices[0] } else { self.filtered_indices[self.current] };
        Some(&self.tracks[idx])
    }

    pub fn update_search(&mut self) {
        let search_lower = self.search.to_lowercase();
        self.filtered_indices = self.tracks.iter().enumerate().filter(|(_, t)| {
            let matches_search = t.title.to_lowercase().contains(&search_lower) || 
                               t.artist.to_lowercase().contains(&search_lower) ||
                               t.album.to_lowercase().contains(&search_lower);
            let matches_favorite = if self.show_favorites_only {
                self.favorites.contains(&t.path)
            } else {
                true
            };
            matches_search && matches_favorite
        }).map(|(i, _)| i).collect();
        if self.current >= self.filtered_indices.len() {
            self.current = if self.filtered_indices.is_empty() { 0 } else { self.filtered_indices.len() - 1 };
        }
        self.list_state.select(Some(self.current));
    }

    pub fn toggle_favorite(&mut self) {
        if let Some(track) = self.current_track() {
            let path = track.path.clone();
            if self.favorites.contains(&path) {
                self.favorites.remove(&path);
            } else {
                self.favorites.insert(path);
            }
            self.save_favorites();
            if self.show_favorites_only {
                self.update_search();
            }
        }
    }

    pub fn toggle_favorite_view(&mut self) {
        self.show_favorites_only = !self.show_favorites_only;
        self.update_search();
    }

    pub fn adjust_volume(&mut self, delta: f32) {
        self.volume = (self.volume + delta).clamp(0.0, 1.0);
        crate::player::set_volume(self.volume);
    }

    pub fn move_track(&mut self, up: bool) {
        if self.filtered_indices.len() < 2 { return; }
        if self.current >= self.filtered_indices.len() { return; }
        let target_idx = if up {
            if self.current == 0 { return; }
            self.current - 1
        } else {
            if self.current >= self.filtered_indices.len() - 1 { return; }
            self.current + 1
        };
        let actual_idx = self.filtered_indices[self.current];
        let actual_target_idx = self.filtered_indices[target_idx];
        self.tracks.swap(actual_idx, actual_target_idx);
        self.filtered_indices[self.current] = actual_target_idx;
        self.filtered_indices[target_idx] = actual_idx;
        self.current = target_idx;
        self.list_state.select(Some(self.current));
        self.last_action = format!("Moved {}", if up { "Up" } else { "Down" });
    }
}

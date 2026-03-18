use crate::api::TrackInfo;
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::fs;
use image::DynamicImage;
use ratatui_image::picker::Picker;

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
    pub picker: Option<Picker>,
}

impl AppState {
    pub fn new(tracks: Vec<TrackInfo>) -> Self {
        // お気に入りをファイルから読み込む
        let favorites = Self::load_favorites().unwrap_or_default();

        let filtered_indices = (0..tracks.len()).collect();
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        
        // Pickerの初期化を試みる
        let picker = Picker::from_query_stdio().ok();

        Self {
            tracks,
            filtered_indices,
            current: 0,
            list_state,
            input_mode: InputMode::Normal,
            current_lyric: "".into(),
            parsed_lyrics: vec![],
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
            picker,
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
            // 保存
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

        // tracks における実際のインデックスを取得
        let actual_idx = self.filtered_indices[self.current];
        let actual_target_idx = self.filtered_indices[target_idx];

        // tracks 内で入れ替え
        self.tracks.swap(actual_idx, actual_target_idx);
        
        // filtered_indices を更新（単に入れ替える）
        self.filtered_indices[self.current] = actual_target_idx;
        self.filtered_indices[target_idx] = actual_idx;

        self.current = target_idx;
        self.list_state.select(Some(self.current));
        self.last_action = format!("Moved {}", if up { "Up" } else { "Down" });
    }
}

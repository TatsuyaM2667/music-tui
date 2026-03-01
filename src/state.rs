use crate::api::TrackInfo;
use ratatui::widgets::ListState;

pub enum InputMode {
    Normal,
    Editing,
}

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
    pub error_msg: Option<String>,
    pub status_msg: String,
    pub last_action: String, // 追加: 再生アイコン
    pub tick_count: u64,
    pub playback_pos: f64,
    pub playing_id: Option<String>,
    pub is_paused: bool,
}

impl AppState {
    pub fn new(mut tracks: Vec<TrackInfo>) -> Self {
        tracks.sort_by(|a, b| {
            a.artist.to_lowercase().cmp(&b.artist.to_lowercase())
                .then(a.album.to_lowercase().cmp(&b.album.to_lowercase()))
                .then(a.title.to_lowercase().cmp(&b.title.to_lowercase()))
        });

        let filtered_indices = (0..tracks.len()).collect();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

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
            error_msg: None,
            status_msg: "Waiting for tracks...".into(),
            last_action: "■".into(),
            tick_count: 0,
            playback_pos: 0.0,
            playing_id: None,
            is_paused: false,
        }
    }

    pub fn current_track(&self) -> Option<&TrackInfo> {
        if self.tracks.is_empty() || self.filtered_indices.is_empty() {
            return None;
        }
        
        let idx = if self.current >= self.filtered_indices.len() {
            self.filtered_indices[0]
        } else {
            self.filtered_indices[self.current]
        };
        
        Some(&self.tracks[idx])
    }

    pub fn update_search(&mut self) {
        let search_lower = self.search.to_lowercase();
        
        // 検索結果のインデックスを更新
        self.filtered_indices = self.tracks
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                t.title.to_lowercase().contains(&search_lower) || 
                t.artist.to_lowercase().contains(&search_lower) ||
                t.album.to_lowercase().contains(&search_lower)
            })
            .map(|(i, _)| i)
            .collect();
        
        // 選択位置が範囲外にならないように調整
        if self.current >= self.filtered_indices.len() {
            self.current = if self.filtered_indices.is_empty() { 0 } else { self.filtered_indices.len() - 1 };
        }
        self.list_state.select(Some(self.current));
    }

    pub fn id_from_path(path: &str) -> String {
        path.trim_start_matches('/')
            .replace(".mp3", "")
    }
}

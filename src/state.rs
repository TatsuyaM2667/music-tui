use crate::api::TrackInfo;

pub struct AppState {
    pub tracks: Vec<TrackInfo>,
    pub current: usize,
    pub current_lyric: String, // 現在表示中の1行
    pub parsed_lyrics: Vec<(f64, String)>, // 解析済みの歌詞リスト
    pub search: String,
    pub is_loading: bool,
    pub error_msg: Option<String>,
    pub status_msg: String,
    pub tick_count: u64,
    pub playback_pos: f64,
    pub playing_id: Option<String>, // 現在再生中の曲のID
    pub is_paused: bool,
}

impl AppState {
    pub fn new(tracks: Vec<TrackInfo>) -> Self {
        Self {
            tracks,
            current: 0,
            current_lyric: "".into(),
            parsed_lyrics: vec![],
            search: "".into(),
            is_loading: true,
            error_msg: None,
            status_msg: "Welcome!".into(),
            tick_count: 0,
            playback_pos: 0.0,
            playing_id: None,
            is_paused: false,
        }
    }
    // ... rest of methods ...

    pub fn current_track(&self) -> &TrackInfo {
        &self.tracks[self.current]
    }

    pub fn id_from_path(path: &str) -> String {
        path.trim_start_matches('/')
            .replace(".mp3", "")
    }
}

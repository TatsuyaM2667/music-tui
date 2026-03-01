use crate::api::TrackInfo;

pub struct AppState {
    pub tracks: Vec<TrackInfo>,
    pub current: usize,
    pub current_lyric: String,
    pub search: String,
}

impl AppState {
    pub fn new(tracks: Vec<TrackInfo>) -> Self {
        Self {
            tracks,
            current: 0,
            current_lyric: "".into(),
            search: "".into(),
        }
    }

    pub fn current_track(&self) -> &TrackInfo {
        &self.tracks[self.current]
    }

    pub fn id_from_path(path: &str) -> String {
        path.trim_start_matches('/')
            .replace(".mp3", "")
    }
}

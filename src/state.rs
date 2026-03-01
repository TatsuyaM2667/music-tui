use crate::api::Track;

pub struct AppState {
    pub tracks: Vec<Track>,
    pub current_index: usize,
    pub current_lyric: String,
    pub search_query: String,
}

impl AppState {
    pub fn new(tracks: Vec<Track>) -> Self {
        Self {
            tracks,
            current_index: 0,
            current_lyric: "".into(),
            search_query: "".into(),
        }
    }

    pub fn current_track(&self) -> &Track {
        &self.tracks[self.current_index]
    }
}

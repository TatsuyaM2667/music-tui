use serde::Deserialize;
use anyhow::Result;

#[derive(Debug, Deserialize, Clone)]
pub struct TrackInfo {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,

    // JSON の duration は f64 なので合わせる
    pub duration: f64,

    // null が来る可能性があるので Option にする
    pub lrc: Option<String>,
    pub date: Option<f64>,
    pub video: Option<String>,
    pub artistImage: Option<String>,
    pub cover: Option<CoverInfo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CoverInfo {
    pub format: String,
    pub data: String,
}

const BASE_URL: &str = "https://music-api.miuranosuketatsuya06.workers.dev";

pub async fn fetch_tracks() -> Result<Vec<TrackInfo>> {
    let url = format!("{}/tracks", BASE_URL);
    let res = reqwest::get(url).await?;
    let tracks = res.json::<Vec<TrackInfo>>().await?;
    Ok(tracks)
}

pub fn stream_url(id: &str) -> String {
    format!("{}/stream/{}", BASE_URL, id)
}

pub async fn fetch_lyrics(id: &str) -> Result<String> {
    let url = format!("{}/lyrics/{}", BASE_URL, id);
    let res = reqwest::get(url).await?;
    Ok(res.text().await?)
}

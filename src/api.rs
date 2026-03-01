use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
}

pub async fn fetch_tracks() -> anyhow::Result<Vec<Track>> {
    let url = "https://your-workers-domain/tracks";
    let res = reqwest::get(url).await?;
    let tracks = res.json::<Vec<Track>>().await?;
    Ok(tracks)
}

pub async fn fetch_lyrics(id: &str) -> anyhow::Result<String> {
    let url = format!("https://your-workers-domain/tracks/{}/lyrics", id);
    let res = reqwest::get(&url).await?;
    Ok(res.text().await?)
}

pub async fn fetch_stream_url(id: &str) -> String {
    format!("https://your-workers-domain/tracks/{}/stream", id)
}

use serde::Deserialize;
use anyhow::Result;
use futures_util::StreamExt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::io::StreamReader;

#[derive(Debug, Deserialize, Clone)]
pub struct TrackInfo {
    pub path: String,
    pub lrc: Option<String>,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub track_number: Option<i32>, // 追加
    pub duration: f64,
}

const BASE_URL: &str = "https://music-api.miuranosuketatsuya06.workers.dev";

pub async fn fetch_tracks_streaming(
    tx_track: tokio::sync::mpsc::Sender<TrackInfo>,
    tx_progress: tokio::sync::mpsc::Sender<f64>,
    pause_signal: Arc<AtomicBool>
) -> Result<()> {
    let url = format!("{}/tracks", BASE_URL);
    let res = reqwest::get(url).await?;
    let total_size = res.content_length().unwrap_or(1);
    
    let mut downloaded: u64 = 0;
    let stream = res.bytes_stream().map(|result| {
        result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });
    let reader = StreamReader::new(stream);
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        while pause_signal.load(Ordering::SeqCst) {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        downloaded += line.len() as u64 + 1;
        if let Ok(track) = serde_json::from_str::<TrackInfo>(&line) {
            let _ = tx_track.send(track).await;
        }
        let progress = (downloaded as f64 / total_size as f64) * 100.0;
        let _ = tx_progress.send(progress).await;
    }
    Ok(())
}

pub fn stream_url_from_path(path: &str) -> String {
    // パス全体をエンコードする。スラッシュもエンコードされるが、
    // Worker側の decodeURIComponent で元に戻るため問題なし
    format!("{}/stream/{}", BASE_URL, urlencoding::encode(path))
}

pub fn lyrics_url_from_path(path: &str) -> String {
    format!("{}/lyrics/{}", BASE_URL, urlencoding::encode(path))
}

pub async fn fetch_lyrics_from_url(url: &str) -> Result<String> {
    let res = reqwest::get(url).await?;
    Ok(res.text().await?)
}

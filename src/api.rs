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
    pub title: String,
    pub artist: String,
    pub album: String,
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
    // reqwest の Response を AsyncRead に変換
    let stream = res.bytes_stream().map(|result| {
        result.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    });
    let reader = StreamReader::new(stream);
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await? {
        // 優先タスクがある場合は待機（一時停止）
        while pause_signal.load(Ordering::SeqCst) {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        downloaded += line.len() as u64 + 1; // +1 for newline
        
        if let Ok(track) = serde_json::from_str::<TrackInfo>(&line) {
            let _ = tx_track.send(track).await;
        }
        
        let progress = (downloaded as f64 / total_size as f64) * 100.0;
        let _ = tx_progress.send(progress).await;
    }

    Ok(())
}

pub fn stream_url(id: &str) -> String {
    format!("{}/stream/{}", BASE_URL, id)
}

pub async fn fetch_lyrics(id: &str) -> Result<String> {
    let url = format!("{}/lyrics/{}", BASE_URL, id);
    let res = reqwest::get(url).await?;
    Ok(res.text().await?)
}

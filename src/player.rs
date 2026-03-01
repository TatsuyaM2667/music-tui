use rodio::{Decoder, OutputStream, Sink};
use anyhow::Result;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

// 再生状態をグローバルに保持 (Sink は Send なので Mutex で守れば Sync になる)
static PLAYER_SINK: Lazy<Arc<Mutex<Option<Sink>>>> = Lazy::new(|| {
    // OutputStream を一回だけ初期化し、Box::leak で永遠に維持する（Send を回避するテクニック）
    if let Ok((stream, handle)) = OutputStream::try_default() {
        // stream はドロップされると音が止まるので leak させてメモリ上に残す
        Box::leak(Box::new(stream));
        
        if let Ok(sink) = Sink::try_new(&handle) {
            return Arc::new(Mutex::new(Some(sink)));
        }
    }
    Arc::new(Mutex::new(None))
});

pub async fn play_from_url(url: &str) -> Result<()> {
    let bytes = reqwest::get(url).await?.bytes().await?;
    let cursor = Cursor::new(bytes);

    let player = PLAYER_SINK.lock().unwrap();
    if let Some(sink) = player.as_ref() {
        sink.clear(); // 既存の曲をクリア
        
        let source = Decoder::new(cursor).unwrap();
        sink.append(source);
        sink.play();
    }
    Ok(())
}

pub fn toggle_pause() -> bool {
    let player = PLAYER_SINK.lock().unwrap();
    if let Some(sink) = player.as_ref() {
        if sink.is_paused() {
            sink.play();
            return false;
        } else {
            sink.pause();
            return true;
        }
    }
    false
}

pub fn get_position() -> f64 {
    let player = PLAYER_SINK.lock().unwrap();
    if let Some(sink) = player.as_ref() {
        // get_pos() は Duration を返す
        return sink.get_pos().as_secs_f64();
    }
    0.0
}

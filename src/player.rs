use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use anyhow::{Result, anyhow};
use std::io::{Read, Seek, SeekFrom, Cursor};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

static AUDIO_HANDLE: Lazy<OutputStreamHandle> = Lazy::new(|| {
    let (stream, handle) = OutputStream::try_default().expect("Audio init failed");
    Box::leak(Box::new(stream));
    handle
});

static GLOBAL_SINK: Lazy<Mutex<Option<Arc<Sink>>>> = Lazy::new(|| Mutex::new(None));

struct StreamWrapper<R: Read> {
    inner: R,
    cache: Vec<u8>,
    pos: u64,
}

impl<R: Read> Read for StreamWrapper<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos < self.cache.len() as u64 {
            let mut cursor = Cursor::new(&self.cache);
            cursor.set_position(self.pos);
            let n = cursor.read(buf)?;
            self.pos += n as u64;
            Ok(n)
        } else {
            let n = self.inner.read(buf)?;
            if self.cache.len() < 10 * 1024 * 1024 {
                self.cache.extend_from_slice(&buf[..n]);
            }
            self.pos += n as u64;
            Ok(n)
        }
    }
}

impl<R: Read> Seek for StreamWrapper<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let target_pos = match pos {
            SeekFrom::Start(n) => n as i64,
            SeekFrom::Current(n) => self.pos as i64 + n,
            SeekFrom::End(_) => self.pos as i64, 
        };
        if target_pos < 0 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid seek")); }
        let target_pos = target_pos as u64;
        if target_pos <= self.cache.len() as u64 {
            self.pos = target_pos;
            Ok(self.pos)
        } else {
            let diff = target_pos - self.pos;
            let mut skip_buf = vec![0u8; diff.min(1024 * 1024) as usize];
            let _ = self.read(&mut skip_buf)?;
            Ok(self.pos)
        }
    }
}

// エラー報告用のチャンネルを引数に追加
pub fn play_from_url_streaming(url: String, tx_err: tokio::sync::mpsc::Sender<String>) -> Result<()> {
    stop();
    let sink = Arc::new(Sink::try_new(&AUDIO_HANDLE).map_err(|e| anyhow!(e))?);
    if let Ok(mut lock) = GLOBAL_SINK.lock() { *lock = Some(sink.clone()); }

    let sink_thread = sink.clone();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        
        // 1. 接続フェーズ
        let response = match client.get(&url).send() {
            Ok(res) => res,
            Err(e) => {
                let _ = tx_err.blocking_send(format!("接続エラー: {}", e));
                return;
            }
        };

        // 2. HTTPステータスチェック
        if !response.status().is_success() {
            let _ = tx_err.blocking_send(format!("HTTPエラー: {} (URL: {})", response.status(), url));
            return;
        }

        // 3. デコードフェーズ
        let stream = StreamWrapper { inner: response, cache: Vec::new(), pos: 0 };
        match Decoder::new(stream) {
            Ok(source) => {
                sink_thread.append(source);
                sink_thread.play();
            }
            Err(e) => {
                let _ = tx_err.blocking_send(format!("再生エラー（デコード失敗）: {:?}", e));
            }
        }
    });
    Ok(())
}

pub fn seek_relative(secs: f64) {
    if let Ok(lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() {
            let current = sink.get_pos();
            let new_pos = current.as_secs_f64() + secs;
            let _ = sink.try_seek(std::time::Duration::from_secs_f64(new_pos.max(0.0)));
        }
    }
}

pub fn stop() {
    if let Ok(mut lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() { sink.stop(); }
        *lock = None;
    }
}

pub fn toggle_pause() -> bool {
    if let Ok(lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() {
            if sink.is_paused() { sink.play(); return false; }
            else { sink.pause(); return true; }
        }
    }
    false
}

pub fn get_position() -> f64 {
    if let Ok(lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() { return sink.get_pos().as_secs_f64(); }
    }
    0.0
}

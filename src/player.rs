use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use anyhow::{Result, anyhow};
use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

static AUDIO_HANDLE: Lazy<OutputStreamHandle> = Lazy::new(|| {
    let (stream, handle) = OutputStream::try_default().expect("Audio init failed");
    Box::leak(Box::new(stream));
    handle
});

static GLOBAL_SINK: Lazy<Mutex<Option<Arc<Sink>>>> = Lazy::new(|| Mutex::new(None));

// 再生ズレを完全に解消するためのストリーミングラッパー
// 読んだデータをすべて Vec に貯めることで、デコーダの「先頭に戻る」要求に完璧に応える
struct StreamingBuffer<R: Read> {
    inner: R,
    data: Vec<u8>,
    pos: usize,
}

impl<R: Read> Read for StreamingBuffer<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos < self.data.len() {
            let n = (&self.data[self.pos..]).read(buf)?;
            self.pos += n;
            Ok(n)
        } else {
            let n = self.inner.read(buf)?;
            if n > 0 {
                self.data.extend_from_slice(&buf[..n]);
                self.pos += n;
            }
            Ok(n)
        }
    }
}

impl<R: Read> Seek for StreamingBuffer<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(s) => s as i64,
            SeekFrom::Current(c) => self.pos as i64 + c,
            SeekFrom::End(_) => return Ok(self.pos as u64),
        };
        if new_pos < 0 { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid seek")); }
        let new_pos = new_pos as usize;
        
        if new_pos <= self.data.len() {
            self.pos = new_pos;
            Ok(self.pos as u64)
        } else {
            let diff = new_pos - self.pos;
            let mut skip_buf = vec![0u8; diff.min(1024 * 1024)]; // 最大1MBずつ
            let _ = self.read(&mut skip_buf)?;
            Ok(self.pos as u64)
        }
    }
}

pub fn play_from_url_streaming(url: String, tx_err: tokio::sync::mpsc::Sender<String>) -> Result<()> {
    stop();
    let sink = Arc::new(Sink::try_new(&AUDIO_HANDLE).map_err(|e| anyhow!(e))?);
    if let Ok(mut lock) = GLOBAL_SINK.lock() { *lock = Some(sink.clone()); }

    let sink_thread = sink.clone();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build().unwrap();
        
        let _ = tx_err.blocking_send("Connecting...".into());
        let response = match client.get(&url).send() {
            Ok(res) => res,
            Err(e) => { let _ = tx_err.blocking_send(format!("Connection Error: {}", e)); return; }
        };

        if !response.status().is_success() {
            let _ = tx_err.blocking_send(format!("HTTP Error: {}", response.status()));
            return;
        }

        let _ = tx_err.blocking_send("Decoding...".into());
        let stream = StreamingBuffer { inner: response, data: Vec::with_capacity(512 * 1024), pos: 0 };
        
        match Decoder::new(stream) {
            Ok(source) => {
                sink_thread.append(source);
                sink_thread.play();
                let _ = tx_err.blocking_send("Playing".into());
            }
            Err(e) => { let _ = tx_err.blocking_send(format!("Decode Error: {:?}", e)); }
        }
    });
    Ok(())
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

pub fn is_finished() -> bool {
    if let Ok(lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() {
            // sink.empty() かつ 0.5秒以上経過している場合に終了とみなす
            return sink.empty() && sink.get_pos().as_secs_f64() > 0.5;
        }
    }
    false
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

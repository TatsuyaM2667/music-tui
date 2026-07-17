use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use anyhow::{Result, anyhow};
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

static AUDIO_HANDLE: Lazy<OutputStreamHandle> = Lazy::new(|| {
    let (stream, handle) = OutputStream::try_default().expect("Audio init failed");
    Box::leak(Box::new(stream));
    handle
});

static GLOBAL_SINK: Lazy<Mutex<Option<Arc<Sink>>>> = Lazy::new(|| Mutex::new(None));
static VOLUME: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(1.0));

// ストリーミングデータを全バッファリングしてからデコードするラッパー
// rodio の Decoder は Seek を要求するため、ダウンロード済みデータを cursor でラップする
struct FullyBuffered {
    cursor: std::io::Cursor<Vec<u8>>,
}

impl FullyBuffered {
    fn from_reader<R: Read>(mut reader: R) -> std::io::Result<Self> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        Ok(Self { cursor: std::io::Cursor::new(data) })
    }
}

impl Read for FullyBuffered {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl Seek for FullyBuffered {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(pos)
    }
}

pub fn play_from_url_streaming(
    url: String, 
    tx_err: tokio::sync::mpsc::Sender<String>,
    tx_art: tokio::sync::mpsc::Sender<Vec<u8>>
) -> Result<()> {
    stop();
    let sink = Arc::new(Sink::try_new(&AUDIO_HANDLE).map_err(|e| anyhow!(e))?);
    
    // Apply current volume
    if let Ok(vol) = VOLUME.lock() {
        sink.set_volume(*vol);
    }

    if let Ok(mut lock) = GLOBAL_SINK.lock() { *lock = Some(sink.clone()); }

    let sink_thread = sink.clone();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
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

        let _ = tx_err.blocking_send("Buffering...".into());

        // BufReader でラップしてから全バッファリング
        let buffered_reader = BufReader::new(response);
        let fully_buffered = match FullyBuffered::from_reader(buffered_reader) {
            Ok(b) => b,
            Err(e) => { let _ = tx_err.blocking_send(format!("Read Error: {}", e)); return; }
        };

        // Try to read ID3 tags for album art from the buffered data
        {
            let data = fully_buffered.cursor.get_ref();
            if data.len() >= 3 && &data[0..3] == b"ID3" {
                let mut cursor = std::io::Cursor::new(data.as_slice());
                if let Ok(tag) = id3::Tag::read_from2(&mut cursor) {
                    if let Some(pic) = tag.pictures().next() {
                        let _ = tx_art.blocking_send(pic.data.clone());
                    }
                }
            }
        }

        match Decoder::new(fully_buffered) {
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

pub fn pause() {
    if let Ok(lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() {
            sink.pause();
        }
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

pub fn seek_to(secs: f64) {
    if let Ok(lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() {
            let _ = sink.try_seek(std::time::Duration::from_secs_f64(secs.max(0.0)));
        }
    }
}

pub fn set_volume(vol: f32) {
    if let Ok(mut lock) = VOLUME.lock() {
        *lock = vol.clamp(0.0, 1.0);
        if let Ok(sink_lock) = GLOBAL_SINK.lock() {
            if let Some(sink) = sink_lock.as_ref() {
                sink.set_volume(*lock);
            }
        }
    }
}

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use anyhow::{Result, anyhow};
use std::io::{Read, Seek, SeekFrom, Cursor};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

// OutputStreamHandle は Send + Sync なので、これだけをグローバルに保持する
static AUDIO_HANDLE: Lazy<OutputStreamHandle> = Lazy::new(|| {
    let (stream, handle) = OutputStream::try_default().expect("音声デバイスの初期化に失敗しました。");
    // stream 本体がドロップされると音が止まるので、Box::leak でメモリに固定する
    Box::leak(Box::new(stream));
    handle
});

// 現在再生中の Sink を Arc で管理して、UIスレッドからも安全にアクセスできるようにする
static GLOBAL_SINK: Lazy<Mutex<Option<Arc<Sink>>>> = Lazy::new(|| Mutex::new(None));

// ストリーミングを「完全シーク可能」に見せるためのラッパー
// 最初の数MBをキャッシュすることで、デコーダの解析によるデータの欠落を防ぐ
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
            // 最初の2MBまでキャッシュ（解析とバッファリングに十分な量）
            if self.cache.len() < 2 * 1024 * 1024 {
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

        if target_pos < 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid seek"));
        }

        let target_pos = target_pos as u64;

        if target_pos <= self.cache.len() as u64 {
            self.pos = target_pos;
            Ok(self.pos)
        } else {
            // 前方へのシーク（未読領域）は読み飛ばしで対応
            let diff = target_pos - self.pos;
            let mut skip_buf = vec![0u8; diff as usize];
            let _ = self.read(&mut skip_buf)?;
            Ok(self.pos)
        }
    }
}

pub fn play_from_url_streaming(url: String) -> Result<()> {
    // 1. 既存の再生を停止
    stop();

    // 2. 新しい Sink を作成
    let sink = Arc::new(Sink::try_new(&AUDIO_HANDLE).map_err(|e| anyhow!(e))?);
    
    // 3. UIが即座に認識できるように先に登録
    if let Ok(mut lock) = GLOBAL_SINK.lock() {
        *lock = Some(sink.clone());
    }

    // 4. ストリーミングスレッド
    let sink_thread = sink.clone();
    std::thread::spawn(move || {
        let client = reqwest::blocking::Client::new();
        if let Ok(response) = client.get(url).send() {
            let stream = StreamWrapper { inner: response, cache: Vec::new(), pos: 0 };
            // Decoder::new が内部でシークを行っても StreamWrapper がキャッシュから正しく返す
            if let Ok(source) = Decoder::new(stream) {
                sink_thread.append(source);
                sink_thread.play();
            }
        }
    });

    Ok(())
}

pub fn stop() {
    if let Ok(mut lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() {
            sink.stop();
        }
        *lock = None;
    }
}

pub fn toggle_pause() -> bool {
    if let Ok(lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() {
            if sink.is_paused() {
                sink.play();
                return false;
            } else {
                sink.pause();
                return true;
            }
        }
    }
    false
}

pub fn get_position() -> f64 {
    if let Ok(lock) = GLOBAL_SINK.lock() {
        if let Some(sink) = lock.as_ref() {
            return sink.get_pos().as_secs_f64();
        }
    }
    0.0
}

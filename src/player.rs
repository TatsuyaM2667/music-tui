use rodio::{Decoder, OutputStream, Sink};
use anyhow::Result;
use std::io::Cursor;

pub async fn play_from_url(url: &str) -> Result<()> {
    // 音源を async で取得
    let bytes = reqwest::get(url).await?.bytes().await?;
    let cursor = Cursor::new(bytes);

    // rodio は Send ではないので専用スレッドで動かす
    std::thread::spawn(move || {
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&handle).unwrap();

        let source = Decoder::new(cursor).unwrap();
        sink.append(source);

        // detach でバックグラウンド再生
        sink.detach();
    });

    Ok(())
}

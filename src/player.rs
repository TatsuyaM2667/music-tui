use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

pub async fn play_from_url(url: &str) -> anyhow::Result<Sink> {
    let (_stream, handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&handle)?;

    let bytes = reqwest::get(url).await?.bytes().await?;
    let cursor = Cursor::new(bytes);
    let source = Decoder::new(cursor)?;

    sink.append(source);
    Ok(sink)
}

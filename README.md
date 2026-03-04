# music-tui

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-Linux-lightgrey.svg)](https://www.kernel.org/)
[![Arch Linux](https://img.shields.io/badge/Developed%20on-Arch%20Linux-blue.svg)](https://archlinux.org/)

`music-tui` は、Rustで書かれたターミナルベースの音楽プレイヤーです。ストリーミング再生、歌詞表示、お気に入り機能、動画連携などの機能を備えています。

> **Note**: 本プロジェクトは **Arch Linux** 環境で開発・テストされています。

## 特徴

- **TUI インターフェース**: `ratatui` を使用した、軽快で直感的なターミナル操作。
- **ストリーミング再生**: ネットワーク経由での音楽再生に対応。
- **歌詞表示**: LRC形式の歌詞をパースし、再生時間に合わせて表示。
- **検索・フィルタリング**: 曲名やアーティスト名でのリアルタイム検索。
- **お気に入り機能**: よく聴く曲をお気に入りに追加し、絞り込み表示が可能。
- **動画連携**: `mpv` を使用して、関連する動画を再生する機能。

## 必要条件

- **Rust**: コンパイルに必要です。
- **mpv**: 動画再生機能を利用する場合に必要です。
- **ネットワーク接続**: ストリーミング再生および歌詞の取得に必要です。

## インストールと実行

1. リポジトリをクローンします。
2. `.env.example` を参考に `.env` ファイルを作成し、必要な環境変数を設定します。
3. 以下のコマンドで実行します。

```bash
cargo run --release
```

## 操作方法

### ノーマルモード

| キー | アクション |
| :--- | :--- |
| `q` | 終了 |
| `/` | 検索モードへ移行 |
| `Up` / `Down` | 曲の選択移動 |
| `Enter` / `Space` | 再生 / 一時停止 |
| `Left` / `Right` | 前の曲・次の曲 (連打で5秒シーク) |
| `v` | mpvで動画を再生 |
| `f` | お気に入りの切り替え (現在の曲) |
| `F` | お気に入り表示の切り替え |

### 検索モード

- 文字入力: 検索クエリの入力。
- `Backspace`: 1文字削除。
- `Esc` / `Enter`: ノーマルモードに戻る。

## 技術スタック

- **言語**: Rust
- **UI ライブラリ**: [ratatui](https://github.com/ratatui/ratatui)
- **オーディオ再生**: [rodio](https://github.com/RustAudio/rodio)
- **非同期処理**: [tokio](https://github.com/tokio-rs/tokio)
- **HTTP クライアント**: [reqwest](https://github.com/seanmonstar/reqwest)

## ライセンス

[MIT License](LICENSE)

---

`#Rust` `#TUI` `#MusicPlayer` `#ArchLinux` `#Terminal` `#Ratatui` `#OpenSource`

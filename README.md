# Oto (音)

Audio framework for pleme-io applications. Shared audio primitives for music playback and voice communication.

## Components

| Module | Purpose |
|--------|---------|
| `player` | Transport controls: play, pause, stop, seek, volume |
| `queue` | Ordered playlist with repeat modes and gapless transitions |
| `decoder` | Multi-codec support via symphonia (FLAC, ALAC, WAV, MP3, AAC, OGG, Opus) |
| `device` | Output device enumeration and selection |
| `voice` | Real-time voice capture/playback for chat (mute, deafen) |

## Usage

```toml
[dependencies]
oto = { git = "https://github.com/pleme-io/oto" }
```

```rust
use oto::{Player, Queue, Decoder};

let mut player = Player::new();
let mut queue = Queue::new();
queue.add("song.flac".into());
player.play();
```

## Build

```bash
cargo build
cargo test --lib
```

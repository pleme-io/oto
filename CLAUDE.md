# Oto (音) — Audio Framework

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.


## Build & Test

```bash
cargo build
cargo test --lib
```

## Architecture

### Modules

| Module | Purpose |
|--------|---------|
| `player.rs` | `Player` — play/pause/stop/volume state machine |
| `queue.rs` | `Queue` — playlist with repeat (Off/All/One) and gapless |
| `decoder.rs` | `Decoder` — symphonia codec detection and support checking |
| `device.rs` | `AudioDevice` — rodio output device management |
| `voice.rs` | `VoiceStream` — real-time capture/playback, mute/deafen |

### Consumers

- **hibiki**: music playback (Player, Queue, Decoder)
- **fumi**: voice chat channels (VoiceStream)

## Design Decisions

- **rodio** for playback: mature, cross-platform, pure Rust
- **symphonia** for decoding: broad codec support, no ffmpeg dependency
- **Separate voice module**: voice streaming has different latency requirements than music playback
- **No async in Player/Queue**: state machines are synchronous; I/O is rodio's responsibility

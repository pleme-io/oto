# Oto (音) — Audio Framework

> **★★★ CSE / Knowable Construction.** This repo operates under **Constructive Substrate Engineering** — canonical specification at [`pleme-io/theory/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md`](https://github.com/pleme-io/theory/blob/main/CONSTRUCTIVE-SUBSTRATE-ENGINEERING.md). The Compounding Directive (operational rules: solve once, load-bearing fixes only, idiom-first, models stay current, direction beats velocity) is in the org-level pleme-io/CLAUDE.md ★★★ section. Read both before non-trivial changes.


## Build & Test

```bash
cargo build                           # pure-logic core (no audio stack)
cargo test  --lib
cargo build --features backend-rodio  # real rodio + symphonia backend
cargo test  --features backend-rodio
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
| `backend.rs` | `PlaybackBackend` (injectable audio-output/decode seam, peer of `AudioDeviceProvider`) + `MockBackend` + `Engine` (headless play/pause/seek/queue driver over any backend) |
| `rodio_backend.rs` | `RodioBackend` — real rodio + symphonia impl behind a dedicated audio OS thread; feature-gated `backend-rodio` |

### Consumers

- **hibiki**: music playback (Player, Queue, Decoder)
- **fumi**: voice chat channels (VoiceStream)

## Design Decisions

- **rodio** for playback: mature, cross-platform, pure Rust
- **symphonia** for decoding: broad codec support, no ffmpeg dependency
- **Separate voice module**: voice streaming has different latency requirements than music playback
- **No async in Player/Queue**: state machines are synchronous; I/O is rodio's responsibility
- **`PlaybackBackend` seam**: the actual audio output + decode is a trait (peer of `AudioDeviceProvider`); `Engine` wires the pure `Player` FSM + `QueueManager` to an injected backend, so consumers embed the typed core instead of re-rolling a rodio thread (Quadro P10). `MockBackend` drives it with zero real I/O — the testability contract.
- **Dedicated audio OS thread**: `RodioBackend` owns the `!Send` rodio `OutputStream` on a `std::thread` (never tokio), controlled over an mpsc channel with lock-free position/end-of-track reads — bounded-latency audio work stays off any async runtime.
- **`backend-rodio` feature-gated**: pure-logic consumers/tests build without the audio stack (cpal + symphonia).

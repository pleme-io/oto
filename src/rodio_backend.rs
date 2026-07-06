//! Real [`PlaybackBackend`] over `rodio` + `symphonia` (feature `backend-rodio`).
//!
//! A dedicated audio OS thread — a plain [`std::thread`], never tokio — owns
//! the `!Send` rodio `OutputStream` and the active `Sink`. [`RodioBackend`]
//! controls that thread over an [`std::sync::mpsc`] command channel and reads
//! playback position / end-of-track back through lock-free shared atomics.
//! Bounded-latency audio work therefore stays entirely off any async runtime,
//! per Quadro §II-T10 / pillar P10.
//!
//! Decoding is the pure-Rust symphonia path (FLAC / ALAC / WAV / AIFF / OGG /
//! MP3 / AAC / MP4) via rodio's `symphonia-*` features — no ffmpeg.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};

use crate::backend::PlaybackBackend;
use crate::error::OtoError;

const DEFAULT_VOLUME: f32 = 1.0;
/// Cadence at which the audio thread refreshes shared position / end-of-track
/// while idle (between commands).
const REFRESH_INTERVAL: Duration = Duration::from_millis(50);
/// Sentinel stored in `duration_ms` for "duration unknown".
const NO_DURATION: u64 = u64::MAX;

/// Lock-free state the audio thread publishes for the backend handle to read.
struct Shared {
    position_ms: AtomicU64,
    duration_ms: AtomicU64,
    ended: AtomicBool,
    has_track: AtomicBool,
}

impl Shared {
    fn new() -> Self {
        Self {
            position_ms: AtomicU64::new(0),
            duration_ms: AtomicU64::new(NO_DURATION),
            ended: AtomicBool::new(false),
            has_track: AtomicBool::new(false),
        }
    }
}

/// Commands sent from the [`RodioBackend`] handle to the audio thread.
enum Command {
    Load {
        path: PathBuf,
        reply: Sender<Result<Option<Duration>, OtoError>>,
    },
    Play,
    Pause,
    Stop,
    Seek(Duration),
    SetVolume(f32),
    Shutdown,
}

/// Real audio backend over rodio + symphonia.
///
/// Construct with [`RodioBackend::new`], which spawns the dedicated audio
/// thread and returns an error if no default output device is available.
/// Drop stops audio and joins the thread.
pub struct RodioBackend {
    tx: Sender<Command>,
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
    volume: f32,
}

impl RodioBackend {
    /// Spawn the audio thread and connect to the default output device.
    ///
    /// Returns [`OtoError::PlaybackFailed`] when no output device is available
    /// (e.g. a headless machine) — the caller can fall back to another backend.
    pub fn new() -> Result<Self, OtoError> {
        let (tx, rx) = mpsc::channel::<Command>();
        let (init_tx, init_rx) = mpsc::channel::<Result<(), OtoError>>();
        let shared = Arc::new(Shared::new());
        let thread_shared = Arc::clone(&shared);

        let handle = std::thread::Builder::new()
            .name("oto-audio".to_owned())
            .spawn(move || run_audio_thread(&rx, &thread_shared, &init_tx))
            .map_err(OtoError::Io)?;

        match init_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                tx,
                shared,
                handle: Some(handle),
                volume: DEFAULT_VOLUME,
            }),
            Ok(Err(e)) => {
                let _ = handle.join();
                Err(e)
            }
            Err(_) => {
                let _ = handle.join();
                Err(OtoError::PlaybackFailed(
                    "audio thread exited before initialization".to_owned(),
                ))
            }
        }
    }

    fn send(&self, cmd: Command) -> Result<(), OtoError> {
        self.tx
            .send(cmd)
            .map_err(|_| OtoError::PlaybackFailed("audio thread not running".to_owned()))
    }
}

impl Drop for RodioBackend {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl PlaybackBackend for RodioBackend {
    fn load(&mut self, path: &Path) -> Result<Option<Duration>, OtoError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.send(Command::Load {
            path: path.to_path_buf(),
            reply: reply_tx,
        })?;
        reply_rx
            .recv()
            .map_err(|_| OtoError::PlaybackFailed("audio thread dropped load reply".to_owned()))?
    }

    fn play(&mut self) -> Result<(), OtoError> {
        self.send(Command::Play)
    }

    fn pause(&mut self) -> Result<(), OtoError> {
        self.send(Command::Pause)
    }

    fn resume(&mut self) -> Result<(), OtoError> {
        self.send(Command::Play)
    }

    fn stop(&mut self) -> Result<(), OtoError> {
        self.send(Command::Stop)
    }

    fn seek(&mut self, position: Duration) -> Result<(), OtoError> {
        self.send(Command::Seek(position))
    }

    fn set_volume(&mut self, volume: f32) -> Result<(), OtoError> {
        let clamped = volume.clamp(0.0, 1.0);
        self.volume = clamped;
        self.send(Command::SetVolume(clamped))
    }

    fn volume(&self) -> f32 {
        self.volume
    }

    fn position(&self) -> Option<Duration> {
        if self.shared.has_track.load(Ordering::Relaxed) {
            Some(Duration::from_millis(
                self.shared.position_ms.load(Ordering::Relaxed),
            ))
        } else {
            None
        }
    }

    fn duration(&self) -> Option<Duration> {
        ms_to_duration(self.shared.duration_ms.load(Ordering::Relaxed))
    }

    fn is_ended(&self) -> bool {
        self.shared.ended.load(Ordering::Relaxed)
    }
}

/// The audio thread body. Owns the `!Send` `OutputStream` for its entire life
/// so all rodio interaction stays on this one non-async thread.
fn run_audio_thread(
    rx: &Receiver<Command>,
    shared: &Shared,
    init_tx: &Sender<Result<(), OtoError>>,
) {
    // The output stream is created on THIS thread; it is `!Send` and must both
    // stay here and be kept alive for the whole thread (dropping it stops
    // audio), hence the `_stream` binding rather than `_`.
    let (_stream, stream_handle) = match OutputStream::try_default() {
        Ok(pair) => pair,
        Err(e) => {
            let _ = init_tx.send(Err(OtoError::PlaybackFailed(e.to_string())));
            return;
        }
    };
    if init_tx.send(Ok(())).is_err() {
        return;
    }

    let mut sink: Option<Sink> = None;
    let mut volume = DEFAULT_VOLUME;

    loop {
        match rx.recv_timeout(REFRESH_INTERVAL) {
            Ok(Command::Load { path, reply }) => match open_sink(&stream_handle, &path, volume) {
                Ok((new_sink, duration)) => {
                    shared.position_ms.store(0, Ordering::Relaxed);
                    shared
                        .duration_ms
                        .store(duration_to_ms(duration), Ordering::Relaxed);
                    shared.ended.store(false, Ordering::Relaxed);
                    shared.has_track.store(true, Ordering::Relaxed);
                    sink = Some(new_sink);
                    let _ = reply.send(Ok(duration));
                }
                Err(e) => {
                    let _ = reply.send(Err(e));
                }
            },
            Ok(Command::Play) => {
                if let Some(s) = &sink {
                    s.play();
                }
            }
            Ok(Command::Pause) => {
                if let Some(s) = &sink {
                    s.pause();
                }
            }
            Ok(Command::Stop) => {
                sink = None;
                shared.has_track.store(false, Ordering::Relaxed);
                shared.ended.store(false, Ordering::Relaxed);
                shared.position_ms.store(0, Ordering::Relaxed);
                shared.duration_ms.store(NO_DURATION, Ordering::Relaxed);
            }
            Ok(Command::Seek(pos)) => {
                if let Some(s) = &sink {
                    let _ = s.try_seek(pos);
                }
            }
            Ok(Command::SetVolume(v)) => {
                volume = v;
                if let Some(s) = &sink {
                    s.set_volume(v);
                }
            }
            Ok(Command::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }

        // Publish live position + end-of-track after handling the event.
        if let Some(s) = &sink {
            shared.position_ms.store(
                u64::try_from(s.get_pos().as_millis()).unwrap_or(u64::MAX),
                Ordering::Relaxed,
            );
            if s.empty() {
                shared.ended.store(true, Ordering::Relaxed);
            }
        }
    }
    // `_stream` and `sink` drop here, stopping audio cleanly.
}

/// Open a file, decode it via rodio's symphonia-backed decoder, and build a
/// paused sink primed with the decoded source.
fn open_sink(
    handle: &OutputStreamHandle,
    path: &Path,
    volume: f32,
) -> Result<(Sink, Option<Duration>), OtoError> {
    let file = File::open(path)?;
    let decoder =
        Decoder::new(BufReader::new(file)).map_err(|e| OtoError::DecoderError(e.to_string()))?;
    let duration = decoder.total_duration();
    let sink = Sink::try_new(handle).map_err(|e| OtoError::PlaybackFailed(e.to_string()))?;
    sink.set_volume(volume);
    sink.append(decoder);
    sink.pause();
    Ok((sink, duration))
}

fn duration_to_ms(d: Option<Duration>) -> u64 {
    match d {
        // Cap below the NO_DURATION sentinel so a real (huge) duration is never
        // mistaken for "unknown".
        Some(dur) => u64::try_from(dur.as_millis()).unwrap_or(NO_DURATION - 1),
        None => NO_DURATION,
    }
}

fn ms_to_duration(ms: u64) -> Option<Duration> {
    if ms == NO_DURATION {
        None
    } else {
        Some(Duration::from_millis(ms))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_ms_roundtrip() {
        assert_eq!(ms_to_duration(duration_to_ms(None)), None);
        assert_eq!(
            ms_to_duration(duration_to_ms(Some(Duration::from_millis(1234)))),
            Some(Duration::from_millis(1234))
        );
    }

    #[test]
    fn duration_to_ms_none_is_sentinel() {
        assert_eq!(duration_to_ms(None), NO_DURATION);
    }

    #[test]
    fn construction_does_not_panic_headless() {
        // On a machine with no audio device this returns Err rather than
        // panicking; on one with a device it returns Ok. Either is acceptable —
        // the contract is "no panic in library code".
        match RodioBackend::new() {
            Ok(mut backend) => {
                // A freshly-built backend reports no loaded track.
                assert!(backend.position().is_none());
                assert!(!backend.is_ended());
                assert!((backend.volume() - DEFAULT_VOLUME).abs() < f32::EPSILON);
                let _ = backend.set_volume(0.5);
                assert!((backend.volume() - 0.5).abs() < f32::EPSILON);
            }
            Err(e) => {
                // Typed error, not a panic.
                let _ = e.to_string();
            }
        }
    }
}

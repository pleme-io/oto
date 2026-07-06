//! Playback backend seam — the injectable audio-output/decoder I/O trait,
//! peer of [`crate::device::AudioDeviceProvider`].
//!
//! [`PlaybackBackend`] abstracts the *actual* audio output + decode so a
//! consumer drives the pure [`Player`] state machine through an injected
//! implementation instead of re-rolling a rodio thread of its own (Quadro
//! §II-T10 / pillar P10 — apps embed typed I/O cores, never re-roll them).
//!
//! - [`MockBackend`] is the zero-I/O implementation, peer of
//!   [`crate::device::MockDeviceProvider`]; it is the testability contract.
//! - The `backend-rodio` feature adds [`crate::rodio_backend::RodioBackend`],
//!   the real rodio + symphonia implementation whose dedicated audio OS thread
//!   lives entirely *behind* this seam.
//! - [`Engine`] is the thin, backend-swappable handle wiring the pure
//!   [`Player`] FSM + [`QueueManager`] to an injected backend.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::OtoError;
use crate::player::{PlaybackState, Player};
use crate::queue::QueueManager;

/// Injectable audio-output + decode seam.
///
/// A `PlaybackBackend` owns the concrete audio pipeline (decoder + output
/// sink). The pure [`Player`] FSM and [`QueueManager`] drive it through
/// [`Engine`], so consumers get backend-swappable, mockable playback without
/// re-rolling the audio stack. Peer of [`crate::device::AudioDeviceProvider`].
///
/// The seam deliberately does not expose the decoded source as a value: the
/// decoded-source type is inherently backend-specific (rodio's `Source`), so
/// crossing it over the seam would leak the backend. `load` therefore performs
/// open + decode internally and hands back only the front-end-agnostic
/// duration.
///
/// The trait is object-safe; `Box<dyn PlaybackBackend>` implements it, so
/// `Engine<Box<dyn PlaybackBackend>>` supports runtime backend selection.
pub trait PlaybackBackend {
    /// Open and decode a track from a filesystem path, readying it for
    /// playback in a paused state at position zero. Replaces any
    /// currently-loaded track. Returns the track's total duration when the
    /// decoder can determine it.
    fn load(&mut self, path: &Path) -> Result<Option<Duration>, OtoError>;

    /// Begin playback of the loaded track from its current position.
    fn play(&mut self) -> Result<(), OtoError>;

    /// Pause playback, retaining the current position.
    fn pause(&mut self) -> Result<(), OtoError>;

    /// Resume playback after a pause.
    fn resume(&mut self) -> Result<(), OtoError>;

    /// Stop playback and discard the loaded track.
    fn stop(&mut self) -> Result<(), OtoError>;

    /// Seek to an absolute position within the loaded track.
    fn seek(&mut self, position: Duration) -> Result<(), OtoError>;

    /// Set the output volume, where `0.0` is silent and `1.0` is unity gain.
    fn set_volume(&mut self, volume: f32) -> Result<(), OtoError>;

    /// Current output volume in `0.0..=1.0`.
    fn volume(&self) -> f32;

    /// Current playback position, if a track is loaded.
    fn position(&self) -> Option<Duration>;

    /// Total duration of the loaded track, if known.
    fn duration(&self) -> Option<Duration>;

    /// Whether the loaded track has played to completion since it was loaded.
    ///
    /// Consumers poll this (via [`Engine::tick`]) to advance the queue at
    /// end-of-track.
    fn is_ended(&self) -> bool;
}

impl PlaybackBackend for Box<dyn PlaybackBackend> {
    fn load(&mut self, path: &Path) -> Result<Option<Duration>, OtoError> {
        (**self).load(path)
    }
    fn play(&mut self) -> Result<(), OtoError> {
        (**self).play()
    }
    fn pause(&mut self) -> Result<(), OtoError> {
        (**self).pause()
    }
    fn resume(&mut self) -> Result<(), OtoError> {
        (**self).resume()
    }
    fn stop(&mut self) -> Result<(), OtoError> {
        (**self).stop()
    }
    fn seek(&mut self, position: Duration) -> Result<(), OtoError> {
        (**self).seek(position)
    }
    fn set_volume(&mut self, volume: f32) -> Result<(), OtoError> {
        (**self).set_volume(volume)
    }
    fn volume(&self) -> f32 {
        (**self).volume()
    }
    fn position(&self) -> Option<Duration> {
        (**self).position()
    }
    fn duration(&self) -> Option<Duration> {
        (**self).duration()
    }
    fn is_ended(&self) -> bool {
        (**self).is_ended()
    }
}

/// Headless [`PlaybackBackend`] with zero real audio I/O.
///
/// Peer of [`crate::device::MockDeviceProvider`]. Simulates load / play /
/// seek / end-of-track so [`Engine`] can be driven and tested without
/// hardware. Test code moves simulated playback time forward with
/// [`MockBackend::advance`].
#[derive(Debug, Clone)]
pub struct MockBackend {
    default_duration: Option<Duration>,
    durations: HashMap<PathBuf, Duration>,
    fail_paths: HashSet<PathBuf>,
    loaded: Option<PathBuf>,
    duration: Option<Duration>,
    position: Duration,
    playing: bool,
    volume: f32,
    ended: bool,
}

impl MockBackend {
    /// Create a mock backend that reports a 180-second duration for every
    /// loaded track.
    #[must_use]
    pub fn new() -> Self {
        Self {
            default_duration: Some(Duration::from_secs(180)),
            durations: HashMap::new(),
            fail_paths: HashSet::new(),
            loaded: None,
            duration: None,
            position: Duration::ZERO,
            playing: false,
            volume: 1.0,
            ended: false,
        }
    }

    /// Set the duration reported for tracks without an explicit per-path entry.
    #[must_use]
    pub fn with_default_duration(mut self, duration: Option<Duration>) -> Self {
        self.default_duration = duration;
        self
    }

    /// Register an explicit duration for a specific path.
    pub fn set_track_duration(&mut self, path: impl Into<PathBuf>, duration: Duration) {
        self.durations.insert(path.into(), duration);
    }

    /// Mark a path so that loading it returns a decoder error (error-path
    /// testing).
    pub fn fail_on(&mut self, path: impl Into<PathBuf>) {
        self.fail_paths.insert(path.into());
    }

    /// Move simulated playback time forward. When playing, advances the
    /// position; on reaching the track duration, marks end-of-track and
    /// pauses (mirroring a real sink draining).
    pub fn advance(&mut self, elapsed: Duration) {
        if !self.playing {
            return;
        }
        self.position = self.position.saturating_add(elapsed);
        if let Some(dur) = self.duration
            && self.position >= dur
        {
            self.position = dur;
            self.playing = false;
            self.ended = true;
        }
    }

    /// Whether the mock currently considers itself playing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.playing
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaybackBackend for MockBackend {
    fn load(&mut self, path: &Path) -> Result<Option<Duration>, OtoError> {
        if self.fail_paths.contains(path) {
            return Err(OtoError::DecoderError(path.display().to_string()));
        }
        let duration = self.durations.get(path).copied().or(self.default_duration);
        self.loaded = Some(path.to_path_buf());
        self.duration = duration;
        self.position = Duration::ZERO;
        self.playing = false;
        self.ended = false;
        Ok(duration)
    }

    fn play(&mut self) -> Result<(), OtoError> {
        if self.loaded.is_some() && !self.ended {
            self.playing = true;
        }
        Ok(())
    }

    fn pause(&mut self) -> Result<(), OtoError> {
        self.playing = false;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), OtoError> {
        self.play()
    }

    fn stop(&mut self) -> Result<(), OtoError> {
        self.loaded = None;
        self.duration = None;
        self.position = Duration::ZERO;
        self.playing = false;
        self.ended = false;
        Ok(())
    }

    fn seek(&mut self, position: Duration) -> Result<(), OtoError> {
        let clamped = self.duration.map_or(position, |d| position.min(d));
        self.position = clamped;
        if self.duration.is_some_and(|d| clamped < d) {
            self.ended = false;
        }
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<(), OtoError> {
        self.volume = volume.clamp(0.0, 1.0);
        Ok(())
    }

    fn volume(&self) -> f32 {
        self.volume
    }

    fn position(&self) -> Option<Duration> {
        self.loaded.as_ref().map(|_| self.position)
    }

    fn duration(&self) -> Option<Duration> {
        self.duration
    }

    fn is_ended(&self) -> bool {
        self.ended
    }
}

/// Outcome of an [`Engine::tick`] poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TickOutcome {
    /// Nothing changed since the last tick.
    Unchanged,
    /// The previous track ended and the named track started.
    Advanced(String),
    /// The previous track ended and the queue is now exhausted; playback
    /// stopped.
    Finished,
}

/// Headless playback engine: the pure [`Player`] FSM + [`QueueManager`] wired
/// to an injected [`PlaybackBackend`].
///
/// Construct with any backend — [`MockBackend`] for tests, or the real
/// `RodioBackend` (feature `backend-rodio`) — and drive
/// play / pause / seek / next / prev / volume / queue. This is the embeddable
/// core shape: headless logic + injected I/O, backend-swappable and mockable.
///
/// Track identifiers in the queue are filesystem paths (as strings); the
/// engine hands each to the backend's [`PlaybackBackend::load`].
pub struct Engine<B: PlaybackBackend> {
    backend: B,
    player: Player,
    queue: QueueManager,
    loaded: bool,
}

impl<B: PlaybackBackend> Engine<B> {
    /// Create an engine over the given backend with an empty queue.
    #[must_use]
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            player: Player::new(),
            queue: QueueManager::new(),
            loaded: false,
        }
    }

    /// Borrow the injected backend.
    #[must_use]
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Mutably borrow the injected backend (e.g. to advance a [`MockBackend`]
    /// in tests).
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Borrow the queue manager.
    #[must_use]
    pub fn queue(&self) -> &QueueManager {
        &self.queue
    }

    /// Mutably borrow the queue manager for enqueue / reorder / repeat control.
    pub fn queue_mut(&mut self) -> &mut QueueManager {
        &mut self.queue
    }

    /// Append a track path to the back of the queue.
    pub fn enqueue(&mut self, track: impl Into<String>) {
        self.queue.queue_mut().push(track.into());
    }

    /// Current playback state.
    #[must_use]
    pub fn state(&self) -> PlaybackState {
        self.player.state()
    }

    /// Whether playback is currently active.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.player.state() == PlaybackState::Playing
    }

    /// Path of the loaded track, if any.
    #[must_use]
    pub fn current_track(&self) -> Option<&str> {
        self.player.current_track()
    }

    /// Live playback position from the backend, if a track is loaded.
    #[must_use]
    pub fn position(&self) -> Option<Duration> {
        self.backend.position()
    }

    /// Total duration of the loaded track, if known.
    #[must_use]
    pub fn duration(&self) -> Option<Duration> {
        self.backend.duration()
    }

    /// Output volume in `0.0..=1.0`.
    #[must_use]
    pub fn volume(&self) -> f32 {
        self.backend.volume()
    }

    /// Progress through the current track as a fraction in `0.0..=1.0`.
    #[must_use]
    pub fn progress(&self) -> f64 {
        match (self.backend.position(), self.backend.duration()) {
            (Some(pos), Some(dur)) if dur > Duration::ZERO => {
                (pos.as_secs_f64() / dur.as_secs_f64()).clamp(0.0, 1.0)
            }
            _ => 0.0,
        }
    }

    fn load_track(&mut self, track: String) -> Result<(), OtoError> {
        let dur = self.backend.load(Path::new(&track))?;
        let secs = dur.map_or(0.0, |d| d.as_secs_f64());
        self.player.set_track(track, secs);
        self.loaded = true;
        Ok(())
    }

    /// Start or resume playback.
    ///
    /// - Playing: no-op.
    /// - Paused: resume in place.
    /// - Stopped with a loaded track: play it.
    /// - Stopped with nothing loaded: reload the queue's current track, or
    ///   pull the next one if there is no current track. Returns without error
    ///   when the queue is empty.
    pub fn play(&mut self) -> Result<(), OtoError> {
        match self.player.state() {
            PlaybackState::Playing => {}
            PlaybackState::Paused => {
                self.backend.resume()?;
                self.player.play();
            }
            PlaybackState::Stopped => {
                if !self.loaded {
                    let Some(track) = self
                        .queue
                        .current()
                        .map(str::to_owned)
                        .or_else(|| self.queue.advance())
                    else {
                        return Ok(());
                    };
                    self.load_track(track)?;
                }
                self.backend.play()?;
                self.player.play();
            }
        }
        Ok(())
    }

    /// Pause playback (no-op unless currently playing).
    pub fn pause(&mut self) -> Result<(), OtoError> {
        if self.player.state() == PlaybackState::Playing {
            self.backend.pause()?;
            self.player.pause();
        }
        Ok(())
    }

    /// Toggle between play and pause.
    pub fn toggle(&mut self) -> Result<(), OtoError> {
        match self.player.state() {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused | PlaybackState::Stopped => self.play(),
        }
    }

    /// Stop playback and discard the loaded track.
    pub fn stop(&mut self) -> Result<(), OtoError> {
        self.backend.stop()?;
        self.player.stop();
        self.loaded = false;
        Ok(())
    }

    /// Advance to the next queued track, loading and playing it. Returns the
    /// started track, or `None` (and stops) when the queue is exhausted.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<String>, OtoError> {
        if let Some(track) = self.queue.advance() {
            self.load_track(track.clone())?;
            self.backend.play()?;
            self.player.play();
            Ok(Some(track))
        } else {
            self.stop()?;
            Ok(None)
        }
    }

    /// Go back to the previous track from history, loading and playing it.
    pub fn previous(&mut self) -> Result<Option<String>, OtoError> {
        match self.queue.previous() {
            Some(track) => {
                self.load_track(track.clone())?;
                self.backend.play()?;
                self.player.play();
                Ok(Some(track))
            }
            None => Ok(None),
        }
    }

    /// Seek to an absolute position in seconds, clamped to `0.0..=duration`.
    pub fn seek(&mut self, position_secs: f64) -> Result<(), OtoError> {
        let clamped = match self.backend.duration().map(|d| d.as_secs_f64()) {
            Some(max) => position_secs.clamp(0.0, max),
            None => position_secs.max(0.0),
        };
        // Guard against NaN / infinity before `Duration::from_secs_f64`, which
        // panics on non-finite input.
        let clamped = if clamped.is_finite() { clamped } else { 0.0 };
        self.backend.seek(Duration::from_secs_f64(clamped))?;
        self.player.seek(clamped);
        Ok(())
    }

    /// Set the output volume, clamped to `0.0..=1.0`.
    pub fn set_volume(&mut self, volume: f32) -> Result<(), OtoError> {
        self.backend.set_volume(volume.clamp(0.0, 1.0))
    }

    /// Poll the backend for end-of-track and auto-advance the queue.
    ///
    /// Call on the consumer's render / tick loop. Returns a [`TickOutcome`]
    /// describing whether a track change occurred.
    pub fn tick(&mut self) -> Result<TickOutcome, OtoError> {
        if self.player.state() == PlaybackState::Playing && self.loaded && self.backend.is_ended() {
            match self.next()? {
                Some(track) => Ok(TickOutcome::Advanced(track)),
                None => Ok(TickOutcome::Finished),
            }
        } else {
            Ok(TickOutcome::Unchanged)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- MockBackend ---

    #[test]
    fn mock_load_reports_default_duration() {
        let mut b = MockBackend::new();
        let dur = b.load(Path::new("song.flac")).unwrap();
        assert_eq!(dur, Some(Duration::from_secs(180)));
        assert_eq!(b.position(), Some(Duration::ZERO));
        assert!(!b.is_playing());
        assert!(!b.is_ended());
    }

    #[test]
    fn mock_per_path_duration_overrides_default() {
        let mut b = MockBackend::new();
        b.set_track_duration("short.mp3", Duration::from_secs(5));
        assert_eq!(
            b.load(Path::new("short.mp3")).unwrap(),
            Some(Duration::from_secs(5))
        );
        assert_eq!(
            b.load(Path::new("other.mp3")).unwrap(),
            Some(Duration::from_secs(180))
        );
    }

    #[test]
    fn mock_with_default_duration_none() {
        let mut b = MockBackend::new().with_default_duration(None);
        assert_eq!(b.load(Path::new("stream.ogg")).unwrap(), None);
        assert_eq!(b.duration(), None);
    }

    #[test]
    fn mock_fail_on_returns_decoder_error() {
        let mut b = MockBackend::new();
        b.fail_on("broken.flac");
        let err = b.load(Path::new("broken.flac")).unwrap_err();
        assert!(matches!(err, OtoError::DecoderError(_)));
    }

    #[test]
    fn mock_play_pause_toggle_playing_flag() {
        let mut b = MockBackend::new();
        b.load(Path::new("t.flac")).unwrap();
        b.play().unwrap();
        assert!(b.is_playing());
        b.pause().unwrap();
        assert!(!b.is_playing());
        b.resume().unwrap();
        assert!(b.is_playing());
    }

    #[test]
    fn mock_play_without_load_is_noop() {
        let mut b = MockBackend::new();
        b.play().unwrap();
        assert!(!b.is_playing());
        assert_eq!(b.position(), None);
    }

    #[test]
    fn mock_advance_reaches_end_of_track() {
        let mut b = MockBackend::new();
        b.set_track_duration("t.flac", Duration::from_secs(10));
        b.load(Path::new("t.flac")).unwrap();
        b.play().unwrap();
        b.advance(Duration::from_secs(4));
        assert_eq!(b.position(), Some(Duration::from_secs(4)));
        assert!(!b.is_ended());
        b.advance(Duration::from_secs(100));
        assert_eq!(b.position(), Some(Duration::from_secs(10)));
        assert!(b.is_ended());
        assert!(!b.is_playing());
    }

    #[test]
    fn mock_advance_does_nothing_when_paused() {
        let mut b = MockBackend::new();
        b.load(Path::new("t.flac")).unwrap();
        // not playing
        b.advance(Duration::from_secs(30));
        assert_eq!(b.position(), Some(Duration::ZERO));
    }

    #[test]
    fn mock_seek_clamps_to_duration() {
        let mut b = MockBackend::new();
        b.set_track_duration("t.flac", Duration::from_secs(10));
        b.load(Path::new("t.flac")).unwrap();
        b.seek(Duration::from_secs(999)).unwrap();
        assert_eq!(b.position(), Some(Duration::from_secs(10)));
    }

    #[test]
    fn mock_stop_clears_state() {
        let mut b = MockBackend::new();
        b.load(Path::new("t.flac")).unwrap();
        b.play().unwrap();
        b.stop().unwrap();
        assert_eq!(b.position(), None);
        assert_eq!(b.duration(), None);
        assert!(!b.is_playing());
        assert!(!b.is_ended());
    }

    #[test]
    fn mock_set_volume_clamps() {
        let mut b = MockBackend::new();
        b.set_volume(2.5).unwrap();
        assert!((b.volume() - 1.0).abs() < f32::EPSILON);
        b.set_volume(-1.0).unwrap();
        assert!(b.volume().abs() < f32::EPSILON);
        b.set_volume(0.4).unwrap();
        assert!((b.volume() - 0.4).abs() < f32::EPSILON);
    }

    // --- Engine + MockBackend ---

    fn engine_with_two_tracks() -> Engine<MockBackend> {
        let mut engine = Engine::new(MockBackend::new());
        engine.enqueue("a.flac");
        engine.enqueue("b.flac");
        engine
    }

    #[test]
    fn engine_new_is_stopped_and_empty() {
        let engine = Engine::new(MockBackend::new());
        assert_eq!(engine.state(), PlaybackState::Stopped);
        assert!(!engine.is_playing());
        assert!(engine.current_track().is_none());
        assert!(engine.queue().queue().is_empty());
    }

    #[test]
    fn engine_play_loads_first_queued_track() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap();
        assert_eq!(engine.state(), PlaybackState::Playing);
        assert_eq!(engine.current_track(), Some("a.flac"));
        assert!(engine.backend().is_playing());
        assert_eq!(engine.duration(), Some(Duration::from_secs(180)));
    }

    #[test]
    fn engine_play_on_empty_queue_is_noop() {
        let mut engine = Engine::new(MockBackend::new());
        engine.play().unwrap();
        assert_eq!(engine.state(), PlaybackState::Stopped);
        assert!(engine.current_track().is_none());
    }

    #[test]
    fn engine_pause_resume_roundtrip() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap();
        engine.pause().unwrap();
        assert_eq!(engine.state(), PlaybackState::Paused);
        assert!(!engine.backend().is_playing());
        engine.play().unwrap();
        assert_eq!(engine.state(), PlaybackState::Playing);
        assert!(engine.backend().is_playing());
    }

    #[test]
    fn engine_toggle_cycles_states() {
        let mut engine = engine_with_two_tracks();
        engine.toggle().unwrap(); // stopped -> playing (loads a.flac)
        assert_eq!(engine.state(), PlaybackState::Playing);
        engine.toggle().unwrap(); // playing -> paused
        assert_eq!(engine.state(), PlaybackState::Paused);
        engine.toggle().unwrap(); // paused -> playing
        assert_eq!(engine.state(), PlaybackState::Playing);
    }

    #[test]
    fn engine_seek_clamps_and_syncs_backend() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap(); // a.flac, 180s
        engine.seek(90.0).unwrap();
        assert_eq!(engine.position(), Some(Duration::from_secs(90)));
        engine.seek(9999.0).unwrap();
        assert_eq!(engine.position(), Some(Duration::from_secs(180)));
        engine.seek(-10.0).unwrap();
        assert_eq!(engine.position(), Some(Duration::ZERO));
    }

    #[test]
    fn engine_seek_ignores_non_finite() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap();
        engine.seek(f64::NAN).unwrap();
        assert_eq!(engine.position(), Some(Duration::ZERO));
        engine.seek(f64::INFINITY).unwrap();
        // clamped to duration (finite); infinity guarded before from_secs_f64
        assert_eq!(engine.position(), Some(Duration::from_secs(180)));
    }

    #[test]
    fn engine_next_advances_queue() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap();
        assert_eq!(engine.current_track(), Some("a.flac"));
        let started = engine.next().unwrap();
        assert_eq!(started, Some("b.flac".to_owned()));
        assert_eq!(engine.current_track(), Some("b.flac"));
        assert_eq!(engine.state(), PlaybackState::Playing);
    }

    #[test]
    fn engine_next_past_end_stops() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap(); // a
        engine.next().unwrap(); // b
        let none = engine.next().unwrap(); // exhausted
        assert_eq!(none, None);
        assert_eq!(engine.state(), PlaybackState::Stopped);
    }

    #[test]
    fn engine_previous_returns_to_prior_track() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap(); // a
        engine.next().unwrap(); // b, history = [a]
        let prev = engine.previous().unwrap();
        assert_eq!(prev, Some("a.flac".to_owned()));
        assert_eq!(engine.current_track(), Some("a.flac"));
        assert_eq!(engine.state(), PlaybackState::Playing);
    }

    #[test]
    fn engine_previous_with_no_history_is_none() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap();
        assert_eq!(engine.previous().unwrap(), None);
    }

    #[test]
    fn engine_end_of_track_auto_advances_on_tick() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap(); // a.flac
        assert_eq!(engine.tick().unwrap(), TickOutcome::Unchanged);
        // simulate a.flac draining to completion
        engine.backend_mut().advance(Duration::from_secs(1_000));
        let outcome = engine.tick().unwrap();
        assert_eq!(outcome, TickOutcome::Advanced("b.flac".to_owned()));
        assert_eq!(engine.current_track(), Some("b.flac"));
        assert_eq!(engine.state(), PlaybackState::Playing);
    }

    #[test]
    fn engine_tick_finishes_when_queue_drains() {
        let mut engine = Engine::new(MockBackend::new());
        engine.enqueue("only.flac");
        engine.play().unwrap();
        engine.backend_mut().advance(Duration::from_secs(1_000));
        let outcome = engine.tick().unwrap();
        assert_eq!(outcome, TickOutcome::Finished);
        assert_eq!(engine.state(), PlaybackState::Stopped);
    }

    #[test]
    fn engine_stop_then_play_replays_current_track() {
        let mut engine = engine_with_two_tracks();
        engine.play().unwrap(); // a.flac
        engine.seek(50.0).unwrap();
        engine.stop().unwrap();
        assert_eq!(engine.state(), PlaybackState::Stopped);
        engine.play().unwrap();
        // stop discarded the load; play reloads the current (a.flac) at 0
        assert_eq!(engine.current_track(), Some("a.flac"));
        assert_eq!(engine.position(), Some(Duration::ZERO));
        assert_eq!(engine.state(), PlaybackState::Playing);
    }

    #[test]
    fn engine_volume_set_and_get() {
        let mut engine = Engine::new(MockBackend::new());
        engine.set_volume(0.3).unwrap();
        assert!((engine.volume() - 0.3).abs() < f32::EPSILON);
        engine.set_volume(5.0).unwrap();
        assert!((engine.volume() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn engine_progress_is_fraction() {
        let mut engine = Engine::new(MockBackend::new());
        engine.enqueue("t.flac");
        engine
            .backend_mut()
            .set_track_duration("t.flac", Duration::from_secs(100));
        engine.play().unwrap();
        engine.seek(25.0).unwrap();
        assert!((engine.progress() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn engine_load_error_propagates() {
        let mut engine = Engine::new(MockBackend::new());
        engine.backend_mut().fail_on("bad.flac");
        engine.enqueue("bad.flac");
        let err = engine.play().unwrap_err();
        assert!(matches!(err, OtoError::DecoderError(_)));
    }

    #[test]
    fn engine_over_boxed_dyn_backend() {
        // Runtime backend selection through a trait object.
        let backend: Box<dyn PlaybackBackend> = Box::new(MockBackend::new());
        let mut engine = Engine::new(backend);
        engine.enqueue("a.flac");
        engine.play().unwrap();
        assert_eq!(engine.state(), PlaybackState::Playing);
        assert_eq!(engine.current_track(), Some("a.flac"));
    }

    #[test]
    fn playback_backend_is_object_safe() {
        let _b: Box<dyn PlaybackBackend> = Box::new(MockBackend::new());
    }
}

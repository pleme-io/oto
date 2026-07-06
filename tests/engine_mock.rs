//! End-to-end coverage of the embeddable core: the pure `Player` FSM wired to
//! an injected `MockBackend` through `Engine`, exercised via the public API
//! only (play / pause / seek / queue / next / prev / end-of-track).

use std::time::Duration;

use oto::{Engine, MockBackend, PlaybackBackend, PlaybackState, TickOutcome};

#[test]
fn full_session_play_seek_queue_advance() {
    let mut engine = Engine::new(MockBackend::new());
    engine.enqueue("track-1.flac");
    engine.enqueue("track-2.mp3");
    engine.enqueue("track-3.ogg");

    // Start playback: pulls the first track from the queue.
    engine.play().unwrap();
    assert_eq!(engine.state(), PlaybackState::Playing);
    assert_eq!(engine.current_track(), Some("track-1.flac"));

    // Seek within the track, clamped to duration.
    engine.seek(60.0).unwrap();
    assert_eq!(engine.position(), Some(Duration::from_secs(60)));

    // Pause / resume roundtrip.
    engine.pause().unwrap();
    assert_eq!(engine.state(), PlaybackState::Paused);
    engine.play().unwrap();
    assert_eq!(engine.state(), PlaybackState::Playing);

    // Skip forward and back.
    assert_eq!(engine.next().unwrap(), Some("track-2.mp3".to_owned()));
    assert_eq!(engine.previous().unwrap(), Some("track-1.flac".to_owned()));
    assert_eq!(engine.current_track(), Some("track-1.flac"));

    // Volume clamps.
    engine.set_volume(0.75).unwrap();
    assert!((engine.volume() - 0.75).abs() < f32::EPSILON);
}

#[test]
fn end_of_track_auto_advances_until_queue_drains() {
    let mut engine = Engine::new(MockBackend::new());
    engine.enqueue("a.flac");
    engine.enqueue("b.flac");
    engine.play().unwrap();

    // Drain track a -> tick advances to b.
    engine.backend_mut().advance(Duration::from_secs(10_000));
    assert_eq!(
        engine.tick().unwrap(),
        TickOutcome::Advanced("b.flac".to_owned())
    );

    // Drain track b -> tick finishes (queue empty).
    engine.backend_mut().advance(Duration::from_secs(10_000));
    assert_eq!(engine.tick().unwrap(), TickOutcome::Finished);
    assert_eq!(engine.state(), PlaybackState::Stopped);
}

#[test]
fn swappable_backend_via_trait_object() {
    let backend: Box<dyn PlaybackBackend> = Box::new(MockBackend::new());
    let mut engine = Engine::new(backend);
    engine.enqueue("x.flac");
    engine.play().unwrap();
    assert!(engine.is_playing());
    assert_eq!(engine.current_track(), Some("x.flac"));
}

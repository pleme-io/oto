/// Audio playback state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

/// Audio player state machine.
///
/// Manages playback state, current track, and position.
/// Pure logic — no hardware interaction.
pub struct Player {
    state: PlaybackState,
    position_secs: f64,
    track_name: Option<String>,
    track_duration: Option<f64>,
}

impl Player {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: PlaybackState::Stopped,
            position_secs: 0.0,
            track_name: None,
            track_duration: None,
        }
    }

    /// Transition to `Playing` state.
    pub fn play(&mut self) {
        self.state = PlaybackState::Playing;
    }

    /// Transition to `Paused` state.
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    /// Transition to `Stopped` state and reset position.
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.position_secs = 0.0;
    }

    /// Toggle between `Playing` and `Paused`.
    ///
    /// When stopped, transitions to `Playing`.
    pub fn toggle(&mut self) {
        match self.state {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused | PlaybackState::Stopped => self.play(),
        }
    }

    /// Set the current position in seconds.
    pub fn seek(&mut self, position_secs: f64) {
        let clamped = if let Some(dur) = self.track_duration {
            position_secs.clamp(0.0, dur)
        } else {
            position_secs.max(0.0)
        };
        self.position_secs = clamped;
    }

    /// Current playback state.
    #[must_use]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Current playback position in seconds.
    #[must_use]
    pub fn position(&self) -> f64 {
        self.position_secs
    }

    /// Duration of the current track, if one is loaded.
    #[must_use]
    pub fn duration(&self) -> Option<f64> {
        self.track_duration
    }

    /// Load a new track by name and duration.
    ///
    /// Resets position to zero and stops playback.
    pub fn set_track(&mut self, name: String, duration: f64) {
        self.track_name = Some(name);
        self.track_duration = Some(duration);
        self.position_secs = 0.0;
        self.state = PlaybackState::Stopped;
    }

    /// Name of the current track, if one is loaded.
    #[must_use]
    pub fn current_track(&self) -> Option<&str> {
        self.track_name.as_deref()
    }

    /// Progress as a fraction from 0.0 to 1.0.
    ///
    /// Returns 0.0 if no track is loaded or duration is zero.
    #[must_use]
    pub fn progress(&self) -> f64 {
        match self.track_duration {
            Some(dur) if dur > 0.0 => (self.position_secs / dur).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_player_is_stopped() {
        let player = Player::new();
        assert_eq!(player.state(), PlaybackState::Stopped);
        assert!(player.position().abs() < f64::EPSILON);
        assert!(player.current_track().is_none());
        assert!(player.duration().is_none());
    }

    #[test]
    fn play_sets_playing() {
        let mut player = Player::new();
        player.play();
        assert_eq!(player.state(), PlaybackState::Playing);
    }

    #[test]
    fn pause_from_playing() {
        let mut player = Player::new();
        player.play();
        player.pause();
        assert_eq!(player.state(), PlaybackState::Paused);
    }

    #[test]
    fn pause_from_stopped_stays_stopped() {
        let mut player = Player::new();
        player.pause();
        assert_eq!(player.state(), PlaybackState::Stopped);
    }

    #[test]
    fn stop_resets_position() {
        let mut player = Player::new();
        player.set_track("song.flac".into(), 200.0);
        player.play();
        player.seek(100.0);
        player.stop();
        assert_eq!(player.state(), PlaybackState::Stopped);
        assert!(player.position().abs() < f64::EPSILON);
    }

    #[test]
    fn toggle_stopped_to_playing() {
        let mut player = Player::new();
        player.toggle();
        assert_eq!(player.state(), PlaybackState::Playing);
    }

    #[test]
    fn toggle_playing_to_paused() {
        let mut player = Player::new();
        player.play();
        player.toggle();
        assert_eq!(player.state(), PlaybackState::Paused);
    }

    #[test]
    fn toggle_paused_to_playing() {
        let mut player = Player::new();
        player.play();
        player.pause();
        player.toggle();
        assert_eq!(player.state(), PlaybackState::Playing);
    }

    #[test]
    fn seek_sets_position() {
        let mut player = Player::new();
        player.set_track("track.mp3".into(), 300.0);
        player.seek(150.0);
        assert!((player.position() - 150.0).abs() < f64::EPSILON);
    }

    #[test]
    fn seek_clamps_to_duration() {
        let mut player = Player::new();
        player.set_track("track.mp3".into(), 200.0);
        player.seek(500.0);
        assert!((player.position() - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn seek_clamps_negative_to_zero() {
        let mut player = Player::new();
        player.seek(-10.0);
        assert!((player.position() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn set_track_resets_state() {
        let mut player = Player::new();
        player.play();
        player.seek(50.0);
        player.set_track("new_song.flac".into(), 180.0);
        assert_eq!(player.state(), PlaybackState::Stopped);
        assert!(player.position().abs() < f64::EPSILON);
        assert_eq!(player.current_track(), Some("new_song.flac"));
        assert!((player.duration().unwrap() - 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_no_track() {
        let player = Player::new();
        assert!((player.progress() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_mid_track() {
        let mut player = Player::new();
        player.set_track("song.wav".into(), 100.0);
        player.seek(25.0);
        assert!((player.progress() - 0.25).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_at_end() {
        let mut player = Player::new();
        player.set_track("song.wav".into(), 100.0);
        player.seek(100.0);
        assert!((player.progress() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_zero_duration() {
        let mut player = Player::new();
        player.set_track("empty.wav".into(), 0.0);
        assert!((player.progress() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_equals_new() {
        let a = Player::new();
        let b = Player::default();
        assert_eq!(a.state(), b.state());
        assert!((a.position() - b.position()).abs() < f64::EPSILON);
    }
}

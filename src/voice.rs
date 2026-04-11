/// State of a voice stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceState {
    /// Not active.
    Idle,
    /// Capturing audio input (microphone).
    Listening,
    /// Sending audio output.
    Transmitting,
}

/// Voice stream state machine for real-time audio communication.
///
/// Manages listening/transmitting states and mute control.
/// Pure logic — no hardware interaction.
pub struct VoiceStream {
    state: VoiceState,
    muted: bool,
}

impl VoiceStream {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: VoiceState::Idle,
            muted: false,
        }
    }

    /// Start capturing audio input.
    pub fn start_listening(&mut self) {
        self.state = VoiceState::Listening;
    }

    /// Stop capturing audio input, return to idle.
    pub fn stop_listening(&mut self) {
        if self.state == VoiceState::Listening {
            self.state = VoiceState::Idle;
        }
    }

    /// Start transmitting audio.
    pub fn start_transmitting(&mut self) {
        self.state = VoiceState::Transmitting;
    }

    /// Stop transmitting, return to idle.
    pub fn stop_transmitting(&mut self) {
        if self.state == VoiceState::Transmitting {
            self.state = VoiceState::Idle;
        }
    }

    /// Current voice state.
    #[must_use]
    pub fn state(&self) -> VoiceState {
        self.state
    }

    /// Whether the stream is doing anything (not idle).
    #[must_use]
    pub fn is_active(&self) -> bool {
        self.state != VoiceState::Idle
    }

    /// Mute the microphone.
    pub fn mute(&mut self) {
        self.muted = true;
    }

    /// Unmute the microphone.
    pub fn unmute(&mut self) {
        self.muted = false;
    }

    /// Whether the microphone is muted.
    #[must_use]
    pub fn is_muted(&self) -> bool {
        self.muted
    }
}

impl Default for VoiceStream {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_voice_stream_is_idle() {
        let vs = VoiceStream::new();
        assert_eq!(vs.state(), VoiceState::Idle);
        assert!(!vs.is_active());
        assert!(!vs.is_muted());
    }

    #[test]
    fn start_listening_sets_listening() {
        let mut vs = VoiceStream::new();
        vs.start_listening();
        assert_eq!(vs.state(), VoiceState::Listening);
        assert!(vs.is_active());
    }

    #[test]
    fn stop_listening_returns_to_idle() {
        let mut vs = VoiceStream::new();
        vs.start_listening();
        vs.stop_listening();
        assert_eq!(vs.state(), VoiceState::Idle);
        assert!(!vs.is_active());
    }

    #[test]
    fn stop_listening_only_from_listening() {
        let mut vs = VoiceStream::new();
        vs.start_transmitting();
        vs.stop_listening(); // should not change state
        assert_eq!(vs.state(), VoiceState::Transmitting);
    }

    #[test]
    fn start_transmitting_sets_transmitting() {
        let mut vs = VoiceStream::new();
        vs.start_transmitting();
        assert_eq!(vs.state(), VoiceState::Transmitting);
        assert!(vs.is_active());
    }

    #[test]
    fn stop_transmitting_returns_to_idle() {
        let mut vs = VoiceStream::new();
        vs.start_transmitting();
        vs.stop_transmitting();
        assert_eq!(vs.state(), VoiceState::Idle);
    }

    #[test]
    fn stop_transmitting_only_from_transmitting() {
        let mut vs = VoiceStream::new();
        vs.start_listening();
        vs.stop_transmitting(); // should not change state
        assert_eq!(vs.state(), VoiceState::Listening);
    }

    #[test]
    fn mute_and_unmute() {
        let mut vs = VoiceStream::new();
        assert!(!vs.is_muted());
        vs.mute();
        assert!(vs.is_muted());
        vs.unmute();
        assert!(!vs.is_muted());
    }

    #[test]
    fn mute_does_not_affect_state() {
        let mut vs = VoiceStream::new();
        vs.start_listening();
        vs.mute();
        assert_eq!(vs.state(), VoiceState::Listening);
        assert!(vs.is_active());
    }

    #[test]
    fn idle_is_not_active() {
        let vs = VoiceStream::new();
        assert!(!vs.is_active());
    }

    #[test]
    fn listening_is_active() {
        let mut vs = VoiceStream::new();
        vs.start_listening();
        assert!(vs.is_active());
    }

    #[test]
    fn transmitting_is_active() {
        let mut vs = VoiceStream::new();
        vs.start_transmitting();
        assert!(vs.is_active());
    }

    #[test]
    fn default_equals_new() {
        let a = VoiceStream::new();
        let b = VoiceStream::default();
        assert_eq!(a.state(), b.state());
        assert_eq!(a.is_muted(), b.is_muted());
    }

    #[test]
    fn transition_listening_to_transmitting() {
        let mut vs = VoiceStream::new();
        vs.start_listening();
        assert_eq!(vs.state(), VoiceState::Listening);
        // Direct transition from listening to transmitting should work
        vs.start_transmitting();
        assert_eq!(vs.state(), VoiceState::Transmitting);
    }

    #[test]
    fn transition_transmitting_to_listening() {
        let mut vs = VoiceStream::new();
        vs.start_transmitting();
        // Direct transition from transmitting to listening should work
        vs.start_listening();
        assert_eq!(vs.state(), VoiceState::Listening);
    }

    #[test]
    fn mute_persists_across_state_changes() {
        let mut vs = VoiceStream::new();
        vs.mute();
        assert!(vs.is_muted());

        vs.start_listening();
        assert!(vs.is_muted(), "mute should persist through state changes");

        vs.stop_listening();
        assert!(vs.is_muted(), "mute should persist after stopping");

        vs.start_transmitting();
        assert!(vs.is_muted(), "mute should persist through transmitting");
    }
}

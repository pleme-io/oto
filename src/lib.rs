//! Oto (音) — audio framework for pleme-io applications.
//!
//! Shared audio primitives for music playback and voice communication:
//! - [`AudioDevice`] / [`AudioDeviceProvider`]: output device enumeration and selection
//! - [`Player`] / [`PlaybackState`]: playback state machine with transport controls
//! - [`Queue`] / [`QueueManager`]: ordered playlist with repeat modes
//! - [`AudioCodec`] / [`AudioInfo`]: codec detection and audio metadata
//! - [`VoiceStream`] / [`VoiceState`]: voice capture/transmit state machine
//! - [`PlaybackBackend`] / [`MockBackend`] / [`Engine`]: the injectable
//!   audio-output and decode seam (peer of [`AudioDeviceProvider`]) plus the
//!   headless engine wiring the pure [`Player`] FSM to it. Enable the
//!   `backend-rodio` feature for [`RodioBackend`], the real rodio + symphonia
//!   implementation.

pub mod backend;
pub mod decoder;
pub mod device;
pub mod error;
pub mod player;
pub mod queue;
#[cfg(feature = "backend-rodio")]
pub mod rodio_backend;
pub mod voice;

pub use backend::{Engine, MockBackend, PlaybackBackend, TickOutcome};
pub use decoder::{AudioCodec, AudioInfo};
pub use device::{AudioDevice, AudioDeviceProvider, MockDeviceProvider};
pub use error::OtoError;
pub use player::{PlaybackState, Player};
pub use queue::{Queue, QueueManager, RepeatMode};
#[cfg(feature = "backend-rodio")]
pub use rodio_backend::RodioBackend;
pub use voice::{VoiceState, VoiceStream};

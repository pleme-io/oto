//! Oto (音) — audio framework for pleme-io applications.
//!
//! Shared audio primitives for music playback and voice communication:
//! - [`AudioDevice`] / [`AudioDeviceProvider`]: output device enumeration and selection
//! - [`Player`] / [`PlaybackState`]: playback state machine with transport controls
//! - [`Queue`] / [`QueueManager`]: ordered playlist with repeat modes
//! - [`AudioCodec`] / [`AudioInfo`]: codec detection and audio metadata
//! - [`VoiceStream`] / [`VoiceState`]: voice capture/transmit state machine

pub mod decoder;
pub mod device;
pub mod error;
pub mod player;
pub mod queue;
pub mod voice;

pub use decoder::{AudioCodec, AudioInfo};
pub use device::{AudioDevice, AudioDeviceProvider, MockDeviceProvider};
pub use error::OtoError;
pub use player::{PlaybackState, Player};
pub use queue::{Queue, QueueManager, RepeatMode};
pub use voice::{VoiceState, VoiceStream};

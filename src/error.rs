/// Errors produced by the oto audio framework.
#[derive(Debug, thiserror::Error)]
pub enum OtoError {
    #[error("device not found: {0}")]
    DeviceNotFound(String),

    #[error("playback failed: {0}")]
    PlaybackFailed(String),

    #[error("decoder error: {0}")]
    DecoderError(String),

    #[error("queue is empty")]
    QueueEmpty,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_not_found_display() {
        let err = OtoError::DeviceNotFound("headphones".into());
        assert_eq!(err.to_string(), "device not found: headphones");
    }

    #[test]
    fn playback_failed_display() {
        let err = OtoError::PlaybackFailed("buffer underrun".into());
        assert_eq!(err.to_string(), "playback failed: buffer underrun");
    }

    #[test]
    fn decoder_error_display() {
        let err = OtoError::DecoderError("invalid header".into());
        assert_eq!(err.to_string(), "decoder error: invalid header");
    }

    #[test]
    fn queue_empty_display() {
        let err = OtoError::QueueEmpty;
        assert_eq!(err.to_string(), "queue is empty");
    }

    #[test]
    fn io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let err = OtoError::from(io_err);
        assert!(err.to_string().contains("file missing"));
    }
}

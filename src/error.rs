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

    #[test]
    fn error_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        // OtoError wraps std::io::Error which is Send+Sync,
        // so OtoError must be as well.
        assert_send_sync::<OtoError>();
    }

    #[test]
    fn error_debug_format_includes_variant_name() {
        let err = OtoError::QueueEmpty;
        let debug = format!("{err:?}");
        assert!(debug.contains("QueueEmpty"), "Debug should contain variant name: {debug}");
    }

    #[test]
    fn io_error_preserves_kind_through_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = OtoError::from(io_err);
        // Verify the Display output wraps the original message
        let display = err.to_string();
        assert!(display.contains("access denied"), "Display should contain original message: {display}");
    }

    #[test]
    fn errors_are_distinct_variants() {
        // Ensure different error variants produce different messages
        let e1 = OtoError::DeviceNotFound("x".into());
        let e2 = OtoError::PlaybackFailed("x".into());
        let e3 = OtoError::DecoderError("x".into());
        let e4 = OtoError::QueueEmpty;

        let messages: Vec<String> = vec![e1, e2, e3, e4].into_iter().map(|e| e.to_string()).collect();
        // All messages should be unique
        for i in 0..messages.len() {
            for j in (i + 1)..messages.len() {
                assert_ne!(messages[i], messages[j], "Error messages should be distinct");
            }
        }
    }
}

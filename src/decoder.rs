/// Supported audio codecs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AudioCodec {
    Flac,
    Alac,
    Wav,
    Mp3,
    Aac,
    Ogg,
    Unknown,
}

impl AudioCodec {
    /// Determine codec from a file extension.
    #[must_use]
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "flac" => Self::Flac,
            "alac" | "m4a" => Self::Alac,
            "wav" => Self::Wav,
            "mp3" => Self::Mp3,
            "aac" => Self::Aac,
            "ogg" | "oga" => Self::Ogg,
            _ => Self::Unknown,
        }
    }

    /// Determine codec from a MIME type.
    #[must_use]
    pub fn from_mime(mime: &str) -> Self {
        match mime {
            "audio/flac" | "audio/x-flac" => Self::Flac,
            "audio/mp4" | "audio/x-m4a" => Self::Alac,
            "audio/wav" | "audio/x-wav" | "audio/wave" => Self::Wav,
            "audio/mpeg" | "audio/mp3" => Self::Mp3,
            "audio/aac" | "audio/x-aac" => Self::Aac,
            "audio/ogg" | "audio/vorbis" => Self::Ogg,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for AudioCodec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Flac => write!(f, "FLAC"),
            Self::Alac => write!(f, "ALAC"),
            Self::Wav => write!(f, "WAV"),
            Self::Mp3 => write!(f, "MP3"),
            Self::Aac => write!(f, "AAC"),
            Self::Ogg => write!(f, "OGG"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Metadata about an audio file's encoding.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioInfo {
    pub codec: AudioCodec,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_secs: f64,
    pub bit_depth: Option<u16>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_from_extension_flac() {
        assert_eq!(AudioCodec::from_extension("flac"), AudioCodec::Flac);
        assert_eq!(AudioCodec::from_extension("FLAC"), AudioCodec::Flac);
    }

    #[test]
    fn codec_from_extension_alac() {
        assert_eq!(AudioCodec::from_extension("alac"), AudioCodec::Alac);
        assert_eq!(AudioCodec::from_extension("m4a"), AudioCodec::Alac);
    }

    #[test]
    fn codec_from_extension_wav() {
        assert_eq!(AudioCodec::from_extension("wav"), AudioCodec::Wav);
    }

    #[test]
    fn codec_from_extension_mp3() {
        assert_eq!(AudioCodec::from_extension("mp3"), AudioCodec::Mp3);
    }

    #[test]
    fn codec_from_extension_aac() {
        assert_eq!(AudioCodec::from_extension("aac"), AudioCodec::Aac);
    }

    #[test]
    fn codec_from_extension_ogg() {
        assert_eq!(AudioCodec::from_extension("ogg"), AudioCodec::Ogg);
        assert_eq!(AudioCodec::from_extension("oga"), AudioCodec::Ogg);
    }

    #[test]
    fn codec_from_extension_unknown() {
        assert_eq!(AudioCodec::from_extension("txt"), AudioCodec::Unknown);
        assert_eq!(AudioCodec::from_extension(""), AudioCodec::Unknown);
    }

    #[test]
    fn codec_from_mime_flac() {
        assert_eq!(AudioCodec::from_mime("audio/flac"), AudioCodec::Flac);
        assert_eq!(AudioCodec::from_mime("audio/x-flac"), AudioCodec::Flac);
    }

    #[test]
    fn codec_from_mime_wav() {
        assert_eq!(AudioCodec::from_mime("audio/wav"), AudioCodec::Wav);
        assert_eq!(AudioCodec::from_mime("audio/x-wav"), AudioCodec::Wav);
        assert_eq!(AudioCodec::from_mime("audio/wave"), AudioCodec::Wav);
    }

    #[test]
    fn codec_from_mime_mp3() {
        assert_eq!(AudioCodec::from_mime("audio/mpeg"), AudioCodec::Mp3);
        assert_eq!(AudioCodec::from_mime("audio/mp3"), AudioCodec::Mp3);
    }

    #[test]
    fn codec_from_mime_aac() {
        assert_eq!(AudioCodec::from_mime("audio/aac"), AudioCodec::Aac);
    }

    #[test]
    fn codec_from_mime_ogg() {
        assert_eq!(AudioCodec::from_mime("audio/ogg"), AudioCodec::Ogg);
        assert_eq!(AudioCodec::from_mime("audio/vorbis"), AudioCodec::Ogg);
    }

    #[test]
    fn codec_from_mime_unknown() {
        assert_eq!(AudioCodec::from_mime("text/plain"), AudioCodec::Unknown);
    }

    #[test]
    fn codec_display() {
        assert_eq!(AudioCodec::Flac.to_string(), "FLAC");
        assert_eq!(AudioCodec::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn audio_info_construction() {
        let info = AudioInfo {
            codec: AudioCodec::Flac,
            sample_rate: 44100,
            channels: 2,
            duration_secs: 245.5,
            bit_depth: Some(24),
        };
        assert_eq!(info.codec, AudioCodec::Flac);
        assert_eq!(info.sample_rate, 44100);
        assert_eq!(info.channels, 2);
        assert!((info.duration_secs - 245.5).abs() < f64::EPSILON);
        assert_eq!(info.bit_depth, Some(24));
    }

    #[test]
    fn codec_serde_roundtrip() {
        let codecs = [
            AudioCodec::Flac,
            AudioCodec::Alac,
            AudioCodec::Wav,
            AudioCodec::Mp3,
            AudioCodec::Aac,
            AudioCodec::Ogg,
            AudioCodec::Unknown,
        ];
        for codec in &codecs {
            let json = serde_json::to_string(codec).expect("serialize");
            let back: AudioCodec = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*codec, back, "roundtrip failed for {codec}");
        }
    }

    #[test]
    fn audio_info_serde_roundtrip() {
        let info = AudioInfo {
            codec: AudioCodec::Ogg,
            sample_rate: 96000,
            channels: 6,
            duration_secs: 3723.456,
            bit_depth: None,
        };
        let json = serde_json::to_string(&info).expect("serialize");
        let back: AudioInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.codec, AudioCodec::Ogg);
        assert_eq!(back.sample_rate, 96000);
        assert_eq!(back.channels, 6);
        assert!((back.duration_secs - 3723.456).abs() < f64::EPSILON);
        assert_eq!(back.bit_depth, None);
    }

    #[test]
    fn codec_from_extension_case_insensitive() {
        // Extensions should be case-insensitive
        assert_eq!(AudioCodec::from_extension("MP3"), AudioCodec::Mp3);
        assert_eq!(AudioCodec::from_extension("Ogg"), AudioCodec::Ogg);
        assert_eq!(AudioCodec::from_extension("WAV"), AudioCodec::Wav);
        assert_eq!(AudioCodec::from_extension("AaC"), AudioCodec::Aac);
        assert_eq!(AudioCodec::from_extension("M4A"), AudioCodec::Alac);
    }

    #[test]
    fn codec_from_mime_alac() {
        assert_eq!(AudioCodec::from_mime("audio/mp4"), AudioCodec::Alac);
        assert_eq!(AudioCodec::from_mime("audio/x-m4a"), AudioCodec::Alac);
    }

    #[test]
    fn codec_from_mime_x_aac() {
        assert_eq!(AudioCodec::from_mime("audio/x-aac"), AudioCodec::Aac);
    }

    #[test]
    fn codec_display_all_variants() {
        assert_eq!(AudioCodec::Alac.to_string(), "ALAC");
        assert_eq!(AudioCodec::Wav.to_string(), "WAV");
        assert_eq!(AudioCodec::Mp3.to_string(), "MP3");
        assert_eq!(AudioCodec::Aac.to_string(), "AAC");
        assert_eq!(AudioCodec::Ogg.to_string(), "OGG");
    }

    #[test]
    fn audio_info_bit_depth_some() {
        let info = AudioInfo {
            codec: AudioCodec::Wav,
            sample_rate: 44100,
            channels: 1,
            duration_secs: 0.0,
            bit_depth: Some(16),
        };
        assert_eq!(info.bit_depth, Some(16));
    }

    #[test]
    fn codec_clone_and_copy() {
        let codec = AudioCodec::Flac;
        let cloned = codec.clone();
        let copied = codec; // Copy
        assert_eq!(codec, cloned);
        assert_eq!(codec, copied);
    }
}

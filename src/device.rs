use crate::error::OtoError;

/// Represents an audio output device.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioDevice {
    /// Create a new audio device descriptor.
    #[must_use]
    pub fn new(id: String, name: String, is_default: bool, sample_rate: u32, channels: u16) -> Self {
        Self {
            id,
            name,
            is_default,
            sample_rate,
            channels,
        }
    }
}

/// Trait for enumerating audio output devices.
///
/// Implementations provide platform-specific device discovery.
/// Use `MockDeviceProvider` for testing without hardware.
pub trait AudioDeviceProvider {
    /// List all available output devices.
    fn list_output_devices(&self) -> Result<Vec<AudioDevice>, OtoError>;

    /// Get the default output device.
    fn default_output_device(&self) -> Result<AudioDevice, OtoError>;
}

/// Mock device provider for headless testing.
#[derive(Debug, Default)]
pub struct MockDeviceProvider {
    devices: Vec<AudioDevice>,
}

impl MockDeviceProvider {
    /// Create an empty mock provider.
    #[must_use]
    pub fn new() -> Self {
        Self { devices: vec![] }
    }

    /// Create a mock provider with a default device already present.
    #[must_use]
    pub fn with_default_device() -> Self {
        Self {
            devices: vec![AudioDevice::new(
                "mock-default".into(),
                "Mock Output".into(),
                true,
                44100,
                2,
            )],
        }
    }

    /// Add a device to the mock provider.
    pub fn add_device(&mut self, device: AudioDevice) {
        self.devices.push(device);
    }
}

impl AudioDeviceProvider for MockDeviceProvider {
    fn list_output_devices(&self) -> Result<Vec<AudioDevice>, OtoError> {
        Ok(self.devices.clone())
    }

    fn default_output_device(&self) -> Result<AudioDevice, OtoError> {
        self.devices
            .iter()
            .find(|d| d.is_default)
            .cloned()
            .ok_or_else(|| OtoError::DeviceNotFound("no default device".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_device_construction() {
        let dev = AudioDevice::new("hw:0".into(), "Speakers".into(), true, 48000, 2);
        assert_eq!(dev.id, "hw:0");
        assert_eq!(dev.name, "Speakers");
        assert!(dev.is_default);
        assert_eq!(dev.sample_rate, 48000);
        assert_eq!(dev.channels, 2);
    }

    #[test]
    fn mock_provider_empty() {
        let provider = MockDeviceProvider::new();
        let devices = provider.list_output_devices().unwrap();
        assert!(devices.is_empty());
    }

    #[test]
    fn mock_provider_no_default_returns_error() {
        let provider = MockDeviceProvider::new();
        let result = provider.default_output_device();
        assert!(result.is_err());
    }

    #[test]
    fn mock_provider_with_default_device() {
        let provider = MockDeviceProvider::with_default_device();
        let devices = provider.list_output_devices().unwrap();
        assert_eq!(devices.len(), 1);
        assert!(devices[0].is_default);

        let default = provider.default_output_device().unwrap();
        assert_eq!(default.name, "Mock Output");
        assert_eq!(default.sample_rate, 44100);
    }

    #[test]
    fn mock_provider_add_device() {
        let mut provider = MockDeviceProvider::new();
        provider.add_device(AudioDevice::new("usb-1".into(), "USB DAC".into(), false, 96000, 2));
        provider.add_device(AudioDevice::new("built-in".into(), "Built-in".into(), true, 44100, 2));

        let devices = provider.list_output_devices().unwrap();
        assert_eq!(devices.len(), 2);

        let default = provider.default_output_device().unwrap();
        assert_eq!(default.name, "Built-in");
    }

    #[test]
    fn mock_provider_default_selects_first_default() {
        let mut provider = MockDeviceProvider::new();
        provider.add_device(AudioDevice::new("a".into(), "A".into(), true, 44100, 2));
        provider.add_device(AudioDevice::new("b".into(), "B".into(), true, 48000, 2));

        let default = provider.default_output_device().unwrap();
        assert_eq!(default.id, "a");
    }

    #[test]
    fn audio_device_serde_roundtrip() {
        let dev = AudioDevice::new("usb-2".into(), "USB DAC Pro".into(), false, 192_000, 8);
        let json = serde_json::to_string(&dev).expect("serialize");
        let back: AudioDevice = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.id, "usb-2");
        assert_eq!(back.name, "USB DAC Pro");
        assert!(!back.is_default);
        assert_eq!(back.sample_rate, 192_000);
        assert_eq!(back.channels, 8);
    }

    #[test]
    fn mock_provider_no_default_among_non_default_devices() {
        let mut provider = MockDeviceProvider::new();
        provider.add_device(AudioDevice::new("a".into(), "A".into(), false, 44100, 2));
        provider.add_device(AudioDevice::new("b".into(), "B".into(), false, 48000, 2));

        let result = provider.default_output_device();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("no default device"));
    }

    #[test]
    fn audio_device_provider_trait_object_safety() {
        // Verify AudioDeviceProvider can be used as a trait object
        let provider: Box<dyn AudioDeviceProvider> = Box::new(MockDeviceProvider::with_default_device());
        let devices = provider.list_output_devices().unwrap();
        assert_eq!(devices.len(), 1);
    }
}

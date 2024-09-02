use std::fmt::Display;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

// Errors
#[derive(Debug)]
pub enum BuzzerError {
    NoOutputDevice,
    NoAvaliableConfigs,
}
impl Display for BuzzerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoOutputDevice => write!(f, "No output device was found"),
            Self::NoAvaliableConfigs => write!(f, "Unable to fetch a stream config")
        }
    }
}

/// Buzzer
/// FIXME: Playing biiip may "click" because first sample in the stream buffer != 0,
///        so i need to somehow reset the stream buffer, before playing the biiip
pub struct Buzzer {
    device: cpal::Device,
    config: cpal::StreamConfig,
    stream: Option<cpal::Stream>,
    pub muted: bool,
    playing: bool,
}
impl Buzzer {
    pub fn new() -> Result<Self, BuzzerError> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or(BuzzerError::NoOutputDevice)?;

        let mut supported_configs_range = device.supported_output_configs()
            .map_err(|_| BuzzerError::NoAvaliableConfigs)?;
        let supported_config = supported_configs_range.next()
            .ok_or(BuzzerError::NoAvaliableConfigs)?
            .with_max_sample_rate();

        Ok(Self {
            device,
            config: supported_config.config(),
            stream: None,
            muted: false,
            playing: false,
        })
    }

    pub fn set_muted(&mut self, state: bool) {
        self.muted = state;
        if state {
            self.set_playing(false);
        }
    }
    pub fn set_playing(&mut self, state: bool) {
        // Do nothing if the state hasn't changed
        if self.playing == state { return; }
        // Do nothing if trying to enable playing while muted
        if state && self.muted { return; }

        self.playing = state;
        if state {
            // Create a stream if not already created
            // Stream starts playing on creation and i cant immediately pause it
            if self.stream.is_none() {
                self.stream = Some(self.device.build_output_stream(
                    &self.config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        for (index, sample) in data.iter_mut().enumerate() {
                            // Generate sine wave, so our biiiip will be quite smooth
                            let val = ((index as f32 / 300.0).cos() + 1.0) / 2.0 * 4.0;

                            *sample = val;
                        }
                    },
                    // FIXME: Just print the error into the console for now
                    |err| eprintln!("Buzzer runtime error: {}", err),
                    None
                ).unwrap());
            }

            let _ = self.stream.as_ref().unwrap().play();
        } else {
            if let Some(stream) = &self.stream {
                let _ = stream.pause();
            }
        };
    }
}

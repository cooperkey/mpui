use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use crossbeam_channel::Receiver;
use parking_lot::Mutex;
use std::sync::Arc;

/// Audio output stream wrapping cpal.
pub struct AudioOutput {
    _stream: Stream,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioOutput {
    /// Create a new audio output stream that reads samples from `rx`.
    ///
    /// `rx` receives chunks of interleaved f32 samples from the player thread.
    /// The cpal callback pulls from a shared buffer fed by those chunks.
    pub fn new(
        sample_rate: Option<u32>,
        channels: Option<u16>,
        sample_rx: Receiver<Vec<f32>>,
    ) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no audio output device found")?;

        let (def_channels, def_sample_rate) = if channels.is_some() && sample_rate.is_some() {
            (2, 48000)
        } else {
            #[cfg(target_os = "android")]
            {
                (2, 48000)
            }
            #[cfg(not(target_os = "android"))]
            {
                match device.default_output_config() {
                    Ok(conf) => (conf.channels(), conf.sample_rate().0),
                    Err(_) => (2, 44100),
                }
            }
        };

        let config = StreamConfig {
            channels: channels.unwrap_or(def_channels),
            sample_rate: cpal::SampleRate(sample_rate.unwrap_or(def_sample_rate)),
            buffer_size: cpal::BufferSize::Default,
        };

        let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let buf_clone = buffer.clone();

        // Drain channel into buffer in a helper closure.
        let fill_buffer = move |needed: usize| {
            let mut buf = buf_clone.lock();
            while buf.len() < needed {
                match sample_rx.try_recv() {
                    Ok(chunk) => buf.extend_from_slice(&chunk),
                    Err(_) => break,
                }
            }
        };

        let buf_out = buffer.clone();
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                fill_buffer(data.len());
                let mut buf = buf_out.lock();
                let available = buf.len().min(data.len());
                data[..available].copy_from_slice(&buf[..available]);
                // Silence any remainder.
                for s in &mut data[available..] {
                    *s = 0.0;
                }
                buf.drain(..available);
            },
            |err| {
                tracing::error!("audio output error: {err}");
            },
            None,
        )?;

        stream.play()?;

        let sample_rate = config.sample_rate.0;
        let channels = config.channels;

        Ok(Self {
            _stream: stream,
            sample_rate,
            channels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_config() {
        let host = cpal::default_host();
        if let Some(device) = host.default_output_device() {
            let config = StreamConfig {
                channels: 2,
                sample_rate: cpal::SampleRate(48000),
                buffer_size: cpal::BufferSize::Default,
            };
            let stream = device.build_output_stream(
                &config,
                move |_data: &mut [f32], _| {},
                |err| eprintln!("err: {err}"),
                None,
            );
            println!("Stream build result: {:?}", stream.is_ok());
            assert!(stream.is_ok());
        }
    }

    #[test]
    fn test_audio_output_new() {
        let (_tx, rx) = crossbeam_channel::unbounded();
        let out = AudioOutput::new(None, None, rx);
        println!("AudioOutput::new result: {:?}", out.is_ok());
        assert!(out.is_ok());
    }
}

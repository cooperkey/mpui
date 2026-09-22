use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use crossbeam_channel::Receiver;
use parking_lot::Mutex;
use std::sync::Arc;

use std::collections::VecDeque;

/// Audio output stream wrapping cpal.
pub struct AudioOutput {
    stream: Stream,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer: Arc<Mutex<VecDeque<f32>>>,
}

impl AudioOutput {
    /// Create a new audio output stream that reads samples from `rx`.
    ///
    /// `rx` receives chunks of interleaved f32 samples from the player thread.
    /// The cpal callback pulls from a shared ringbuffer fed by those chunks.
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

        let buffer: Arc<Mutex<VecDeque<f32>>> = Arc::new(Mutex::new(VecDeque::with_capacity(8192)));
        let buf_out = buffer.clone();

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut buf = buf_out.lock();
                while buf.len() < data.len() {
                    match sample_rx.try_recv() {
                        Ok(chunk) => buf.extend(chunk),
                        Err(_) => break,
                    }
                }
                for sample in data.iter_mut() {
                    *sample = buf.pop_front().unwrap_or(0.0);
                }
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
            stream,
            sample_rate,
            channels,
            buffer,
        })
    }

    /// Pause output stream playback at the hardware device level.
    pub fn pause(&self) -> Result<()> {
        self.stream.pause().context("failed to pause audio stream")
    }

    /// Resume output stream playback at the hardware device level.
    pub fn resume(&self) -> Result<()> {
        self.stream.play().context("failed to resume audio stream")
    }

    /// Flush any pending samples in the output buffer.
    pub fn flush(&self) {
        self.buffer.lock().clear();
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

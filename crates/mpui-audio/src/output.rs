use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use crossbeam_channel::Receiver;
use parking_lot::Mutex;
use std::sync::Arc;

/// Audio output stream wrapping cpal.
pub struct AudioOutput {
    _stream: Stream,
}

impl AudioOutput {
    /// Create a new audio output stream that reads samples from `rx`.
    ///
    /// `rx` receives chunks of interleaved f32 samples from the player thread.
    /// The cpal callback pulls from a shared buffer fed by those chunks.
    pub fn new(
        sample_rate: u32,
        channels: u16,
        sample_rx: Receiver<Vec<f32>>,
    ) -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("no audio output device found")?;

        let config = StreamConfig {
            channels,
            sample_rate: cpal::SampleRate(sample_rate),
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

        Ok(Self { _stream: stream })
    }
}

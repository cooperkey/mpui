use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use crossbeam_channel::Receiver;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Audio output stream wrapping cpal with a lock-free callback buffer and stream health monitoring.
pub struct AudioOutput {
    stream: Stream,
    pub sample_rate: u32,
    pub channels: u16,
    flush_requested: Arc<AtomicBool>,
    is_alive: Arc<AtomicBool>,
    buffered_count: Arc<AtomicUsize>,
    last_callback_micros: Arc<AtomicU64>,
    start_instant: Instant,
}

impl AudioOutput {
    /// Create a new audio output stream that reads samples from `sample_rx`.
    ///
    /// The cpal callback pulls from a lock-free SPSC ringbuffer without mutex contention or allocations.
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

        let is_alive = Arc::new(AtomicBool::new(true));
        let is_alive_cb = is_alive.clone();
        let is_alive_err = is_alive.clone();

        let flush_requested = Arc::new(AtomicBool::new(false));
        let flush_requested_cb = flush_requested.clone();

        let buffered_count = Arc::new(AtomicUsize::new(0));
        let buffered_count_cb = buffered_count.clone();

        let start_instant = Instant::now();
        let last_callback_micros = Arc::new(AtomicU64::new(0));
        let last_callback_cb = last_callback_micros.clone();

        let mut buf = VecDeque::<f32>::with_capacity(32768);

        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let now_micros = start_instant.elapsed().as_micros() as u64;
                last_callback_cb.store(now_micros, Ordering::Relaxed);
                is_alive_cb.store(true, Ordering::Relaxed);

                if flush_requested_cb.swap(false, Ordering::SeqCst) {
                    buf.clear();
                    while sample_rx.try_recv().is_ok() {}
                    buffered_count_cb.store(0, Ordering::Release);
                }

                while buf.len() < data.len() {
                    match sample_rx.try_recv() {
                        Ok(chunk) => buf.extend(chunk),
                        Err(_) => break,
                    }
                }

                let available = data.len().min(buf.len());
                let mut idx = 0;
                for s in buf.drain(..available) {
                    data[idx] = s;
                    idx += 1;
                }
                data[idx..].fill(0.0);
                buffered_count_cb.store(buf.len(), Ordering::Release);
            },
            move |err| {
                tracing::error!("audio output error: {err}");
                is_alive_err.store(false, Ordering::SeqCst);
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
            flush_requested,
            is_alive,
            buffered_count,
            last_callback_micros,
            start_instant,
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

    /// Flush any pending samples in the output ringbuffer and incoming channel.
    pub fn flush(&self) {
        self.flush_requested.store(true, Ordering::SeqCst);
        self.buffered_count.store(0, Ordering::Release);
    }

    /// Check if the output ringbuffer is completely drained.
    pub fn is_buffer_empty(&self) -> bool {
        self.buffered_count.load(Ordering::Acquire) == 0
    }

    /// Returns whether the audio stream is currently healthy and active.
    pub fn is_healthy(&self) -> bool {
        self.is_alive.load(Ordering::SeqCst)
    }

    /// Duration elapsed since the last audio callback executed.
    pub fn last_callback_elapsed(&self) -> Duration {
        let last = self.last_callback_micros.load(Ordering::Relaxed);
        let cur = self.start_instant.elapsed().as_micros() as u64;
        Duration::from_micros(cur.saturating_sub(last))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_config() {
        let host = cpal::default_host();
        if let Some(device) = host.default_output_device() {
            if let Ok(conf) = device.default_output_config() {
                println!("DEFAULT OUTPUT CONFIG: channels={}, sample_rate={}", conf.channels(), conf.sample_rate().0);
            }
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
        assert!(out.is_ok());
    }

    #[test]
    fn test_callback_buffer_logic() {
        let (sample_tx, sample_rx) = crossbeam_channel::unbounded::<Vec<f32>>();
        let mut buf = VecDeque::<f32>::with_capacity(32768);

        // Send 40 chunks of 2304 samples (92,160 samples)
        for _ in 0..40 {
            let _ = sample_tx.send(vec![1.0; 2304]);
        }

        let mut data = vec![0.0f32; 512]; // typical cpal buffer size
        let mut total_samples_read = 0;

        // Run 100 callbacks
        for _ in 0..100 {
            while buf.len() < data.len() {
                match sample_rx.try_recv() {
                    Ok(chunk) => buf.extend(chunk),
                    Err(_) => break,
                }
            }

            let available = data.len().min(buf.len());
            let mut idx = 0;
            for s in buf.drain(..available) {
                data[idx] = s;
                idx += 1;
            }
            data[idx..].fill(0.0);
            total_samples_read += available;
        }

        while let Ok(chunk) = sample_rx.try_recv() {
            buf.extend(chunk);
        }

        let total_accounted = total_samples_read + buf.len();
        assert_eq!(total_accounted, 40 * 2304);
    }
}

use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;

use crate::visualizer::SpectrumAnalyzer;
use crate::decoder::StreamingDecoder;
use crate::equalizer::Equalizer;

/// Commands sent from the UI thread to the player.
pub enum PlayerCommand {
    Play(std::path::PathBuf),
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    SetVolume(f32),
    SetEqBand { band: usize, gain: f32 },
    SetEqPreset([f32; 10]),
    Quit,
}

/// State exposed to the UI thread.
#[derive(Debug, Clone)]
pub struct PlaybackState {
    pub playing: bool,
    pub paused: bool,
    pub position: Duration,
    pub duration: Duration,
    pub volume: f32,
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self {
            playing: false,
            paused: false,
            position: Duration::ZERO,
            duration: Duration::ZERO,
            volume: 0.8,
            sample_rate: 44100,
            channels: 2,
        }
    }
}

/// Top-level player that owns the audio pipeline.
pub struct Player {
    pub state: Arc<Mutex<PlaybackState>>,
    pub command_tx: Sender<PlayerCommand>,
    command_rx: Receiver<PlayerCommand>,
    sample_tx: Sender<Vec<f32>>,
    sample_rx: Receiver<Vec<f32>>,
    pub spectrum: Arc<Mutex<Vec<f32>>>,
}

impl Player {
    pub fn new() -> Self {
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let (sample_tx, sample_rx) = crossbeam_channel::bounded(64);
        let analyzer = SpectrumAnalyzer::new(2048);
        let spectrum = analyzer.spectrum_handle();

        Self {
            state: Arc::new(Mutex::new(PlaybackState::default())),
            command_tx,
            command_rx,
            sample_tx,
            sample_rx,
            spectrum,
        }
    }

    pub fn run(self) {
        let mut decoder: Option<StreamingDecoder> = None;
        let mut current_samples_decoded: u64 = 0;
        let mut analyzer = SpectrumAnalyzer::new(2048);
        let mut equalizer = Equalizer::new(44100, 2);
        let mut audio_output = crate::output::AudioOutput::new(None, None, self.sample_rx.clone()).ok();
        let mut active_out_rate = audio_output.as_ref().map(|o| o.sample_rate).unwrap_or(44100);
        let mut active_out_channels = audio_output.as_ref().map(|o| o.channels as usize).unwrap_or(2);
        
        loop {
            let cmd = if decoder.is_some() && !self.state.lock().paused {
                self.command_rx.try_recv().ok()
            } else {
                self.command_rx.recv().ok()
            };

            let mut quit = false;
            
            let mut handle_cmd = |cmd: PlayerCommand| {
                match cmd {
                    PlayerCommand::Play(path) => {
                        if let Ok(dec) = StreamingDecoder::new(&path) {
                            let (duration, sample_rate, channels) = (dec.total_duration, dec.sample_rate, dec.channels);
                            decoder = Some(dec);
                            current_samples_decoded = 0;
                            let mut state = self.state.lock();
                            state.playing = true;
                            state.paused = false;
                            state.position = Duration::ZERO;

                            // Recreate audio output stream to match track sample rate and channels if different
                            if audio_output.as_ref().map(|o| (o.sample_rate, o.channels as usize)) != Some((sample_rate, channels)) {
                                if let Ok(new_output) = crate::output::AudioOutput::new(Some(sample_rate), Some(channels as u16), self.sample_rx.clone()) {
                                    active_out_rate = new_output.sample_rate;
                                    active_out_channels = new_output.channels as usize;
                                    audio_output = Some(new_output);
                                }
                            }
                            
                            let old_gains = *equalizer.gains();
                            let old_enabled = equalizer.is_enabled();
                            equalizer = Equalizer::new(sample_rate, channels);
                            equalizer.set_all_bands(&old_gains);
                            equalizer.set_enabled(old_enabled);
                            
                            state.sample_rate = sample_rate;
                            state.channels = channels as u16;
                            state.duration = if let Some(frames) = duration {
                                Duration::from_secs_f64(frames as f64 / sample_rate as f64)
                            } else {
                                Duration::ZERO
                            };
                        }
                    }
                    PlayerCommand::Pause => {
                        self.state.lock().paused = true;
                    }
                    PlayerCommand::Resume => {
                        self.state.lock().paused = false;
                    }
                    PlayerCommand::Stop => {
                        decoder = None;
                        let mut state = self.state.lock();
                        state.playing = false;
                        state.paused = false;
                        state.position = Duration::ZERO;
                    }
                    PlayerCommand::Seek(duration) => {
                        if let Some(ref mut dec) = decoder {
                            let _ = dec.seek(duration);
                            current_samples_decoded = (duration.as_secs_f64() * dec.sample_rate as f64) as u64;
                            self.state.lock().position = duration;
                        }
                    }
                    PlayerCommand::SetVolume(v) => {
                        self.state.lock().volume = v;
                    }
                    PlayerCommand::SetEqBand { band, gain } => {
                        equalizer.set_band(band, gain);
                    }
                    PlayerCommand::SetEqPreset(gains) => {
                        equalizer.set_all_bands(&gains);
                    }
                    PlayerCommand::Quit => {
                        quit = true;
                    }
                }
            };

            if let Some(c) = cmd {
                handle_cmd(c);
            }
            
            while let Ok(c) = self.command_rx.try_recv() {
                handle_cmd(c);
            }

            if quit {
                break;
            }

            if let Some(ref mut dec) = decoder {
                let paused = self.state.lock().paused;
                if !paused {
                    if let Some(mut frame) = dec.next_frame() {
                        equalizer.process(&mut frame);
                        
                        let volume = self.state.lock().volume;
                        for s in &mut frame {
                            *s *= volume;
                        }
                        
                        let channels = dec.channels;
                        let mut mono = Vec::with_capacity(frame.len() / channels);
                        for chunk in frame.chunks(channels) {
                            let sum: f32 = chunk.iter().sum();
                            mono.push(sum / channels as f32);
                        }
                        analyzer.analyze(&mono);
                        {
                            let mut shared_spec = self.spectrum.lock();
                            let local_spec = analyzer.spectrum.lock();
                            shared_spec.clone_from(&local_spec);
                        }

                        current_samples_decoded += (frame.len() / channels) as u64;
                        self.state.lock().position = Duration::from_secs_f64(current_samples_decoded as f64 / dec.sample_rate as f64);
                        
                        // Channel conversion & sample rate matching
                        let ch_adapted = convert_channels(&frame, channels, active_out_channels);
                        let resampled = resample_linear(&ch_adapted, dec.sample_rate, active_out_rate, active_out_channels);

                        if self.sample_tx.send(resampled).is_err() {
                            break;
                        }
                    } else {
                        decoder = None;
                        let mut state = self.state.lock();
                        state.playing = false;
                        state.position = Duration::ZERO;
                    }
                }
            }
        }
    }

    /// Get the sample receiver for constructing the audio output stream.
    pub fn sample_receiver(&self) -> Receiver<Vec<f32>> {
        self.sample_rx.clone()
    }

    /// Get a clone of the command sender for the UI thread.
    pub fn command_sender(&self) -> Sender<PlayerCommand> {
        self.command_tx.clone()
    }
}

fn convert_channels(input: &[f32], in_channels: usize, out_channels: usize) -> Vec<f32> {
    if in_channels == out_channels || in_channels == 0 || out_channels == 0 {
        return input.to_vec();
    }
    let frames = input.len() / in_channels;
    let mut out = Vec::with_capacity(frames * out_channels);
    for frame in input.chunks_exact(in_channels) {
        if in_channels == 1 && out_channels == 2 {
            out.push(frame[0]);
            out.push(frame[0]);
        } else if in_channels == 2 && out_channels == 1 {
            out.push(0.5 * (frame[0] + frame[1]));
        } else {
            for ch in 0..out_channels {
                out.push(frame[ch % in_channels]);
            }
        }
    }
    out
}

fn resample_linear(input: &[f32], in_rate: u32, out_rate: u32, channels: usize) -> Vec<f32> {
    if in_rate == out_rate || in_rate == 0 || out_rate == 0 || channels == 0 || input.is_empty() {
        return input.to_vec();
    }
    let in_frames = input.len() / channels;
    let ratio = in_rate as f64 / out_rate as f64;
    let out_frames = ((in_frames as f64) / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_frames * channels);

    for out_frame_idx in 0..out_frames {
        let in_pos = out_frame_idx as f64 * ratio;
        let idx0 = in_pos.floor() as usize;
        let frac = (in_pos - idx0 as f64) as f32;
        let idx1 = (idx0 + 1).min(in_frames.saturating_sub(1));

        for ch in 0..channels {
            let s0 = input[idx0 * channels + ch];
            let s1 = input[idx1 * channels + ch];
            out.push(s0 + frac * (s1 - s0));
        }
    }
    out
}

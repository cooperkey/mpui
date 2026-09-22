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
    SetReplayGainMode(String),
    Quit,
}

/// Events emitted by the player to the UI thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerEvent {
    TrackEnded,
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
    pub event_tx: Sender<PlayerEvent>,
    pub event_rx: Receiver<PlayerEvent>,
    pub spectrum: Arc<Mutex<Vec<f32>>>,
}

impl Player {
    pub fn new() -> Self {
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let (sample_tx, sample_rx) = crossbeam_channel::bounded(8);
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        let analyzer = SpectrumAnalyzer::new(2048);
        let spectrum = analyzer.spectrum_handle();

        Self {
            state: Arc::new(Mutex::new(PlaybackState::default())),
            command_tx,
            command_rx,
            sample_tx,
            sample_rx,
            event_tx,
            event_rx,
            spectrum,
        }
    }

    pub fn run(self) {
        let mut decoder: Option<StreamingDecoder> = None;
        let mut current_samples_decoded: u64 = 0;
        let mut analyzer = SpectrumAnalyzer::with_spectrum(2048, self.spectrum.clone());
        let mut equalizer = Equalizer::new(44100, 2);
        let mut resampler = LinearResampler::new();
        let mut audio_output = crate::output::AudioOutput::new(None, None, self.sample_rx.clone()).ok();
        let mut active_out_rate = audio_output.as_ref().map(|o| o.sample_rate).unwrap_or(44100);
        let mut active_out_channels = audio_output.as_ref().map(|o| o.channels as usize).unwrap_or(2);
        let mut replaygain_mode = "track".to_string();
        let mut replaygain_factor: f32 = 1.0;
        
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
                        while self.sample_rx.try_recv().is_ok() {}
                        if let Some(ref out) = audio_output {
                            out.flush();
                            let _ = out.resume();
                        }
                        resampler.reset();
                        if let Ok(dec) = StreamingDecoder::new(&path) {
                            let (duration, sample_rate, channels) = (dec.total_duration, dec.sample_rate, dec.channels);
                            replaygain_factor = calculate_replaygain(&dec, &replaygain_mode);
                            decoder = Some(dec);
                            current_samples_decoded = 0;
                            let mut state = self.state.lock();
                            state.playing = true;
                            state.paused = false;
                            state.position = Duration::ZERO;

                            // Ensure audio output stream exists
                            if audio_output.is_none() {
                                if let Ok(new_output) = crate::output::AudioOutput::new(None, None, self.sample_rx.clone()) {
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
                        if let Some(ref out) = audio_output {
                            let _ = out.pause();
                        }
                    }
                    PlayerCommand::Resume => {
                        self.state.lock().paused = false;
                        if let Some(ref out) = audio_output {
                            let _ = out.resume();
                        }
                    }
                    PlayerCommand::Stop => {
                        decoder = None;
                        let mut state = self.state.lock();
                        state.playing = false;
                        state.paused = false;
                        state.position = Duration::ZERO;
                        self.spectrum.lock().clear();
                        while self.sample_rx.try_recv().is_ok() {}
                        if let Some(ref out) = audio_output {
                            out.flush();
                            let _ = out.pause();
                        }
                    }
                    PlayerCommand::Seek(duration) => {
                        while self.sample_rx.try_recv().is_ok() {}
                        if let Some(ref out) = audio_output {
                            out.flush();
                        }
                        resampler.reset();
                        if let Some(ref mut dec) = decoder {
                            if dec.seek(duration).is_ok() {
                                current_samples_decoded = (duration.as_secs_f64() * dec.sample_rate as f64) as u64;
                                self.state.lock().position = duration;
                            }
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
                    PlayerCommand::SetReplayGainMode(mode) => {
                        replaygain_mode = mode;
                        if let Some(ref dec) = decoder {
                            replaygain_factor = calculate_replaygain(dec, &replaygain_mode);
                        }
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
                        let channels = dec.channels;
                        let sample_rate = dec.sample_rate;

                        // Ensure equalizer parameters match true decoded format
                        if equalizer.channels() != channels || equalizer.sample_rate() != sample_rate {
                            let old_gains = *equalizer.gains();
                            let old_enabled = equalizer.is_enabled();
                            equalizer = Equalizer::new(sample_rate, channels);
                            equalizer.set_all_bands(&old_gains);
                            equalizer.set_enabled(old_enabled);

                            let mut state = self.state.lock();
                            state.sample_rate = sample_rate;
                            state.channels = channels as u16;
                        }

                        // ReplayGain loudness normalization
                        if (replaygain_factor - 1.0).abs() > f32::EPSILON {
                            for s in &mut frame {
                                *s *= replaygain_factor;
                            }
                        }

                        equalizer.process(&mut frame);

                        // Tap post-EQ mono signal for visualizer (pre-volume for consistent dynamics)
                        let mut mono = Vec::with_capacity(frame.len() / channels);
                        for chunk in frame.chunks(channels) {
                            let sum: f32 = chunk.iter().sum();
                            mono.push(sum / channels as f32);
                        }
                        analyzer.analyze(&mono);

                        // Apply master volume attenuation for output
                        let volume = self.state.lock().volume;
                        if (volume - 1.0).abs() > f32::EPSILON {
                            for s in &mut frame {
                                *s *= volume;
                            }
                        }

                        current_samples_decoded += (frame.len() / channels) as u64;
                        self.state.lock().position = Duration::from_secs_f64(current_samples_decoded as f64 / sample_rate as f64);
                        
                        // Channel conversion & sample rate matching (continuous phase interpolation)
                        let to_send = if channels == active_out_channels && sample_rate == active_out_rate {
                            frame
                        } else {
                            let ch_adapted = convert_channels(&frame, channels, active_out_channels);
                            resampler.process(&ch_adapted, sample_rate, active_out_rate, active_out_channels)
                        };

                        if self.sample_tx.send(to_send).is_err() {
                            break;
                        }
                    } else {
                        // Allow remaining buffered samples to output so track ends naturally
                        let mut interrupted = false;
                        while !self.sample_tx.is_empty() {
                            if !self.command_rx.is_empty() {
                                interrupted = true;
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        if !interrupted {
                            if let Some(ref out) = audio_output {
                                while !out.buffer.lock().is_empty() {
                                    if !self.command_rx.is_empty() {
                                        interrupted = true;
                                        break;
                                    }
                                    std::thread::sleep(Duration::from_millis(10));
                                }
                            }
                        }

                        if !interrupted {
                            decoder = None;
                            let mut state = self.state.lock();
                            state.playing = false;
                            state.position = Duration::ZERO;
                            self.spectrum.lock().clear();
                            let _ = self.event_tx.send(PlayerEvent::TrackEnded);
                        }
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

    /// Get a receiver for player events (e.g. track end).
    pub fn event_receiver(&self) -> Receiver<PlayerEvent> {
        self.event_rx.clone()
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

#[derive(Default)]
struct LinearResampler {
    phase: f64,
}

impl LinearResampler {
    fn new() -> Self {
        Self { phase: 0.0 }
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn process(&mut self, input: &[f32], in_rate: u32, out_rate: u32, channels: usize) -> Vec<f32> {
        if in_rate == out_rate || in_rate == 0 || out_rate == 0 || channels == 0 || input.is_empty() {
            self.phase = 0.0;
            return input.to_vec();
        }
        let in_frames = input.len() / channels;
        let step = in_rate as f64 / out_rate as f64;
        let mut out = Vec::new();

        while self.phase < in_frames as f64 {
            let idx0 = self.phase.floor() as usize;
            let frac = (self.phase - idx0 as f64) as f32;
            let idx1 = (idx0 + 1).min(in_frames - 1);

            for ch in 0..channels {
                let s0 = input[idx0 * channels + ch];
                let s1 = input[idx1 * channels + ch];
                out.push(s0 + frac * (s1 - s0));
            }
            self.phase += step;
        }

        self.phase -= in_frames as f64;
        out
    }
}

fn calculate_replaygain(dec: &StreamingDecoder, mode: &str) -> f32 {
    let db = match mode {
        "track" => dec.replaygain_track.or(dec.replaygain_album),
        "album" => dec.replaygain_album.or(dec.replaygain_track),
        _ => None,
    };
    db.map(|d| 10.0f32.powf(d / 20.0)).unwrap_or(1.0)
}

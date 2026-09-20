use std::sync::Arc;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;

use crate::visualizer::SpectrumAnalyzer;
use crate::decoder::StreamingDecoder;

/// Commands sent from the UI thread to the player.
pub enum PlayerCommand {
    Play(std::path::PathBuf),
    Pause,
    Resume,
    Stop,
    Seek(Duration),
    SetVolume(f32),
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
                            decoder = Some(dec);
                            current_samples_decoded = 0;
                            let mut state = self.state.lock();
                            state.playing = true;
                            state.paused = false;
                            state.position = Duration::ZERO;
                            let (duration, sample_rate, channels) = {
                                let d = decoder.as_ref().unwrap();
                                (d.total_duration, d.sample_rate, d.channels)
                            };
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

                        current_samples_decoded += (frame.len() / channels) as u64;
                        self.state.lock().position = Duration::from_secs_f64(current_samples_decoded as f64 / dec.sample_rate as f64);
                        
                        if self.sample_tx.send(frame).is_err() {
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

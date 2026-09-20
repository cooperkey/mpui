use std::path::Path;

use anyhow::{Context, Result};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decoded audio data from a single file.
pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: usize,
}

pub struct StreamingDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    pub sample_rate: u32,
    pub channels: usize,
    pub total_duration: Option<u64>,
}

impl StreamingDecoder {
    pub fn new(path: &Path) -> Result<Self> {
        let file = std::fs::File::open(path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }

        let probed = symphonia::default::get_probe().format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )?;

        let format = probed.format;

        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .context("no supported audio track found")?;

        let track_id = track.id;
        let sample_rate = track
            .codec_params
            .sample_rate
            .context("unknown sample rate")?;
        let channels = track
            .codec_params
            .channels
            .map(|c| c.count())
            .unwrap_or(2);
        
        let total_duration = track.codec_params.n_frames;

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())?;

        Ok(Self {
            format,
            decoder,
            track_id,
            sample_rate,
            channels,
            total_duration,
        })
    }

    pub fn next_frame(&mut self) -> Option<Vec<f32>> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(p) => p,
                Err(_) => return None,
            };

            if packet.track_id() != self.track_id {
                continue;
            }

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    let spec = *decoded.spec();
                    let duration = decoded.capacity();

                    let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, spec);
                    sample_buf.copy_interleaved_ref(decoded);
                    return Some(sample_buf.samples().to_vec());
                }
                Err(symphonia::core::errors::Error::IoError(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return None;
                }
                Err(symphonia::core::errors::Error::DecodeError(_)) => {
                    continue;
                }
                Err(_) => return None,
            }
        }
    }

    pub fn seek(&mut self, time: std::time::Duration) -> Result<()> {
        let ts = (time.as_secs_f64() * self.sample_rate as f64) as u64;
        self.format.seek(
            symphonia::core::formats::SeekMode::Accurate,
            symphonia::core::formats::SeekTo::TimeStamp { ts, track_id: self.track_id },
        )?;
        self.decoder.reset();
        Ok(())
    }
}

/// Decode an entire audio file into interleaved f32 samples.
///
/// For v0.1 this decodes fully into memory. Streaming decode will be
/// implemented in the audio thread for v0.2+.
pub fn decode_file(path: &Path) -> Result<DecodedAudio> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe().format(
        &hint,
        mss,
        &FormatOptions::default(),
        &MetadataOptions::default(),
    )?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("no supported audio track found")?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .context("unknown sample rate")?;
    let channels = track
        .codec_params
        .channels
        .map(|c| c.count())
        .unwrap_or(2);

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(e.into()),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = decoder.decode(&packet)?;
        let spec = *decoded.spec();
        let duration = decoded.capacity();

        let mut sample_buf = SampleBuffer::<f32>::new(duration as u64, spec);
        sample_buf.copy_interleaved_ref(decoded);
        all_samples.extend_from_slice(sample_buf.samples());
    }

    Ok(DecodedAudio {
        samples: all_samples,
        sample_rate,
        channels,
    })
}

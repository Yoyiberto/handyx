use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub struct AudioRecorder {
    is_recording: Arc<AtomicBool>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    current_level: Arc<Mutex<f32>>,
    sample_rate: u32,
    stream: Option<Stream>,
}

impl AudioRecorder {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            is_recording: Arc::new(AtomicBool::new(false)),
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            current_level: Arc::new(Mutex::new(0.0)),
            sample_rate,
            stream: None,
        }
    }

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::SeqCst)
    }

    pub fn get_current_level(&self) -> f32 {
        *self.current_level.lock().unwrap()
    }

    pub fn start(&mut self) -> Result<(), String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No default audio input device found".to_string())?;

        let supported_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let device_sample_rate = supported_config.sample_rate().into();
        let channels = supported_config.channels();
        let stream_config: StreamConfig = supported_config.clone().into();

        {
            let mut buf = self.audio_buffer.lock().unwrap();
            buf.clear();
        }
        self.is_recording.store(true, Ordering::SeqCst);

        let is_recording_flag = self.is_recording.clone();
        let buffer_clone = self.audio_buffer.clone();
        let level_clone = self.current_level.clone();
        let target_rate = self.sample_rate;

        let err_fn = |err| eprintln!("Audio stream error: {}", err);

        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => device.build_input_stream(
                stream_config,
                move |data: &[f32], _| {
                    if is_recording_flag.load(Ordering::SeqCst) {
                        process_audio_chunk(
                            data,
                            channels,
                            device_sample_rate,
                            target_rate,
                            &buffer_clone,
                            &level_clone,
                        );
                    }
                },
                err_fn,
                None,
            ),
            SampleFormat::I16 => device.build_input_stream(
                stream_config,
                move |data: &[i16], _| {
                    if is_recording_flag.load(Ordering::SeqCst) {
                        let f32_data: Vec<f32> = data.iter().map(|&s| s as f32 / 32768.0).collect();
                        process_audio_chunk(
                            &f32_data,
                            channels,
                            device_sample_rate,
                            target_rate,
                            &buffer_clone,
                            &level_clone,
                        );
                    }
                },
                err_fn,
                None,
            ),
            SampleFormat::U16 => device.build_input_stream(
                stream_config,
                move |data: &[u16], _| {
                    if is_recording_flag.load(Ordering::SeqCst) {
                        let f32_data: Vec<f32> = data
                            .iter()
                            .map(|&s| (s as f32 - 32768.0) / 32768.0)
                            .collect();
                        process_audio_chunk(
                            &f32_data,
                            channels,
                            device_sample_rate,
                            target_rate,
                            &buffer_clone,
                            &level_clone,
                        );
                    }
                },
                err_fn,
                None,
            ),
            _ => return Err("Unsupported audio format".to_string()),
        }
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> Result<Vec<u8>, String> {
        self.is_recording.store(false, Ordering::SeqCst);
        self.stream = None;

        let samples = {
            let buf = self.audio_buffer.lock().unwrap();
            buf.clone()
        };

        if samples.is_empty() {
            return Err("No audio samples captured".to_string());
        }

        let spec = WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut cursor, spec)
                .map_err(|e| format!("WAV writer init failed: {}", e))?;
            for &sample in &samples {
                let clamped = sample.max(-1.0).min(1.0);
                let val = (clamped * 32767.0) as i16;
                writer
                    .write_sample(val)
                    .map_err(|e| format!("WAV write sample failed: {}", e))?;
            }
            writer
                .finalize()
                .map_err(|e| format!("WAV finalize failed: {}", e))?;
        }

        Ok(cursor.into_inner())
    }

    pub fn get_raw_samples(&self) -> Vec<f32> {
        let buf = self.audio_buffer.lock().unwrap();
        buf.clone()
    }
}

fn process_audio_chunk(
    data: &[f32],
    channels: u16,
    input_rate: u32,
    target_rate: u32,
    buffer: &Arc<Mutex<Vec<f32>>>,
    level: &Arc<Mutex<f32>>,
) {
    if data.is_empty() {
        return;
    }

    let channels_usize = channels as usize;
    let mut mono: Vec<f32> = Vec::with_capacity(data.len() / channels_usize);
    for chunk in data.chunks_exact(channels_usize) {
        let sum: f32 = chunk.iter().sum();
        mono.push(sum / channels_usize as f32);
    }

    let mut sum_sq = 0.0;
    for &s in &mono {
        sum_sq += s * s;
    }
    let rms = (sum_sq / mono.len() as f32).sqrt();
    if let Ok(mut lvl) = level.lock() {
        *lvl = rms * 4.0;
    }

    let resampled = if input_rate == target_rate {
        mono
    } else {
        resample_linear(&mono, input_rate, target_rate)
    };

    if let Ok(mut buf) = buffer.lock() {
        buf.extend(resampled);
    }
}

fn resample_linear(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let target_len = (input.len() as f64 / ratio).round() as usize;
    let mut output = Vec::with_capacity(target_len);

    for i in 0..target_len {
        let src_idx = i as f64 * ratio;
        let index_floor = src_idx.floor() as usize;
        let index_ceil = (index_floor + 1).min(input.len() - 1);
        let frac = (src_idx - index_floor as f64) as f32;

        let sample = if index_floor < input.len() {
            input[index_floor] * (1.0 - frac) + input[index_ceil] * frac
        } else {
            0.0
        };
        output.push(sample);
    }

    output
}

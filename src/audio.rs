use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Recorder {
    stream: Option<cpal::Stream>,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    path: PathBuf,
}

impl Recorder {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self {
            stream: None,
            writer: Arc::new(Mutex::new(None)),
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 0,
            channels: 0,
            path,
        })
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("no input device"))?;
        let config = device.default_input_config()?;
        let spec = WavSpec {
            channels: config.channels(),
            sample_rate: config.sample_rate().0,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let writer = WavWriter::create(&self.path, spec)?;
        let w = Arc::clone(&self.writer);
        *w.lock().unwrap() = Some(writer);

        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();

        let err_fn = |e| tracing::error!("audio stream error: {}", e);
        let writer_clone = Arc::clone(&self.writer);
        let samples_clone = Arc::clone(&self.samples);
        let channels = config.channels() as usize;
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if let Some(wr) = writer_clone.lock().unwrap().as_mut() {
                        let mut smp = samples_clone.lock().unwrap();
                        for frame in data.chunks(channels.max(1)) {
                            let mut acc = 0.0f32;
                            for &v in frame {
                                acc += v;
                            }
                            let mono = if frame.is_empty() {
                                0.0
                            } else {
                                acc / frame.len() as f32
                            };
                            smp.push(mono);
                            for &v in frame {
                                let _ = wr.write_sample((v * i16::MAX as f32) as i16);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if let Some(wr) = writer_clone.lock().unwrap().as_mut() {
                        let mut smp = samples_clone.lock().unwrap();
                        for frame in data.chunks(channels.max(1)) {
                            let mut acc = 0.0f32;
                            for &v in frame {
                                acc += v as f32;
                            }
                            let mono = if frame.is_empty() {
                                0.0
                            } else {
                                acc / frame.len() as f32 / i16::MAX as f32
                            };
                            smp.push(mono);
                            for &v in frame {
                                let _ = wr.write_sample(v);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )?,
            _ => return Err(anyhow::anyhow!("unsupported sample format")),
        };
        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop(&mut self) -> anyhow::Result<PathBuf> {
        self.stream = None;
        if let Some(w) = self.writer.lock().unwrap().take() {
            w.finalize()?;
        }
        Ok(self.path.clone())
    }

    pub fn samples(&self) -> Arc<Mutex<Vec<f32>>> {
        Arc::clone(&self.samples)
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn elapsed_seconds(&self) -> f32 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.samples.lock().unwrap().len() as f32 / self.sample_rate as f32
    }
}

pub fn write_wav(path: &std::path::Path, samples: &[f32], sample_rate: u32) -> anyhow::Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut writer = hound::WavWriter::create(path, spec)?;
    for &v in samples {
        let s = (v * i16::MAX as f32).clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        writer.write_sample(s)?;
    }
    writer.finalize()?;
    Ok(())
}

pub struct Player {
    samples: Arc<Vec<f32>>,
    src_sr: u32,
    src_ch: usize,
    out_sr: u32,
    out_ch: usize,
    stream: Option<cpal::Stream>,
    pos: Arc<Mutex<f64>>,
    playing: Arc<Mutex<bool>>,
}

impl Player {
    pub fn from_wav(path: PathBuf) -> anyhow::Result<Self> {
        let mut reader = hound::WavReader::open(&path)?;
        let spec = reader.spec();
        let src_sr = spec.sample_rate;
        let src_ch = spec.channels as usize;
        let bits = spec.bits_per_sample;
        let mut samples: Vec<f32> = Vec::new();
        match spec.sample_format {
            hound::SampleFormat::Float => {
                for s in reader.into_samples::<f32>() {
                    samples.push(s.unwrap_or(0.0));
                }
            }
            hound::SampleFormat::Int => {
                let scale = (1i64 << (bits.saturating_sub(1))) as f32;
                for s in reader.into_samples::<i32>() {
                    samples.push(s.unwrap_or(0) as f32 / scale);
                }
            }
        }

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| anyhow::anyhow!("no output device"))?;
        let config = device.default_output_config()?;
        let out_sr = config.sample_rate().0;
        let out_ch = config.channels() as usize;

        let samples_arc = Arc::new(samples);
        let pos = Arc::new(Mutex::new(0.0f64));
        let playing = Arc::new(Mutex::new(false));

        let samples_cb = Arc::clone(&samples_arc);
        let pos_cb = Arc::clone(&pos);
        let playing_cb = Arc::clone(&playing);
        let src_sr_f = src_sr as f64;
        let out_sr_f = out_sr as f64;
        let src_ch_usize = src_ch.max(1);
        let out_ch_usize = out_ch.max(1);

        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _| {
                let is_playing = *playing_cb.lock().unwrap();
                if !is_playing {
                    for v in data.iter_mut() {
                        *v = 0.0;
                    }
                    return;
                }
                let mut pos_guard = pos_cb.lock().unwrap();
                let mut p = *pos_guard;
                let src_frames = samples_cb.len() / src_ch_usize;
                let step = src_sr_f / out_sr_f;
                let out_frames = data.len() / out_ch_usize;
                for i in 0..out_frames {
                    if p >= src_frames as f64 {
                        for c in 0..out_ch_usize {
                            data[i * out_ch_usize + c] = 0.0;
                        }
                        continue;
                    }
                    let src_idx = (p as usize) * src_ch_usize;
                    for c in 0..out_ch_usize {
                        let src_c = if c < src_ch_usize { c } else { 0 };
                        let val = samples_cb.get(src_idx + src_c).copied().unwrap_or(0.0);
                        data[i * out_ch_usize + c] = val;
                    }
                    p += step;
                }
                *pos_guard = p;
                if p >= src_frames as f64 {
                    *playing_cb.lock().unwrap() = false;
                }
            },
            |e| tracing::error!("audio output stream error: {}", e),
            None,
        )?;
        stream.play()?;

        Ok(Self {
            samples: samples_arc,
            src_sr,
            src_ch: src_ch.max(1),
            out_sr,
            out_ch: out_ch.max(1),
            stream: Some(stream),
            pos,
            playing,
        })
    }

    pub fn play(&mut self) {
        *self.pos.lock().unwrap() = 0.0;
        *self.playing.lock().unwrap() = true;
    }

    pub fn stop(&mut self) {
        *self.playing.lock().unwrap() = false;
        *self.pos.lock().unwrap() = 0.0;
    }

    pub fn is_playing(&self) -> bool {
        *self.playing.lock().unwrap()
    }

    pub fn position_secs(&self) -> f32 {
        *self.pos.lock().unwrap() as f32 / self.src_sr as f32
    }

    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / self.src_ch as f32 / self.src_sr as f32
    }

    pub fn samples(&self) -> Arc<Vec<f32>> {
        Arc::clone(&self.samples)
    }

    pub fn sample_rate(&self) -> u32 {
        self.src_sr
    }
}

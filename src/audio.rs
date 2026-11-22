use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Recorder {
    stream: Option<cpal::Stream>,
    writer: Arc<Mutex<Option<WavWriter<std::io::BufWriter<std::fs::File>>>>>,
    path: PathBuf,
}

impl Recorder {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        Ok(Self { stream: None, writer: Arc::new(Mutex::new(None)), path })
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| anyhow::anyhow!("no input device"))?;
        let config = device.default_input_config()?;
        let spec = WavSpec {
            channels: config.channels(),
            sample_rate: config.sample_rate().0,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        if let Some(parent) = self.path.parent() { std::fs::create_dir_all(parent)?; }
        let writer = WavWriter::create(&self.path, spec)?;
        let w = Arc::clone(&self.writer);
        *w.lock().unwrap() = Some(writer);

        let err_fn = |e| tracing::error!("audio stream error: {}", e);
        let writer_clone = Arc::clone(&self.writer);
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    if let Some(wr) = writer_clone.lock().unwrap().as_mut() {
                        for &s in data { let _ = wr.write_sample((s * i16::MAX as f32) as i16); }
                    }
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    if let Some(wr) = writer_clone.lock().unwrap().as_mut() {
                        for &s in data { let _ = wr.write_sample(s); }
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
}

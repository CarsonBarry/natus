use crate::audio::Recorder;
use crate::clicks::{ClickRecorder, ClickSequence};
use crate::config::Config;
use crate::filler::remove_fillers;
use crate::screenshot;
use crate::storage::{CaptureEntry, Session};
use crate::theme;
use crate::tunnel;
use crate::voice_commands::{self, VoiceCommand};
use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

pub struct NatusApp {
    cfg: Config,
    rt: Arc<Runtime>,
    // recording state
    session: Option<Session>,
    recorder: Option<Recorder>,
    audio_path: Option<PathBuf>,
    audio_samples: Option<std::sync::Arc<std::sync::Mutex<Vec<f32>>>>,
    audio_sample_rate: u32,
    recording: bool,
    view_seconds: f32,
    player: Option<crate::audio::Player>,
    capturing_sequence: bool,
    sequence_entries: Vec<CaptureEntry>,
    // click recording
    click_recorder: ClickRecorder,
    click_sequences: Vec<ClickSequence>,
    recording_clicks: bool,
    // voice commands
    commands: Vec<VoiceCommand>,
    new_cmd_phrase: String,
    voice_listening_enabled: bool,
    commands_shared: Arc<std::sync::Mutex<Vec<VoiceCommand>>>,
    command_tx: mpsc::Sender<VoiceCommand>,
    command_rx: Option<mpsc::Receiver<VoiceCommand>>,
    listener_stop: Option<Arc<AtomicBool>>,
    last_rec_cmd_check: Option<Instant>,
    last_triggered_phrase: Option<String>,
    last_triggered_at: Option<Instant>,
    // ui
    tab: Tab,
    status: String,
    review_index: usize,
    transcript: String,
    summary: String,
    // settings fields
    hangar_url: String,
    stt_model: String,
    chat_model: String,
    stt_base: String,
    chat_base: String,
    hangar_status: String,
    discovered: Vec<(String, String)>,
    ssh_enabled: bool,
    ssh_host: String,
    ssh_user: String,
    ssh_local_port: String,
    ssh_remote_port: String,
    tunnel: tunnel::SshTunnel,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    Record,
    Review,
    Commands,
    Clicks,
    Settings,
}

impl NatusApp {
    pub fn new(cfg: Config, rt: Arc<Runtime>) -> Self {
        let commands = voice_commands::default_commands();
        let commands_shared = Arc::new(std::sync::Mutex::new(commands.clone()));
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let hangar_url = cfg.hangar_url.clone();
        let stt_model = cfg.stt_model.clone();
        let chat_model = cfg.chat_model.clone();
        let stt_base = cfg.stt_base_url.clone();
        let chat_base = cfg.chat_base_url.clone();
        let ssh_enabled = cfg.ssh_tunnel_enabled;
        let ssh_host = cfg.ssh_tunnel_host.clone();
        let ssh_user = cfg.ssh_tunnel_user.clone();
        let ssh_local_port = cfg.ssh_tunnel_local_port.to_string();
        let ssh_remote_port = cfg.ssh_tunnel_remote_port.to_string();
        let mut tunnel = tunnel::SshTunnel::new();
        if ssh_enabled {
            let _ = tunnel.start(&cfg);
        }
        Self {
            cfg,
            rt,
            session: None,
            recorder: None,
            audio_path: None,
            audio_samples: None,
            audio_sample_rate: 0,
            recording: false,
            capturing_sequence: false,
            sequence_entries: vec![],
            click_recorder: ClickRecorder::new(),
            click_sequences: vec![],
            recording_clicks: false,
            commands,
            new_cmd_phrase: String::new(),
            voice_listening_enabled: true,
            commands_shared,
            command_tx: cmd_tx,
            command_rx: Some(cmd_rx),
            listener_stop: None,
            last_rec_cmd_check: None,
            last_triggered_phrase: None,
            last_triggered_at: None,
            tab: Tab::Record,
            status: "Ready".into(),
            review_index: 0,
            view_seconds: 10.0,
            player: None,
            transcript: String::new(),
            summary: String::new(),
            hangar_url,
            stt_model,
            chat_model,
            stt_base,
            chat_base,
            hangar_status: "Not checked".into(),
            discovered: vec![],
            ssh_enabled,
            ssh_host,
            ssh_user,
            ssh_local_port,
            ssh_remote_port,
            tunnel,
        }
    }
}

impl eframe::App for NatusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll global hotkey events for quick capture.
        if let Ok(ev) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            let _ = ev;
            self.do_capture();
            self.status = "Hotkey capture".into();
        }

        // Poll voice commands detected by background listeners.
        if let Some(rx) = &self.command_rx {
            // drain into a vec first to avoid borrow issues
            let mut received = vec![];
            while let Ok(cmd) = rx.try_recv() {
                received.push(cmd);
            }
            for cmd in received {
                let skip = match (&self.last_triggered_phrase, &self.last_triggered_at) {
                    (Some(p), Some(t)) => p == &cmd.phrase && t.elapsed() < Duration::from_secs(4),
                    _ => false,
                };
                if skip {
                    continue;
                }
                self.last_triggered_phrase = Some(cmd.phrase.clone());
                self.last_triggered_at = Some(Instant::now());
                self.execute_command(cmd);
            }
        }

        // Manage live voice-command listening depending on recording state.
        if self.recording {
            if let Some(flag) = self.listener_stop.take() {
                flag.store(true, Ordering::Relaxed);
            }
            let now = Instant::now();
            let should_check = self
                .last_rec_cmd_check
                .map(|t| t.elapsed() >= Duration::from_secs(3))
                .unwrap_or(true);
            if should_check {
                self.last_rec_cmd_check = Some(now);
                if let Some(samples) = self.audio_samples.clone() {
                    let sr = self.audio_sample_rate;
                    if sr > 0 {
                        let cfg = self.cfg.clone();
                        let commands = Arc::clone(&self.commands_shared);
                        let tx = self.command_tx.clone();
                        std::thread::spawn(move || {
                            let needed = (sr as usize) * 3;
                            let buf = if let Ok(b) = samples.lock() {
                                let n = b.len();
                                let start = n.saturating_sub(needed);
                                b[start..].to_vec()
                            } else {
                                return;
                            };
                            if buf.is_empty() {
                                return;
                            }
                            let path = std::env::temp_dir()
                                .join(format!("natus_rec_cmd_{}.wav", uuid::Uuid::new_v4()));
                            if crate::audio::write_wav(&path, &buf, sr).is_err() {
                                return;
                            }
                            let rt = tokio::runtime::Runtime::new().unwrap();
                            let text = rt.block_on(async {
                                crate::stt::transcribe(&cfg, &path)
                                    .await
                                    .unwrap_or_default()
                            });
                            let _ = std::fs::remove_file(&path);
                            let cmds = commands.lock().unwrap().clone();
                            if let Some(cmd) = crate::voice_commands::match_command(&text, &cmds) {
                                let _ = tx.send(cmd);
                            }
                        });
                    }
                }
            }
        } else {
            self.last_rec_cmd_check = None;
            if self.voice_listening_enabled && self.listener_stop.is_none() {
                let stop = Arc::new(AtomicBool::new(false));
                self.listener_stop = Some(Arc::clone(&stop));
                let cfg = self.cfg.clone();
                let commands = Arc::clone(&self.commands_shared);
                let tx = self.command_tx.clone();
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    while !stop.load(Ordering::Relaxed) {
                        let path = std::env::temp_dir()
                            .join(format!("natus_cmd_{}.wav", uuid::Uuid::new_v4()));
                        let mut rec = match crate::audio::Recorder::new(path.clone()) {
                            Ok(r) => r,
                            Err(_) => {
                                std::thread::sleep(Duration::from_millis(500));
                                continue;
                            }
                        };
                        if rec.start().is_err() {
                            std::thread::sleep(Duration::from_millis(500));
                            continue;
                        }
                        for _ in 0..20 {
                            if stop.load(Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        let _ = rec.stop();
                        if stop.load(Ordering::Relaxed) {
                            let _ = std::fs::remove_file(&path);
                            break;
                        }
                        let cmds = commands.lock().unwrap().clone();
                        let text = rt.block_on(async {
                            crate::stt::transcribe(&cfg, &path)
                                .await
                                .unwrap_or_default()
                        });
                        let _ = std::fs::remove_file(&path);
                        if let Some(cmd) = crate::voice_commands::match_command(&text, &cmds) {
                            let _ = tx.send(cmd);
                        }
                        for _ in 0..5 {
                            if stop.load(Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    }
                });
            } else if !self.voice_listening_enabled {
                if let Some(flag) = self.listener_stop.take() {
                    flag.store(true, Ordering::Relaxed);
                }
            }
        }

        if self.recording || self.voice_listening_enabled || self.listener_stop.is_some() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Natus");
                ui.separator();
                for (t, label) in [
                    (Tab::Record, "Record"),
                    (Tab::Review, "Review"),
                    (Tab::Commands, "Commands"),
                    (Tab::Clicks, "Clicks"),
                    (Tab::Settings, "Settings"),
                ] {
                    if ui.selectable_label(self.tab == t, label).clicked() {
                        self.tab = t;
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let p = theme::pal(ui.ctx());
                    let running = self.tunnel.is_running();
                    let color = if running { p.teal } else { p.warn };
                    ui.colored_label(
                        color,
                        if running {
                            "Tunnel: connected"
                        } else {
                            "Tunnel: offline"
                        },
                    );
                });
            });
        });
        if self.recording
            || self
                .player
                .as_ref()
                .map(|p| p.is_playing())
                .unwrap_or(false)
        {
            ctx.request_repaint();
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Record => self.ui_record(ui),
            Tab::Review => self.ui_review(ui),
            Tab::Commands => self.ui_commands(ui),
            Tab::Clicks => self.ui_clicks(ui),
            Tab::Settings => self.ui_settings(ui),
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            let p = theme::pal(ui.ctx());
            ui.colored_label(p.dim, &self.status);
        });
    }
}

impl NatusApp {
    fn ui_record(&mut self, ui: &mut egui::Ui) {
        ui.heading("Recording & Capture");
        ui.label(
            "Create a session, record audio, capture screenshots, then transcribe and summarize.",
        );
        ui.separator();
        if self.session.is_none() {
            if ui.button("New Session").clicked() {
                self.session = Some(Session::new("Feedback session"));
                self.status = "Session created".into();
            }
        } else {
            ui.label(format!(
                "Session: {}",
                self.session.as_ref().unwrap().meta.id
            ));
        }
        ui.separator();

        ui.group(|ui| {
            ui.horizontal(|ui| {
                if !self.recording {
                    if ui.button("Start Audio Recording").clicked() {
                        if let Some(s) = &self.session {
                            let p = self.cfg.session_dir(&s.meta.id).join("audio.wav");
                            let mut rec = Recorder::new(p.clone()).unwrap();
                            if rec.start().is_ok() {
                                self.audio_samples = Some(rec.samples());
                                self.audio_sample_rate = rec.sample_rate();
                                self.recorder = Some(rec);
                                self.audio_path = Some(p);
                                self.recording = true;
                                self.status = "Recording audio...".into();
                            }
                        } else {
                            self.status = "Create a session first".into();
                        }
                    }
                } else {
                    let pp = theme::pal(ui.ctx());
                    if ui
                        .button(
                            egui::RichText::new("Stop Audio Recording")
                                .color(pp.del)
                                .strong(),
                        )
                        .clicked()
                    {
                        if let Some(mut rec) = self.recorder.take() {
                            if let Ok(p) = rec.stop() {
                                self.status = format!("Saved audio: {}", p.display());
                            }
                            self.audio_samples = Some(rec.samples());
                            self.audio_sample_rate = rec.sample_rate();
                        }
                        self.recording = false;
                    }
                }
                if self.recording {
                    self.draw_recording_indicator(ui);
                }
            });

            if self.recording {
                let elapsed = self
                    .recorder
                    .as_ref()
                    .map(|r| r.elapsed_seconds())
                    .unwrap_or(0.0);
                ui.label(format!("Elapsed: {}", fmt_time(elapsed)));
                let level = self.current_level();
                let (r, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 8.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter_at(r);
                painter.rect_filled(r, 2.0, theme::pal(ui.ctx()).panel3);
                let filled = r.with_max_x(r.min.x + r.width() * level.clamp(0.0, 1.0));
                painter.rect_filled(filled, 2.0, theme::pal(ui.ctx()).add);
            }

            ui.separator();
            self.draw_waveform(ui);

            ui.horizontal(|ui| {
                ui.label("Zoom (seconds visible):");
                ui.add(egui::Slider::new(&mut self.view_seconds, 1.0..=60.0).suffix(" s"));
            });

            if self.audio_path.is_some() && !self.recording {
                ui.separator();
                ui.horizontal(|ui| {
                    let playing = self
                        .player
                        .as_ref()
                        .map(|p| p.is_playing())
                        .unwrap_or(false);
                    if playing {
                        if ui.button("Stop").clicked() {
                            if let Some(p) = &mut self.player {
                                p.stop();
                            }
                        }
                    } else {
                        if ui.button("Play Recording").clicked() {
                            if self.player.is_none() {
                                if let Some(path) = &self.audio_path {
                                    match crate::audio::Player::from_wav(path.clone()) {
                                        Ok(pl) => {
                                            self.audio_sample_rate = pl.sample_rate();
                                            self.player = Some(pl);
                                        }
                                        Err(e) => self.status = format!("Player error: {}", e),
                                    }
                                }
                            }
                            if let Some(p) = &mut self.player {
                                p.play();
                            }
                        }
                    }
                    if let Some(p) = &self.player {
                        ui.label(format!(
                            "{:.1}s / {:.1}s",
                            p.position_secs(),
                            p.duration_secs()
                        ));
                    }
                });
            }
        });

        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Capture Screenshot").clicked() {
                self.do_capture();
            }
            if !self.capturing_sequence {
                if ui.button("Start Capture Sequence").clicked() {
                    self.capturing_sequence = true;
                    self.sequence_entries.clear();
                    self.status =
                        "Capture sequence started. Use 'Capture' or say 'capture'.".into();
                }
            } else {
                if ui.button("Stop Capture Sequence").clicked() {
                    self.capturing_sequence = false;
                    if let Some(s) = &mut self.session {
                        for e in self.sequence_entries.drain(..) {
                            s.entries.push(e);
                        }
                        let _ = s.save(&self.cfg);
                    }
                    self.status = "Capture sequence saved.".into();
                }
                if ui.button("Capture (seq)").clicked() {
                    self.do_capture_seq();
                }
            }
        });
        ui.separator();
        if ui.button("Transcribe and Summarize").clicked() {
            self.do_summarize();
        }
        if !self.transcript.is_empty() {
            ui.collapsing("Transcript", |ui| {
                ui.label(&self.transcript);
            });
        }
        if !self.summary.is_empty() {
            ui.collapsing("Summary", |ui| {
                ui.label(&self.summary);
            });
        }
        if let Some(s) = &self.session {
            ui.separator();
            ui.label(format!("Entries: {}", s.entries.len()));
            for (i, e) in s.entries.iter().enumerate() {
                ui.label(format!("{}: {}", i, e.text));
            }
        }
    }

    fn draw_recording_indicator(&self, ui: &mut egui::Ui) {
        let p = theme::pal(ui.ctx());
        let t = ui.input(|i| i.time);
        let alpha = (0.5 + 0.5 * (t * 4.0).sin()).clamp(0.0, 1.0);
        let color = egui::Color32::from_rgba_unmultiplied(220, 50, 50, (alpha * 255.0) as u8);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
        ui.painter().circle_filled(rect.center(), 6.0, color);
        ui.colored_label(p.del, "RECORDING");
    }

    fn current_level(&self) -> f32 {
        if let Some(arc) = &self.audio_samples {
            if let Ok(buf) = arc.try_lock() {
                let n = buf.len();
                if n == 0 {
                    return 0.0;
                }
                let sr = self.audio_sample_rate.max(1) as usize;
                let start = n.saturating_sub(sr / 10);
                let mut sum = 0.0f32;
                let mut cnt = 0;
                for v in &buf[start..] {
                    sum += v * v;
                    cnt += 1;
                }
                if cnt == 0 {
                    return 0.0;
                }
                (sum / cnt as f32).sqrt() * 3.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    fn draw_waveform(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width().max(100.0);
        let height = 120.0;
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let p = theme::pal(ui.ctx());
        painter.rect_filled(rect, 4.0, p.panel2);

        let view_seconds = self.view_seconds;

        if let Some(arc) = &self.audio_samples {
            if let Ok(buf) = arc.try_lock() {
                let sr = self.audio_sample_rate;
                if sr > 0 && !buf.is_empty() {
                    let duration = buf.len() as f32 / sr as f32;
                    let playhead = if self.recording {
                        duration
                    } else if let Some(pl) = &self.player {
                        if pl.is_playing() {
                            pl.position_secs()
                        } else {
                            duration
                        }
                    } else {
                        duration
                    };
                    self.draw_waveform_data(
                        &painter,
                        rect,
                        width,
                        height,
                        &buf,
                        sr,
                        view_seconds,
                        playhead,
                        p,
                    );
                    return;
                }
            }
        }
        if let Some(pl) = &self.player {
            let s = pl.samples();
            let sr = pl.sample_rate();
            if sr > 0 && !s.is_empty() {
                let duration = s.len() as f32 / sr as f32;
                let playhead = if pl.is_playing() {
                    pl.position_secs()
                } else {
                    duration
                };
                self.draw_waveform_data(
                    &painter,
                    rect,
                    width,
                    height,
                    &s,
                    sr,
                    view_seconds,
                    playhead,
                    p,
                );
                return;
            }
        }
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No audio",
            egui::FontId::default(),
            p.dim,
        );
    }

    fn draw_waveform_data(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        width: f32,
        height: f32,
        buf: &[f32],
        sr: u32,
        view_seconds: f32,
        playhead: f32,
        p: theme::Palette,
    ) {
        let total = buf.len();
        let duration = total as f32 / sr as f32;
        let view_dur = view_seconds.max(0.5);

        let anchor = if self.recording { duration } else { playhead };
        let mut view_end = anchor.max(view_dur);
        if view_end > duration && duration >= view_dur {
            view_end = duration.max(view_dur);
        }
        if duration < view_dur {
            view_end = view_dur;
        }
        let view_start = (view_end - view_dur).max(0.0);

        let start_sample = (view_start * sr as f32) as usize;
        let end_sample = ((view_end * sr as f32) as usize)
            .min(total)
            .max(start_sample + 1);
        let span = end_sample.saturating_sub(start_sample).max(1);

        let center_y = rect.center().y;
        let half_h = height / 2.0 - 4.0;

        painter.line_segment(
            [
                egui::pos2(rect.left(), center_y),
                egui::pos2(rect.right(), center_y),
            ],
            egui::Stroke::new(1.0, p.border),
        );

        let w = width as usize;
        for x in 0..w {
            let i0 = start_sample + (x * span / w);
            let i1 = start_sample + ((x + 1) * span / w);
            let i1 = i1.max(i0 + 1).min(end_sample);
            let mut mn = 0.0f32;
            let mut mx = 0.0f32;
            let mut any = false;
            for i in i0..i1 {
                let v = buf[i];
                if !any {
                    mn = v;
                    mx = v;
                    any = true;
                } else {
                    mn = mn.min(v);
                    mx = mx.max(v);
                }
            }
            if !any {
                mn = 0.0;
                mx = 0.0;
            }
            let y0 = center_y - mx * half_h;
            let y1 = center_y - mn * half_h;
            let xc = rect.left() + x as f32;
            painter.line_segment(
                [egui::pos2(xc, y0), egui::pos2(xc, y1)],
                egui::Stroke::new(1.0, p.accent),
            );
        }

        let ph_x = rect.left() + (playhead - view_start) / view_dur * width;
        if ph_x >= rect.left() && ph_x <= rect.right() {
            painter.line_segment(
                [
                    egui::pos2(ph_x, rect.top()),
                    egui::pos2(ph_x, rect.bottom()),
                ],
                egui::Stroke::new(2.0, p.warn),
            );
        }

        painter.text(
            egui::pos2(rect.left() + 4.0, rect.bottom() - 2.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{:.1}s", view_start),
            egui::FontId::monospace(10.0),
            p.dim,
        );
        painter.text(
            egui::pos2(rect.right() - 4.0, rect.bottom() - 2.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("{:.1}s", view_end),
            egui::FontId::monospace(10.0),
            p.dim,
        );
    }

    fn do_capture(&mut self) {
        let id = self.session.as_ref().map(|s| s.meta.id.clone());
        if id.is_none() {
            self.status = "Create a session first".into();
            return;
        }
        let id = id.unwrap();
        let dir = self
            .cfg
            .session_dir(&id)
            .join(&self.cfg.screenshot_dir_name);
        let ts = chrono::Utc::now().timestamp_millis();
        let path = dir.join(format!("shot_{}.png", ts));
        match screenshot::capture_primary(path.clone()) {
            Ok(p) => {
                let entry = CaptureEntry {
                    timestamp_ms: ts,
                    screenshot_path: Some(p),
                    text: String::new(),
                    cleaned_text: None,
                };
                if let Some(s) = &mut self.session {
                    s.entries.push(entry);
                    let _ = s.save(&self.cfg);
                }
                self.status = "Screenshot captured".into();
            }
            Err(e) => self.status = format!("Screenshot failed: {}", e),
        }
    }

    fn do_capture_seq(&mut self) {
        let id = self.session.as_ref().map(|s| s.meta.id.clone());
        if id.is_none() {
            self.status = "Create a session first".into();
            return;
        }
        let id = id.unwrap();
        let dir = self
            .cfg
            .session_dir(&id)
            .join(&self.cfg.screenshot_dir_name);
        let ts = chrono::Utc::now().timestamp_millis();
        let path = dir.join(format!("shot_{}.png", ts));
        match screenshot::capture_primary(path.clone()) {
            Ok(p) => {
                let entry = CaptureEntry {
                    timestamp_ms: ts,
                    screenshot_path: Some(p),
                    text: String::new(),
                    cleaned_text: None,
                };
                self.sequence_entries.push(entry);
                self.status = format!("Captured #{}", self.sequence_entries.len());
            }
            Err(e) => self.status = format!("Screenshot failed: {}", e),
        }
    }

    pub fn execute_command(&mut self, cmd: VoiceCommand) {
        self.last_triggered_phrase = Some(cmd.phrase.clone());
        self.last_triggered_at = Some(Instant::now());
        self.execute_command_inner(cmd)
    }

    pub fn handle_voice_text(&mut self, text: &str) {
        let cmds = self.commands.clone();
        if let Some(cmd) = crate::voice_commands::match_command(text, &cmds) {
            self.execute_command(cmd);
        }
    }

    fn execute_command_inner(&mut self, cmd: VoiceCommand) {
        match cmd.action {
            voice_commands::CommandAction::Capture => self.do_capture(),
            voice_commands::CommandAction::StartSequence => {
                self.capturing_sequence = true;
                self.sequence_entries.clear();
                self.status = "Capture sequence started (voice).".into();
            }
            voice_commands::CommandAction::StopSequence => {
                self.capturing_sequence = false;
                if let Some(s) = &mut self.session {
                    for e in self.sequence_entries.drain(..) {
                        s.entries.push(e);
                    }
                    let _ = s.save(&self.cfg);
                }
                self.status = "Capture sequence saved (voice).".into();
            }
            voice_commands::CommandAction::StartClickRecording => {
                self.click_recorder.start();
                self.recording_clicks = true;
                self.status = "Recording clicks (voice)...".into();
            }
            voice_commands::CommandAction::StopClickRecording => {
                if self.recording_clicks {
                    let seq = self.click_recorder.stop();
                    self.click_sequences.push(seq);
                    self.recording_clicks = false;
                    self.status = "Click sequence saved (voice).".into();
                }
            }
            voice_commands::CommandAction::PlayClickSequence { name } => {
                if let Some(seq) = self
                    .click_sequences
                    .iter()
                    .find(|s| s.name == name)
                    .cloned()
                {
                    std::thread::spawn(move || crate::clicks::play(&seq));
                } else if let Some(seq) = self.click_sequences.last().cloned() {
                    std::thread::spawn(move || crate::clicks::play(&seq));
                }
            }
            voice_commands::CommandAction::RunScript { path, args } => {
                let _ = voice_commands::run_script(&path, &args);
                self.status = format!("Ran script: {}", path.display());
            }
            voice_commands::CommandAction::Custom { label } => {
                self.status = format!("Voice command: {}", label);
                if label.to_lowercase().contains("capture") {
                    self.do_capture();
                }
            }
        }
    }

    fn do_summarize(&mut self) {
        let audio = self.audio_path.clone();
        let cfg = self.cfg.clone();
        let rt = Arc::clone(&self.rt);
        let fillers = cfg.filler_words.clone();
        let transcript = rt.block_on(async {
            if let Some(a) = &audio {
                match crate::stt::transcribe(&cfg, a).await {
                    Ok(t) => t,
                    Err(e) => format!("STT error: {}", e),
                }
            } else {
                String::new()
            }
        });
        let cleaned = remove_fillers(&transcript, &fillers);
        self.transcript = cleaned.clone();
        let summary = rt.block_on(async {
            match crate::llm::summarize(&cfg, &cleaned).await {
                Ok(s) => s,
                Err(e) => format!("LLM error: {}", e),
            }
        });
        self.summary = summary;
        self.status = "Summarized".into();
    }

    fn ui_review(&mut self, ui: &mut egui::Ui) {
        ui.heading("Review");
        if let Some(s) = &self.session {
            if s.entries.is_empty() {
                ui.label("No entries.");
                return;
            }
            let idx = self.review_index.min(s.entries.len().saturating_sub(1));
            self.review_index = idx;
            ui.horizontal(|ui| {
                if ui.button("Prev").clicked() && self.review_index > 0 {
                    self.review_index -= 1;
                }
                if ui.button("Next").clicked() && self.review_index + 1 < s.entries.len() {
                    self.review_index += 1;
                }
                ui.label(format!("{}/{}", self.review_index + 1, s.entries.len()));
            });
            let e = &s.entries[self.review_index];
            if let Some(p) = &e.screenshot_path {
                ui.label(format!("Screenshot: {}", p.display()));
                if let Ok(img) = image::open(p) {
                    let size = [img.width() as usize, img.height() as usize];
                    let rgba = img.to_rgba8();
                    let tex = ctx_texture(ui.ctx(), size, &rgba);
                    ui.image(&tex);
                }
            }
            ui.label(format!("Text: {}", e.text));
            if let Some(c) = &e.cleaned_text {
                ui.label(format!("Cleaned: {}", c));
            }
        } else {
            ui.label("No session.");
        }
    }

    fn ui_commands(&mut self, ui: &mut egui::Ui) {
        ui.heading("Voice Commands");
        ui.label("Built-in and custom commands. Triggered live from microphone audio, during or outside recording.");
        ui.checkbox(
            &mut self.voice_listening_enabled,
            "Enable live voice-command listening",
        );
        for c in &self.commands {
            ui.label(format!("- {}", c.phrase));
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_cmd_phrase);
            if ui.button("Add 'capture'-style command").clicked()
                && !self.new_cmd_phrase.trim().is_empty()
            {
                self.commands.push(voice_commands::VoiceCommand {
                    phrase: self.new_cmd_phrase.clone(),
                    action: voice_commands::CommandAction::Custom {
                        label: self.new_cmd_phrase.clone(),
                    },
                });
                if let Ok(mut g) = self.commands_shared.lock() {
                    *g = self.commands.clone();
                }
                self.new_cmd_phrase.clear();
            }
        });
        ui.label("Custom script trigger: configure via editing commands.json (future).");
    }

    fn ui_clicks(&mut self, ui: &mut egui::Ui) {
        ui.heading("Click Sequences");
        ui.horizontal(|ui| {
            if !self.recording_clicks {
                if ui.button("Start Recording Clicks").clicked() {
                    self.click_recorder.start();
                    self.recording_clicks = true;
                    self.status = "Recording clicks...".into();
                }
            } else {
                if ui.button("Stop Recording Clicks").clicked() {
                    let seq = self.click_recorder.stop();
                    self.click_sequences.push(seq);
                    self.recording_clicks = false;
                    self.status = "Click sequence saved.".into();
                }
            }
        });
        ui.label(format!("Saved sequences: {}", self.click_sequences.len()));
        for (i, s) in self.click_sequences.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}: {} events", i, s.events.len()));
                if ui.button("Play").clicked() {
                    let seq = s.clone();
                    std::thread::spawn(move || crate::clicks::play(&seq));
                }
            });
        }
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings");
        ui.label("Hangar URL");
        ui.text_edit_singleline(&mut self.hangar_url);
        ui.horizontal(|ui| {
            if ui.button("Test Hangar Connection").clicked() {
                let url = self.hangar_url.trim_end_matches('/').to_string();
                let rt = Arc::clone(&self.rt);
                let res = rt.block_on(async {
                    let client = reqwest::Client::new();
                    match client.get(format!("{}/v1/models", url)).send().await {
                        Ok(resp) if resp.status().is_success() => {
                            let v: serde_json::Value = resp.json().await.unwrap_or_default();
                            let mut found = vec![];
                            if let Some(data) = v["data"].as_array() {
                                for m in data {
                                    let id = m["id"].as_str().unwrap_or("").to_string();
                                    let kind =
                                        m["hangar"]["kind"].as_str().unwrap_or("model").to_string();
                                    found.push((id, kind));
                                }
                            }
                            Ok(found)
                        }
                        Ok(resp) => Err(format!("HTTP {}", resp.status())),
                        Err(e) => Err(format!("Error: {}", e)),
                    }
                });
                match res {
                    Ok(found) => {
                        self.discovered = found;
                        self.hangar_status =
                            format!("Connected ({} models)", self.discovered.len());
                    }
                    Err(e) => self.hangar_status = e,
                }
            }
            ui.label(&self.hangar_status);
        });
        ui.label("STT model / route");
        ui.text_edit_singleline(&mut self.stt_model);
        if !self.discovered.is_empty() {
            egui::ComboBox::from_label("Pick STT route")
                .selected_text(&self.stt_model)
                .show_ui(ui, |ui| {
                    for (id, kind) in &self.discovered {
                        if kind == "stt" {
                            if ui.selectable_label(self.stt_model == *id, id).clicked() {
                                self.stt_model = id.clone();
                            }
                        }
                    }
                });
        }
        ui.label("Chat model / route");
        ui.text_edit_singleline(&mut self.chat_model);
        if !self.discovered.is_empty() {
            egui::ComboBox::from_label("Pick chat route")
                .selected_text(&self.chat_model)
                .show_ui(ui, |ui| {
                    for (id, kind) in &self.discovered {
                        if kind == "chat" {
                            if ui.selectable_label(self.chat_model == *id, id).clicked() {
                                self.chat_model = id.clone();
                            }
                        }
                    }
                });
        }
        ui.label("STT base URL (fallback)");
        ui.text_edit_singleline(&mut self.stt_base);
        ui.label("Chat base URL (fallback)");
        ui.text_edit_singleline(&mut self.chat_base);
        ui.separator();
        ui.heading("SSH tunnel to Hangar (Tailscale)");
        ui.checkbox(&mut self.ssh_enabled, "Enable SSH tunnel");
        ui.label("SSH host");
        ui.text_edit_singleline(&mut self.ssh_host);
        ui.label("SSH user");
        ui.text_edit_singleline(&mut self.ssh_user);
        ui.label("Local port");
        ui.text_edit_singleline(&mut self.ssh_local_port);
        ui.label("Remote port");
        ui.text_edit_singleline(&mut self.ssh_remote_port);
        ui.horizontal(|ui| {
            if ui.button("Start Tunnel").clicked() {
                self.cfg.ssh_tunnel_host = self.ssh_host.clone();
                self.cfg.ssh_tunnel_user = self.ssh_user.clone();
                self.cfg.ssh_tunnel_local_port = self.ssh_local_port.parse().unwrap_or(8500);
                self.cfg.ssh_tunnel_remote_port = self.ssh_remote_port.parse().unwrap_or(8500);
                self.cfg.ssh_tunnel_enabled = self.ssh_enabled;
                match self.tunnel.start(&self.cfg) {
                    Ok(_) => self.status = format!("SSH tunnel: {}", self.tunnel.status),
                    Err(e) => self.status = format!("SSH tunnel failed: {}", e),
                }
            }
            if ui.button("Stop Tunnel").clicked() {
                self.tunnel.stop();
                self.status = format!("SSH tunnel: {}", self.tunnel.status);
            }
            let st = self.tunnel.status.clone();
            let running = self.tunnel.is_running();
            ui.label(format!("Status: {} (running={})", st, running));
        });
        if ui.button("Save").clicked() {
            self.cfg.hangar_url = self.hangar_url.clone();
            self.cfg.stt_model = self.stt_model.clone();
            self.cfg.chat_model = self.chat_model.clone();
            self.cfg.stt_base_url = self.stt_base.clone();
            self.cfg.chat_base_url = self.chat_base.clone();
            self.cfg.ssh_tunnel_enabled = self.ssh_enabled;
            self.cfg.ssh_tunnel_host = self.ssh_host.clone();
            self.cfg.ssh_tunnel_user = self.ssh_user.clone();
            self.cfg.ssh_tunnel_local_port = self.ssh_local_port.parse().unwrap_or(8500);
            self.cfg.ssh_tunnel_remote_port = self.ssh_remote_port.parse().unwrap_or(8500);
            let _ = self.cfg.save();
            self.status = "Settings saved".into();
        }
    }
}

fn ctx_texture(ctx: &egui::Context, size: [usize; 2], rgba: &[u8]) -> egui::TextureHandle {
    let img = egui::ColorImage::from_rgba_unmultiplied(size, rgba);
    ctx.load_texture("shot", img, egui::TextureOptions::default())
}

fn fmt_time(sec: f32) -> String {
    let total = sec.max(0.0) as u64;
    let m = total / 60;
    let s = total % 60;
    let ms = ((sec - total as f32) * 1000.0).abs() as u64;
    format!("{:02}:{:02}.{:03}", m, s, ms)
}

use crate::audio::Recorder;
use crate::clicks::{ClickRecorder, ClickSequence};
use crate::config::Config;
use crate::filler::remove_fillers;
use crate::screenshot;
use crate::storage::{CaptureEntry, Session};
use crate::voice_commands::{self, VoiceCommand};
use eframe::egui;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

pub struct NatusApp {
    cfg: Config,
    rt: Arc<Runtime>,
    // recording state
    session: Option<Session>,
    recorder: Option<Recorder>,
    audio_path: Option<PathBuf>,
    recording: bool,
    capturing_sequence: bool,
    sequence_entries: Vec<CaptureEntry>,
    // click recording
    click_recorder: ClickRecorder,
    click_sequences: Vec<ClickSequence>,
    recording_clicks: bool,
    // voice commands
    commands: Vec<VoiceCommand>,
    new_cmd_phrase: String,
    // ui
    tab: Tab,
    status: String,
    review_index: usize,
    transcript: String,
    summary: String,
    // settings fields
    hangar_url: String,
    stt_base: String,
    chat_base: String,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab { Record, Review, Commands, Clicks, Settings }

impl NatusApp {
    pub fn new(cfg: Config, rt: Arc<Runtime>) -> Self {
        let commands = voice_commands::default_commands();
        let hangar_url = cfg.hangar_url.clone();
        let stt_base = cfg.stt_base_url.clone();
        let chat_base = cfg.chat_base_url.clone();
        Self {
            cfg, rt, session: None, recorder: None, audio_path: None, recording: false,
            capturing_sequence: false, sequence_entries: vec![],
            click_recorder: ClickRecorder::new(), click_sequences: vec![], recording_clicks: false,
            commands, new_cmd_phrase: String::new(),
            tab: Tab::Record, status: "Ready".into(), review_index: 0,
            transcript: String::new(), summary: String::new(),
            hangar_url, stt_base, chat_base,
        }
    }
}

impl eframe::App for NatusApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Record, "Record");
                ui.selectable_value(&mut self.tab, Tab::Review, "Review");
                ui.selectable_value(&mut self.tab, Tab::Commands, "Voice Commands");
                ui.selectable_value(&mut self.tab, Tab::Clicks, "Click Sequences");
                ui.selectable_value(&mut self.tab, Tab::Settings, "Settings");
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.tab {
                Tab::Record => self.ui_record(ui),
                Tab::Review => self.ui_review(ui),
                Tab::Commands => self.ui_commands(ui),
                Tab::Clicks => self.ui_clicks(ui),
                Tab::Settings => self.ui_settings(ui),
            }
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.status);
        });
    }
}

impl NatusApp {
    fn ui_record(&mut self, ui: &mut egui::Ui) {
        ui.heading("Recording & Capture");
        if self.session.is_none() {
            if ui.button("New Session").clicked() {
                self.session = Some(Session::new("Feedback session"));
                self.status = "Session created".into();
            }
        } else {
            ui.label(format!("Session: {}", self.session.as_ref().unwrap().meta.id));
        }
        ui.separator();
        ui.horizontal(|ui| {
            if !self.recording {
                if ui.button("Start Audio Recording").clicked() {
                    if let Some(s) = &self.session {
                        let p = self.cfg.session_dir(&s.meta.id).join("audio.wav");
                        let mut rec = Recorder::new(p.clone()).unwrap();
                        if rec.start().is_ok() {
                            self.recorder = Some(rec);
                            self.audio_path = Some(p);
                            self.recording = true;
                            self.status = "Recording audio...".into();
                        }
                    } else { self.status = "Create a session first".into(); }
                }
            } else {
                if ui.button("Stop Audio Recording").clicked() {
                    if let Some(mut rec) = self.recorder.take() {
                        if let Ok(p) = rec.stop() {
                            self.status = format!("Saved audio: {}", p.display());
                        }
                    }
                    self.recording = false;
                }
            }
            if ui.button("Capture Screenshot").clicked() {
                self.do_capture();
            }
            if !self.capturing_sequence {
                if ui.button("Start Capture Sequence").clicked() {
                    self.capturing_sequence = true;
                    self.sequence_entries.clear();
                    self.status = "Capture sequence started. Use 'Capture' or say 'capture'.".into();
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
        if ui.button("Transcribe & Summarize").clicked() {
            self.do_summarize();
        }
        if !self.transcript.is_empty() {
            ui.collapsing("Transcript", |ui| { ui.label(&self.transcript); });
        }
        if !self.summary.is_empty() {
            ui.collapsing("Summary", |ui| { ui.label(&self.summary); });
        }
        // entries list
        if let Some(s) = &self.session {
            ui.separator();
            ui.label(format!("Entries: {}", s.entries.len()));
            for (i, e) in s.entries.iter().enumerate() {
                ui.label(format!("{}: {}", i, e.text));
            }
        }
    }

    fn do_capture(&mut self) {
        let id = self.session.as_ref().map(|s| s.meta.id.clone());
        if id.is_none() { self.status = "Create a session first".into(); return; }
        let id = id.unwrap();
        let dir = self.cfg.session_dir(&id).join(&self.cfg.screenshot_dir_name);
        let ts = chrono::Utc::now().timestamp_millis();
        let path = dir.join(format!("shot_{}.png", ts));
        match screenshot::capture_primary(path.clone()) {
            Ok(p) => {
                let entry = CaptureEntry { timestamp_ms: ts, screenshot_path: Some(p), text: String::new(), cleaned_text: None };
                if let Some(s) = &mut self.session { s.entries.push(entry); let _ = s.save(&self.cfg); }
                self.status = "Screenshot captured".into();
            }
            Err(e) => self.status = format!("Screenshot failed: {}", e),
        }
    }

    fn do_capture_seq(&mut self) {
        let id = self.session.as_ref().map(|s| s.meta.id.clone());
        if id.is_none() { self.status = "Create a session first".into(); return; }
        let id = id.unwrap();
        let dir = self.cfg.session_dir(&id).join(&self.cfg.screenshot_dir_name);
        let ts = chrono::Utc::now().timestamp_millis();
        let path = dir.join(format!("shot_{}.png", ts));
        match screenshot::capture_primary(path.clone()) {
            Ok(p) => {
                let entry = CaptureEntry { timestamp_ms: ts, screenshot_path: Some(p), text: String::new(), cleaned_text: None };
                self.sequence_entries.push(entry);
                self.status = format!("Captured #{}", self.sequence_entries.len());
            }
            Err(e) => self.status = format!("Screenshot failed: {}", e),
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
            } else { String::new() }
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
            if s.entries.is_empty() { ui.label("No entries."); return; }
            let idx = self.review_index.min(s.entries.len().saturating_sub(1));
            self.review_index = idx;
            ui.horizontal(|ui| {
                if ui.button("Prev").clicked() && self.review_index > 0 { self.review_index -= 1; }
                if ui.button("Next").clicked() && self.review_index + 1 < s.entries.len() { self.review_index += 1; }
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
            if let Some(c) = &e.cleaned_text { ui.label(format!("Cleaned: {}", c)); }
        } else {
            ui.label("No session.");
        }
    }

    fn ui_commands(&mut self, ui: &mut egui::Ui) {
        ui.heading("Voice Commands");
        ui.label("Built-in and custom commands. Matching is substring-based on transcript.");
        for c in &self.commands {
            ui.label(format!("- {}", c.phrase));
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.new_cmd_phrase);
            if ui.button("Add 'capture'-style command").clicked() && !self.new_cmd_phrase.trim().is_empty() {
                self.commands.push(voice_commands::VoiceCommand {
                    phrase: self.new_cmd_phrase.clone(),
                    action: voice_commands::CommandAction::Custom { label: self.new_cmd_phrase.clone() },
                });
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
        ui.label("STT base URL");
        ui.text_edit_singleline(&mut self.stt_base);
        ui.label("Chat base URL");
        ui.text_edit_singleline(&mut self.chat_base);
        if ui.button("Save").clicked() {
            self.cfg.hangar_url = self.hangar_url.clone();
            self.cfg.stt_base_url = self.stt_base.clone();
            self.cfg.chat_base_url = self.chat_base.clone();
            let _ = self.cfg.save();
            self.status = "Settings saved".into();
        }
    }
}

fn ctx_texture(ctx: &egui::Context, size: [usize; 2], rgba: &[u8]) -> egui::TextureHandle {
    let img = egui::ColorImage::from_rgba_unmultiplied(size, rgba);
    ctx.load_texture("shot", img, egui::TextureOptions::default())
}

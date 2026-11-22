mod app;
mod audio;
mod clicks;
mod config;
mod filler;
mod hangar;
mod llm;
mod screenshot;
mod stt;
mod storage;
mod voice_commands;

use app::NatusApp;
use config::Config;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = Config::load();
    let rt = Arc::new(Runtime::new().expect("tokio runtime"));
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Natus",
        options,
        Box::new(|_cc| Ok(Box::new(NatusApp::new(cfg, rt)))),
    )
}

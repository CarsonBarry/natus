mod app;
mod audio;
mod clicks;
mod config;
mod filler;
mod hangar;
mod llm;
mod screenshot;
mod storage;
mod stt;
mod theme;
mod tunnel;
mod voice_commands;

use app::NatusApp;
use config::Config;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyManager};
use std::sync::Arc;
use tokio::runtime::Runtime;

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = Config::load();
    let rt = Arc::new(Runtime::new().expect("tokio runtime"));

    // Register a global hotkey for quick capture (Ctrl+Shift+C).
    if let Ok(mgr) = GlobalHotKeyManager::new() {
        let hk = HotKey::new(
            Some(
                global_hotkey::hotkey::Modifiers::CONTROL | global_hotkey::hotkey::Modifiers::SHIFT,
            ),
            global_hotkey::hotkey::Code::KeyC,
        );
        let _ = mgr.register(hk);
        // keep manager alive for process lifetime
        std::mem::forget(mgr);
    }

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Natus",
        options,
        Box::new(|cc| {
            theme::apply(&cc.egui_ctx, theme::palette(true));
            Ok(Box::new(NatusApp::new(cfg, rt)))
        }),
    )
}

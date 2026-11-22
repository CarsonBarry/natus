use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceCommand {
    pub phrase: String,
    pub action: CommandAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CommandAction {
    Capture,
    StartSequence,
    StopSequence,
    StartClickRecording,
    StopClickRecording,
    PlayClickSequence { name: String },
    RunScript { path: PathBuf, args: Vec<String> },
    Custom { label: String },
}

pub fn default_commands() -> Vec<VoiceCommand> {
    vec![
        VoiceCommand { phrase: "capture".into(), action: CommandAction::Capture },
        VoiceCommand { phrase: "start capture sequence".into(), action: CommandAction::StartSequence },
        VoiceCommand { phrase: "stop capture sequence".into(), action: CommandAction::StopSequence },
        VoiceCommand { phrase: "start recording clicks".into(), action: CommandAction::StartClickRecording },
        VoiceCommand { phrase: "stop recording clicks".into(), action: CommandAction::StopClickRecording },
    ]
}

pub fn match_command(text: &str, commands: &[VoiceCommand]) -> Option<VoiceCommand> {
    let t = text.to_lowercase();
    for c in commands {
        if t.contains(&c.phrase.to_lowercase()) {
            return Some(c.clone());
        }
    }
    None
}

pub fn run_script(path: &PathBuf, args: &[String]) -> anyhow::Result<()> {
    std::process::Command::new(path).args(args).spawn()?;
    Ok(())
}

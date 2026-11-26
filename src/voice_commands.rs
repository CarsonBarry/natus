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
        VoiceCommand {
            phrase: "capture".into(),
            action: CommandAction::Capture,
        },
        VoiceCommand {
            phrase: "start capture sequence".into(),
            action: CommandAction::StartSequence,
        },
        VoiceCommand {
            phrase: "stop capture sequence".into(),
            action: CommandAction::StopSequence,
        },
        VoiceCommand {
            phrase: "start recording clicks".into(),
            action: CommandAction::StartClickRecording,
        },
        VoiceCommand {
            phrase: "stop recording clicks".into(),
            action: CommandAction::StopClickRecording,
        },
    ]
}

pub fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn match_command(text: &str, commands: &[VoiceCommand]) -> Option<VoiceCommand> {
    let t = normalize(text);
    for c in commands {
        if t.contains(&normalize(&c.phrase)) {
            return Some(c.clone());
        }
    }
    None
}

pub fn match_commands(text: &str, commands: &[VoiceCommand]) -> Vec<VoiceCommand> {
    let t = normalize(text);
    commands
        .iter()
        .filter(|c| t.contains(&normalize(&c.phrase)))
        .cloned()
        .collect()
}

pub fn run_script(path: &PathBuf, args: &[String]) -> anyhow::Result<()> {
    std::process::Command::new(path).args(args).spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_command_case_insensitive() {
        let cmds = vec![VoiceCommand {
            phrase: "capture".into(),
            action: CommandAction::Capture,
        }];
        let m = match_command("Please CAPTURE now", &cmds).unwrap();
        assert_eq!(m.phrase, "capture");
    }

    #[test]
    fn match_commands_multiple() {
        let cmds = vec![
            VoiceCommand {
                phrase: "capture".into(),
                action: CommandAction::Capture,
            },
            VoiceCommand {
                phrase: "start capture sequence".into(),
                action: CommandAction::StartSequence,
            },
        ];
        let ms = match_commands("capture and start capture sequence", &cmds);
        assert_eq!(ms.len(), 2);
    }

    #[test]
    fn normalize_collapses() {
        assert_eq!(normalize("  Foo   BAR "), "foo bar");
    }
}

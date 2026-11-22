use crate::config::Config;
use std::process::{Child, Command, Stdio};

pub struct SshTunnel {
    child: Option<Child>,
    pub status: String,
}

impl SshTunnel {
    pub fn new() -> Self {
        Self { child: None, status: "Stopped".into() }
    }

    pub fn start(&mut self, cfg: &Config) -> anyhow::Result<()> {
        if self.child.is_some() {
            return Ok(());
        }
        let spec = format!(
            "{}:127.0.0.1:{}",
            cfg.ssh_tunnel_local_port, cfg.ssh_tunnel_remote_port
        );
        let target = format!("{}@{}", cfg.ssh_tunnel_user, cfg.ssh_tunnel_host);
        let child = Command::new("ssh")
            .args(["-N", "-L", &spec, &target])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        self.child = Some(child);
        self.status = "Running".into();
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        self.status = "Stopped".into();
    }

    pub fn is_running(&mut self) -> bool {
        if let Some(c) = &mut self.child {
            match c.try_wait() {
                Ok(Some(_)) => {
                    self.child = None;
                    self.status = "Exited".into();
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.stop();
    }
}
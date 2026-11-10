//! Talks to the root helper (`relink-helper`, Linux) through `sudo -n`.
//! Absent or not allowed (no sudoers entry), every call reports so and the
//! app falls back to what works without privileges.

use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

pub const PATH: &str = "/usr/local/bin/relink-helper";

/// Whether the helper can run, re-probed every few minutes.
#[derive(Default)]
pub struct Helper {
    available: bool,
    probed: Option<Instant>,
}

impl Helper {
    pub fn available(&mut self) -> bool {
        if !cfg!(target_os = "linux") {
            return false;
        }
        if self
            .probed
            .is_none_or(|t| t.elapsed() > Duration::from_secs(300))
        {
            self.probed = Some(Instant::now());
            self.available = std::path::Path::new(PATH).exists() && run(&["probe"]).is_ok();
        }
        self.available
    }

    pub fn route_add(&self, ip: Ipv4Addr) -> Result<String, String> {
        run(&["route-add", &ip.to_string()])
    }

    pub fn kill_socket(
        &self,
        local: Ipv4Addr,
        local_port: u16,
        remote: Ipv4Addr,
        remote_port: u16,
    ) -> Result<String, String> {
        run(&[
            "kill-socket",
            &local.to_string(),
            &local_port.to_string(),
            &remote.to_string(),
            &remote_port.to_string(),
        ])
    }
}

fn run(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("sudo")
        .arg("-n")
        .arg(PATH)
        .args(args)
        .output()
        .map_err(|e| format!("sudo: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

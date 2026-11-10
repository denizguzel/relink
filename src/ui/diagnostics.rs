//! Text built from the window's state: the bug-report dump for the
//! clipboard, and the normalisation of names typed into list editors.

use super::{Ui, status_text};

/// Everything a bug report needs, as text for the clipboard: version, status,
/// VPN adapter, connections, hosts and the latest events.
pub fn diagnostics(s: &Ui) -> String {
    use std::fmt::Write;
    let mut o = String::new();
    let p = s.published.as_ref();
    let _ = writeln!(
        o,
        "Re-Link {} ({} {})",
        s.version,
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    let _ = writeln!(
        o,
        "language={} notifications={} interval={}s autoFix={}",
        s.cfg.language, s.cfg.notifications, s.cfg.interval_seconds, s.cfg.auto_fix
    );
    let _ = writeln!(
        o,
        "status: {}",
        status_text(s.status(), p.map(|p| &p.report))
    );
    if let Some(p) = p {
        match &p.report.vpn {
            Some(a) => {
                let _ = writeln!(o, "vpn: {} (if {}) {:?}", a.description, a.index, a.addrs);
            }
            None => {
                let _ = writeln!(o, "vpn: not connected");
            }
        }
        let _ = writeln!(
            o,
            "checked {}s ago, settling={}",
            s.now.saturating_sub(p.checked_at),
            p.report.settling
        );
        let _ = writeln!(o, "\n-- browser connections");
        for (c, rt, kind) in &p.report.conns {
            let route = rt.map_or("-".to_string(), |r| {
                format!("/{} if={}", r.prefix_len, r.if_index)
            });
            let _ = writeln!(
                o,
                "{} pid={} {}:{} -> {}:{}  {route}  {kind:?}",
                c.process, c.pid, c.local, c.local_port, c.remote, c.remote_port
            );
        }
        let _ = writeln!(o, "\n-- watched hosts ({})", p.hosts.len());
        for h in &p.hosts {
            let ips: Vec<String> = h.ips.iter().map(|i| i.to_string()).collect();
            let _ = writeln!(o, "{} [{}] {:?}", h.host, ips.join(", "), h.kind);
        }
    }
    let _ = writeln!(o, "\n-- recent events");
    for e in s.events.iter().take(25) {
        let (y, m, d) = e.day;
        let _ = writeln!(o, "{y}-{m:02}-{d:02} {} [{:?}] {}", e.time, e.level, e.text);
    }
    o
}

/// A host or domain typed into a list editor, normalised; None when it is
/// empty or not a plausible name.
pub fn clean_name(input: &str) -> Option<String> {
    let s = input.trim().to_lowercase();
    let s = s
        .strip_prefix("https://")
        .or_else(|| s.strip_prefix("http://"))
        .unwrap_or(&s);
    let s = s
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim_start_matches("*.")
        .trim_matches('.');
    let ok = !s.is_empty()
        && s.len() <= 253
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.');
    ok.then(|| s.to_string())
}

#[cfg(test)]
mod name_tests {
    use super::clean_name;

    #[test]
    fn names_are_normalised_or_rejected() {
        assert_eq!(
            clean_name("  HTTPS://Metrics.Etiya.com/path?x=1 "),
            Some("metrics.etiya.com".into())
        );
        assert_eq!(clean_name("*.etiya.com"), Some("etiya.com".into()));
        assert_eq!(clean_name(".etiya.com."), Some("etiya.com".into()));
        assert_eq!(clean_name("two words"), None);
        assert_eq!(clean_name("   "), None);
    }
}

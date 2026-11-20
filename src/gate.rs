//! Whether a company host turns away requests that do not come through the
//! VPN. When FortiClient never installs a route for a load balancer, every
//! address of its hosts goes over the home network and `diagnose` cannot tell
//! them from public sites, unless Re-Link learned them earlier (a fresh
//! install has not). The load balancer itself can tell: its IP allow list
//! answers `403` with `Server: awselb/…`, while the application behind it
//! answers with its own server. One plain HTTPS request shows which it is.
//!
//! `answer` and `targets` are pure; `ask` runs the operating system's `curl`.

use std::collections::HashSet;
use std::process::Stdio;

use crate::diagnose::{HostKind, Report};

/// What a host said to a request from outside the VPN.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The load balancer turned it away: the host belongs on the VPN.
    Gated,
    /// Something else answered: a public site, as far as this shows.
    Open,
    /// No answer (timeout, TLS or name error): proves nothing.
    Unknown,
}

/// How long one request may take, in seconds.
const MAX_TIME: &str = "8";

/// Reads `curl --dump-header` output. Only the last response counts: a proxy
/// puts its own `200 Connection established` block before it.
pub fn answer(headers: &str) -> Answer {
    let Some(start) = headers.rfind("HTTP/") else {
        return Answer::Unknown;
    };
    let mut lines = headers[start..].lines();
    let status = lines
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok());
    let Some(status) = status else {
        return Answer::Unknown;
    };
    let server = lines
        .take_while(|l| !l.trim().is_empty())
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("server"))
        .map(|(_, v)| v.trim().to_ascii_lowercase());
    if status == 403 && server.is_some_and(|s| s.starts_with("awselb")) {
        Answer::Gated
    } else {
        Answer::Open
    }
}

/// Watched hosts worth asking: the VPN is up and settled, the host goes over
/// the home network (`Internet`), and it is not already known to belong on
/// the VPN (then `diagnose` reports it by itself).
pub fn targets(r: &Report, required: &HashSet<String>) -> Vec<String> {
    if r.vpn.is_none() || r.settling {
        return Vec::new();
    }
    r.hosts
        .iter()
        .filter(|(h, _, kind)| *kind == HostKind::Internet && !required.contains(h))
        .map(|(h, _, _)| h.clone())
        .filter(|h| plain_host(h))
        .collect()
}

/// A bare DNS name, so it cannot change the meaning of the URL built from it.
fn plain_host(h: &str) -> bool {
    !h.is_empty()
        && h.len() <= 253
        && h.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
}

/// Asks `host` once, over whatever route the system picks (as the browser
/// would). Redirects are not followed: the first answer is the one that counts.
pub fn ask(host: &str) -> Answer {
    if !plain_host(host) {
        return Answer::Unknown;
    }
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let out = crate::platform::curl()
        .args(["--silent", "--proto", "=https", "--max-time", MAX_TIME])
        .args(["--output", null, "--dump-header", "-"])
        .args(["--user-agent", "relink-gate"])
        .arg(format!("https://{host}/"))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match out {
        Ok(o) => answer(&String::from_utf8_lossy(&o.stdout)),
        Err(_) => Answer::Unknown,
    }
}

/// Asks every host at once (one `curl` each) and returns their answers.
pub fn ask_all(hosts: &[String]) -> Vec<(String, Answer)> {
    let tries: Vec<_> = hosts
        .iter()
        .map(|h| {
            let h = h.clone();
            std::thread::spawn(move || {
                let a = ask(&h);
                (h, a)
            })
        })
        .collect();
    tries.into_iter().filter_map(|t| t.join().ok()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnose::Adapter;
    use std::net::Ipv4Addr;

    #[test]
    fn the_load_balancers_own_403_means_gated() {
        let h = "HTTP/1.1 403 Forbidden\r\nServer: awselb/2.0\r\nDate: x\r\n\r\n";
        assert_eq!(answer(h), Answer::Gated);
        let h2 = "HTTP/2 403 \r\nserver: awselb/2.0\r\n\r\n";
        assert_eq!(answer(h2), Answer::Gated);
    }

    #[test]
    fn an_applications_answer_is_open() {
        for h in [
            "HTTP/1.1 403 Forbidden\r\nServer: nginx\r\n\r\n",
            "HTTP/1.1 403 Forbidden\r\n\r\n",
            "HTTP/1.1 503 Service Temporarily Unavailable\r\nServer: awselb/2.0\r\n\r\n",
            "HTTP/1.1 302 Found\r\nlocation: /oauth2\r\nserver: istio-envoy\r\n\r\n",
            "HTTP/1.1 200 OK\r\nServer: nginx\r\n\r\n",
        ] {
            assert_eq!(answer(h), Answer::Open, "{h}");
        }
    }

    #[test]
    fn behind_a_proxy_the_last_response_counts() {
        let h = "HTTP/1.1 200 Connection established\r\n\r\nHTTP/1.1 403 Forbidden\r\nServer: awselb/2.0\r\n\r\n";
        assert_eq!(answer(h), Answer::Gated);
    }

    #[test]
    fn no_response_proves_nothing() {
        assert_eq!(answer(""), Answer::Unknown);
        assert_eq!(answer("HTTP/1.1 abc\r\n"), Answer::Unknown);
    }

    fn report(hosts: &[(&str, HostKind)]) -> Report {
        Report {
            vpn: Some(Adapter {
                index: 11,
                description: "Fortinet SSL VPN".into(),
                up: true,
                addrs: vec![Ipv4Addr::new(10, 0, 0, 2)],
            }),
            hosts: hosts
                .iter()
                .map(|(h, k)| (h.to_string(), vec![Ipv4Addr::new(1, 2, 3, 4)], *k))
                .collect(),
            ..Default::default()
        }
    }

    #[test]
    fn only_home_network_hosts_not_yet_known_are_asked() {
        let r = report(&[
            ("bss.example.com", HostKind::Internet),
            ("known.example.com", HostKind::Internet),
            ("vpn.example.com", HostKind::Vpn),
            ("missing.example.com", HostKind::Missing),
            ("bad host/x", HostKind::Internet),
        ]);
        let required: HashSet<String> = ["known.example.com".to_string()].into();
        assert_eq!(targets(&r, &required), ["bss.example.com"]);
    }

    #[test]
    fn nothing_is_asked_without_a_settled_vpn() {
        let mut r = report(&[("bss.example.com", HostKind::Internet)]);
        r.settling = true;
        assert!(targets(&r, &HashSet::new()).is_empty());
        r.settling = false;
        r.vpn = None;
        assert!(targets(&r, &HashSet::new()).is_empty());
    }

    #[test]
    fn an_odd_name_is_never_requested() {
        assert_eq!(ask("evil.com/@x"), Answer::Unknown);
        assert_eq!(ask(""), Answer::Unknown);
    }
}

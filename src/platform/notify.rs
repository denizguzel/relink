//! Desktop notifications: rune-notify toasts on Windows, notify-send elsewhere.

#[cfg(not(windows))]
type ClickHandler = Box<dyn Fn(&str) + Send + Sync>;
#[cfg(not(windows))]
static ON_CLICK: std::sync::OnceLock<ClickHandler> = std::sync::OnceLock::new();

/// Calls `f` with a clicked notification's action ("fix", "open", or
/// "default" for the body), from any thread.
pub fn on_notification_click(f: impl Fn(&str) + Send + Sync + 'static) {
    #[cfg(windows)]
    rune_notify::on_click(move |click| f(click.get("action").unwrap_or("default")));
    #[cfg(not(windows))]
    let _ = ON_CLICK.set(Box::new(f));
}

/// Posts a desktop notification. `action` names what a click should do and
/// `action_label` is its button text.
pub fn notify(id: &str, title: &str, body: &str, action: Option<(&str, &str)>) {
    #[cfg(windows)]
    {
        let mut note = rune_notify::Notification::new(id, title).body(body);
        if let Some((a, _)) = action {
            note = note.info("action", a);
        }
        rune_notify::post(&note);
    }
    #[cfg(not(windows))]
    {
        // notify-send ≥ 0.7.9 waits while actions are offered and prints the chosen one.
        let mut cmd = std::process::Command::new("notify-send");
        cmd.args(["-a", "Re-Link", "-u", "normal"]);
        if let Some((a, label)) = action {
            cmd.arg("-A")
                .arg(format!("default={label}"))
                .arg("-A")
                .arg(format!("{a}={label}"));
        }
        cmd.arg(title).arg(body);
        let has_action = action.is_some();
        let fallback = action.map(|(a, _)| a.to_string());
        let _ = id;
        std::thread::spawn(move || {
            let Ok(out) = cmd.output() else { return };
            if !has_action {
                return;
            }
            let picked = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let picked = if picked == "default" {
                fallback.unwrap_or_default()
            } else {
                picked
            };
            if !picked.is_empty()
                && let Some(f) = ON_CLICK.get()
            {
                f(&picked);
            }
        });
    }
}

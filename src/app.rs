//! The tray app: the window, the monitor thread, the tray and notifications.

use std::sync::{Arc, Mutex, mpsc};

use relink::config::{Config, Paths, WindowGeom};
use relink::monitor::{Log, Monitor};
use relink::tray::TrayHost;
use relink::ui::{self, Cmd, Event, Level, Link, Ui};
use relink::{APP_ID, VERSION, brand, platform, t};

/// The window's first size, set before the window opens: the saved one, or
/// one computed from the screen.
static WINDOW_SIZE: std::sync::OnceLock<(f32, f32)> = std::sync::OnceLock::new();

/// The window's description; also what the headless snapshots render.
pub fn spec() -> rune::AppSpec<Ui> {
    rune::AppSpec {
        scenes: ui::demo::SCENES,
        fonts: rune_theme_opencode::FONTS,
        families: rune_theme_opencode::FAMILIES,
        poll: Some(ui::poll),
        on_key: Some(ui::on_key),
        min_size: Some((760.0, 520.0)),
        ..rune::AppSpec::new(
            "Re-Link",
            *WINDOW_SIZE.get().unwrap_or(&(1000.0, 680.0)),
            Ui::default,
            ui::view,
        )
    }
}

/// Chooses the first window size: the saved one, else one fitted to the screen.
pub fn choose_window_size(paths: &Paths) {
    let size = platform::pick_size(
        WindowGeom::load(&paths.window),
        platform::screen_work_area(),
    );
    let _ = WINDOW_SIZE.set(size);
}

/// Writes the default settings and the app icon on first run, and gives the
/// process its notification identity.
fn prepare_files(paths: &Paths, cfg: &Config, cfg_err: &Option<String>, log: &Log) {
    if cfg_err.is_none()
        && !paths.config.exists()
        && let Err(e) = cfg.save(&paths.config)
    {
        log.line(&format!("write default config: {e}"));
    }
    let icon = paths.dir.join("relink.ico");
    // Rewritten when it differs, so an update brings the new icon.
    if std::fs::read(&icon).ok().as_deref() != Some(brand::ICON_ICO) {
        let _ = std::fs::write(&icon, brand::ICON_ICO);
    }
    if let Err(e) =
        rune_notify::register(&rune_notify::Identity::new(APP_ID, "Re-Link").icon(&icon))
    {
        log.line(&format!("notifications unavailable: {e}"));
    }
}

/// Runs the app until the user quits.
/// `updated`: started by the in-app update, which the activity log says.
pub fn run(paths: Paths, cfg: Config, cfg_err: Option<String>, hidden: bool, updated: bool) {
    // A start at login stays in the tray unless the user asked for the window.
    let hidden = hidden && cfg.start_hidden;
    // A second launch brings the running window forward and exits.
    if !rune::single_instance(APP_ID) {
        return;
    }
    let log = Log::open(paths.log.clone());
    log.line(&format!(
        "Re-Link {VERSION} starting, config {}",
        paths.config.display()
    ));
    prepare_files(&paths, &cfg, &cfg_err, &log);
    // An in-app update replaced only the executable: show its version in
    // Windows' "Installed apps" too.
    if let Some(was) = platform::sync_install_entry(&paths.exe, VERSION) {
        log.line(&format!("installed apps entry updated: {was} -> {VERSION}"));
    }

    let shared = Arc::new(Mutex::new(cfg.clone()));
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
    let link = Link {
        cmd: cmd_tx.clone(),
        cfg: shared.clone(),
        config_path: paths.config.clone(),
        log_path: paths.log.clone(),
        window_path: paths.window.clone(),
        exe: paths.exe.clone(),
    };
    let startup = platform::startup_enabled(&paths.exe);
    let history = relink::monitor::load_history(&paths.log, ui::MAX_EVENTS);
    let mut first = vec![(t!("ev_started", version = VERSION), Level::Info)];
    if updated {
        first.push((t!("ev_updated", version = VERSION), Level::Good));
    }
    if let Some(e) = cfg_err {
        first.push((t!("ev_config_error", error = e), Level::Warn));
    }
    for (text, level) in &first {
        log.event(text, *level);
    }

    let _ = rune::set_app_icon(brand::ICON_PNG);
    let mut app = rune::App::new(spec()).on_close(ui::on_close).init(move || {
        let mut s = Ui {
            version: VERSION,
            cfg,
            startup,
            vpn_client: platform::vpn_client_available(),
            link: Some(link),
            now: relink::config::now_unix(),
            ..Ui::default()
        };
        // Earlier sessions first (oldest at the back), then this start.
        s.events = history
            .into_iter()
            .map(|(ts, text, level)| Event::new(ts, text, level))
            .collect();
        for (text, level) in first {
            s.push_event(text, level);
        }
        s
    });
    let handle = app.handle();

    let tray = TrayHost::new(handle.clone(), cmd_tx.clone());
    ui::HAS_TRAY.store(tray.is_some(), std::sync::atomic::Ordering::Relaxed);
    app = match (&tray, hidden) {
        // Closing the window keeps Re-Link in the tray and gives the GPU back.
        (Some(_), true) => app.windowless().release_gpu_when_closed(),
        (Some(_), false) => app.keep_running().release_gpu_when_closed(),
        // No tray: closing must quit, or there would be no way back.
        (None, _) => app,
    };

    let (click_handle, click_cmd) = (handle.clone(), cmd_tx.clone());
    platform::on_notification_click(move |action| {
        if action == "fix" {
            let _ = click_cmd.send(Cmd::FixNow);
        }
        click_handle.open();
    });

    let monitor = Monitor::new(handle, shared, paths, tray.clone(), log);
    std::thread::Builder::new()
        .name("relink-monitor".into())
        .spawn(move || monitor.run(cmd_rx))
        .expect("spawn monitor");

    app.run();
    drop(tray);
}

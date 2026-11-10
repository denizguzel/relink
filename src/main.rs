//! Re-Link's entry point: the command-line modes, or the tray app.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod cli;

use std::process::ExitCode;

use relink::config::{Config, Paths};
use relink::{VERSION, platform};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);

    // Headless snapshots and other Rune tooling flags: just the window.
    if has("--snap") || has("--tree") || has("--scene") {
        rune::run(app::spec());
        return ExitCode::SUCCESS;
    }

    let paths = Paths::detect();
    app::choose_window_size(&paths);
    let (cfg, cfg_err) = Config::load(&paths.config);
    relink::i18n::apply(&cfg.language);

    if ["--version", "--check", "--dump", "--fix", "--help", "-h"]
        .iter()
        .any(|f| has(f))
    {
        platform::attach_console();
    }
    if has("--version") {
        println!("Re-Link {VERSION}");
        return ExitCode::SUCCESS;
    }
    if has("--help") || has("-h") {
        cli::help();
        return ExitCode::SUCCESS;
    }
    if has("--dump") {
        cli::dump();
        return cli::check(&paths, &cfg, cfg_err, true);
    }
    if has("--check") {
        return cli::check(&paths, &cfg, cfg_err, has("-v"));
    }
    if has("--fix") {
        return cli::fix(&cfg);
    }

    // Started by an update: let the old version finish quitting first, then
    // delete it.
    if let Some(pid) = args
        .iter()
        .position(|a| a == relink::update::WAIT_ARG)
        .and_then(|i| args.get(i + 1))
        .and_then(|p| p.parse().ok())
    {
        platform::wait_for_exit(pid, std::time::Duration::from_secs(20));
    }
    relink::update::remove_old();

    app::run(paths, cfg, cfg_err, has(platform::HIDDEN_ARG));
    ExitCode::SUCCESS
}

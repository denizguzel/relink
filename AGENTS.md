# Re-Link — tray app for Windows and Linux (Rust + Rune)

Single portable `relink.exe` that detects browser sockets stuck outside the
FortiClient split tunnel and restarts Chromium's Network Service to clear them.
Developed in WSL, cross-compiled for Windows. README.md has the user-facing story.
A Go version preceded it; it was removed once this one reached parity.

## Build and verification

```sh
make test       # cargo test (pure modules; runs on Linux)
make build      # cargo xwin → target/x86_64-pc-windows-msvc/release/relink.exe
make lint       # cargo fmt --check + clippy -D warnings (what CI runs)
make package    # every release file that can be built here → dist/release/
make linux      # cargo zigbuild (glibc 2.31) → dist/release/relink-<v>-linux-x86_64{,.tar.gz}
make desktop    # build, then replace the portable copy on the Windows Desktop and start it
make check      # copy to dist/relink-windows and run --check -v (read-only)
make snaps      # render every demo scene headless to target/snaps/*.png
```

Linux builds (tests, snapshots) link libxkbcommon and libwayland-client; without
the -dev packages, `make` points `-L` at `target/linklibs` symlinks to the .so.0
files. Snapshots use Mesa's lavapipe (`VK_ICD_FILENAMES=.../lvp_icd.json`).

Never run `--fix` or the tray against the user's real browser without asking:
it kills their Chrome/Edge network service.

## Architecture

- `diagnose`: pure detection rules over a `Snapshot` (stuck sockets, missing
  VPN routes, per-connection and per-host classification). Unit-tested.
- `engine`: pure, time-injected policy: when to notify, auto-fix, or fix after
  a VPN reconnect. Unit-tested.
- `config`: `relink.json` (settings, camelCase, defaults for missing fields),
  `relink-state.json` (learned VPN hosts, 7-day TTL), APPDATA paths (portable only if relink.json sits beside the exe).
- `bookmarks`: company hosts from Chrome/Edge Bookmarks files.
- `i18n` + `locales/`: `rune-i18n` catalogs (one JSON per language, compiled by
  `build.rs`, keys checked at compile time via `t!`). Add every key to both.
- `net`: `net/windows.rs` (iphlpapi/toolhelp/ntdll via `windows-sys`: TCP owner
  table, `GetBestRoute2`, adapters, command lines, terminate) and `net/linux.rs`
  (`/proc/net/tcp` + `/proc/<pid>/fd`, `ip -j` addr/route; parsers unit-tested).
- `helper` + `src/bin/relink-helper.rs` (Linux): the root helper, run via
  `sudo -n` after `scripts/install-linux.sh`. Adds /32 routes through the VPN
  tunnel (`proto 222`) and closes single sockets (`ss -K`). Engine emits
  `AddRoutes` instead of reconnect advice when it is available.
- `monitor/`: background thread. `mod.rs` is the loop; `check.rs` one check
  (collect, analyze, publish), `actions.rs` what the engine decided, `settle.rs`
  waiting for the VPN's routes (adaptive, drives the progress bar), `names.rs`
  rate-limited name resolution, `notify.rs` toasts filtered by level, `log.rs`
  the log file and the history reloaded at start, `bookmark_cache.rs`,
  `pause.rs` the notification pause.
- `tray`: Rune tray; rebuilt when the status colour changes (no in-place icon API).
- `ui/`: Rune + rune-kit (OpenCode theme) window. `state.rs` the window state,
  `shell.rs` layout, `sidebar.rs`, `status.rs` status texts, `diagnostics.rs`
  the copy-to-clipboard bug report, `tick.rs` per-second work (window position
  memory), `input.rs` shortcuts, `pages/` one module per page (`overview/` is
  split further: hero, welcome, paused, stats, details, problems; `hosts/` has
  `explain.rs`/`list.rs`, `connections/group.rs`, `activity/days.rs`: the pure
  logic of a page lives beside it with its tests), `icons.rs` (generated: Lucide
  only, by `scripts/lucide-icons.py`; add a name there instead of drawing or
  mixing other icon sets), `close.rs` the first-close
  tray notice, `pause.rs`, `demo.rs` the headless scenes (also used by tests).
- `platform/`: `startup` (HKCU Run / XDG autostart), `desktop` (open file,
  console), `time`, `window` (screen/window geometry via Win32), `notify`
  (rune-notify toasts / `notify-send -A`).
- `src/main.rs` only dispatches; `src/app.rs` runs the tray app, `src/cli.rs`
  the `--check/--dump/--fix` modes.
- `build.rs`: message catalogs, and the exe icon + version resource on Windows.

## Brand

The mark is a Lucide glyph (`link`) with a gradient stroke (indigo → blue →
cyan) on a quiet bordered tile; the wordmark is Inter semibold with tight tracking. `scripts/brand/build.mjs` is the single description: it renders
everything in `assets/brand/` (app icon `.ico`/PNG, the tray tile per status as raw
RGBA, README/social banner, installer wizard images). Run it after changing the
design (`cd scripts/brand && npm install && npm run build`) and commit the output;
`src/brand.rs` only embeds the files and holds the status colours (keep them in
sync with `STATUS` in the script). In the window the logo is the same glyph on a
tile, tinted with the status colour. The GitHub social preview
(`assets/brand/social-preview.png`) is uploaded by hand in the repository settings.

## Distribution and updates

Releases are GitHub Releases built by `.github/workflows/release.yml` on a `v*`
tag (`RELEASING.md`; `scripts/release.sh` bumps, commits and tags, never pushes;
`scripts/package.py` makes every release file, locally and in CI). Windows
installs per user only (`packaging/windows/relink.iss`, no administrator rights,
no HKLM; CI checks this). `update/` checks the latest release through the OS's
`curl`, and `Update and restart` swaps the running executable (rename to `.old`,
verified against the release's `SHA256SUMS`, new process waits for the old one
with `--wait-pid`). `release.rs`, `asset.rs`, `version.rs` and `install.rs`'s swap are
pure and tested; `monitor/updates.rs` schedules the checks.

## Code structure

Put decisions in small pure functions (clock and state passed in) and test them
next to the code; `make test` must stay green on Linux. The UI is tested through
the demo scenes (`ui/tests.rs`). The code is formatted with the default `cargo fmt`
(no rustfmt.toml); run it before finishing a change.

Keep modules small and single-purpose (aim for well under ~300 lines; split a
file or a long function before it becomes a catch-all). Platform differences go
behind `cfg` inside `platform/` and `net/`, not scattered through the app.

## Skills

Repeatable workflows live in `.agents/skills/<name>/SKILL.md`:
`relink-deploy` (build, stop, copy to the Desktop, start), `relink-verify`
(fmt, tests, snapshots, both builds), `relink-diagnose` (investigating a bug
report), `add-message`, `add-icon`, `add-demo-scene`.

## Contracts

- Windows: no administrator rights (no route changes, no hosts file, no `SetTcpEntry`).
  Linux: privileged work only in `relink-helper`; keep it small, std + serde_json,
  absolute tool paths, validated arguments, VPN interface detected inside it.
- Linux binaries must stay on glibc ≤ 2.31 (Ubuntu 20.04 is the oldest supported) (`make linux`); check with
  `objdump -T … | grep GLIBC_`.
- Don't depend on `powershell.exe`/`pwsh.exe` (often blocked by policy on managed PCs).
- Tray changes happen off the tray's own thread (dropping a tray joins it).
- Rune is pinned to the rev Hex uses; bump all `rune*` crates together.

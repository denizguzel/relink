# Changelog

What changed in each Re-Link release, newest first. Each section is also that
release's notes on GitHub: the release workflow publishes it, and
`scripts/release.sh` refuses to tag a version that has none.

## 0.6.1 — 2026-11-20

### Fixes

- Connecting the VPN no longer raises a false "company addresses outside the
  VPN" alarm. FortiClient adds its last routes 30–45 seconds after connecting
  (often for addresses that changed when DNS switched to the company's
  servers); Re-Link now waits for them while hosts are still missing, and goes
  green as soon as everything is in place.
- The progress bar while the VPN connects no longer jumps backwards (38/39,
  then 6/39): it counts the ready addresses afresh at every check, and names
  are looked up again every few seconds while the routes arrive.
- Routes that arrive after a warning turn the status green at once, instead of
  another 20 seconds of "preparing".
- The same missing addresses are announced once, even when their names resolve
  to other addresses of the same service.

## 0.6.0 — 2026-11-20

### New

- **A VPN that is connected but not working.** After sleep or a network switch,
  FortiClient can say *connected* while nothing gets through. Re-Link now tries
  a few company addresses now and then; when none answers, the tray icon turns
  red, the Overview says "VPN is connected but not working" and a notification
  opens FortiClient. `checkTunnel: false` in `relink.json` turns it off.
- **After lock or sleep.** When the VPN dropped while the computer was locked
  (Win+L) or asleep, Re-Link tells you the moment you are back; a click opens
  FortiClient. It can also open FortiClient at once, or do nothing (Settings →
  After lock or sleep). On Linux it reads the screen lock from logind and
  notifies.
- **VPN session time.** The Overview shows how long the VPN has been connected,
  and so does the tray tooltip.
- **Warning before the session ends.** Once Re-Link has seen sessions end after
  a fixed time (8 hours, say), it shows the time left and warns about 10
  minutes before.
- The welcome screen and the help sheet cover the new choices.

### Fixes

- The window opened at its minimum size every time: a hidden helper window's
  size had been saved as the app's. It now opens where and how large you left
  it.
- Clicking the tray icon now brings the window to the front.
- A missing route that fixed itself no longer makes the next VPN reconnect
  restart every browser's network service.
- Fewer checks while the VPN stays down (every few seconds after the first two
  minutes), so less battery.
- Pausing or checking for updates no longer delays the next check.
- If the previous version hangs after an update, the new one ends it instead of
  quitting too.

### Notes

- Linux: the root helper changed (it now replaces only routes it added
  itself). Rerun `sudo ./install-linux.sh` from the new archive; the in-app
  update does not replace the helper.

## 0.5.1 — 2026-11-10

### Fixes

- Windows "Installed apps" shows the version actually running after an in-app
  update (the update replaces only the program).
- The updater runs `curl.exe` by its full System32 path, logs "Updated to X"
  after an update, and announces the same release only once.

## 0.5.0 — 2026-11-10

### New

- **Connect VPN** (Windows): while the VPN is down, the Overview and the tray
  menu open FortiClient on its Remote Access page.
- **Check now** beside "Check for updates", working even when the periodic
  check is off; the first check runs right at start.

### Fixes

- The status line says "Watching live" while checking every second and "just
  now" / "N min ago" otherwise, instead of ticking every second.
- The progress bar while the VPN's routes arrive hides once every address is
  ready.

## 0.4.0 — 2026-11-10

### New

- Redesigned Overview: the browser → VPN → company path in the status card, a
  slim counter strip, recent activity beside it, technical facts folded away.
- Help sheet explaining the tray icon colours; a welcome screen with the
  everyday choices.
- Zoom (Ctrl +, Ctrl −, Ctrl 0, and in Settings).
- Settings: keep running in the tray when the window closes, start hidden at
  login.

### Fixes

- The wait for the VPN's routes also starts when routes vanish while the VPN
  stays up, and it no longer reads as healthy too early.
- Plainer texts throughout.

## 0.3.0 — 2026-11-10

### New

- A new release is announced under "Check for updates" in Settings, with the
  install button, instead of a banner on the Overview.
- The sidebar hints at a new version, and pressing the version copies it.

### Fixes

- "Installed apps" lists the app as "Re-Link" instead of "Re-Link version x.y.z".

## 0.2.1 — 2026-11-10

The first public release: Re-Link finds browser connections stuck outside the
FortiClient split tunnel and restarts the browser's network service to clear
them, without closing tabs.

- Windows installer (per user, no administrator rights) and portable zip; Linux
  archive with an optional root helper.
- In-app updates from GitHub Releases, verified against `SHA256SUMS`.
- Light and dark themes; Turkish and English.
- The window opens centred on the monitor you are working on, and a desktop
  shortcut is added.

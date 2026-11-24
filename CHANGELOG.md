# Changelog

What changed in each Re-Link release, newest first. Each section is also that
release's notes on GitHub: the release workflow publishes it, and
`scripts/release.sh` refuses to tag a version that has none.

## 0.8.1 — 2026-11-24

### Fixes

- **"VPN connected for" counted the night the computer was off.** After turning
  the computer off in the evening and on in the morning, Re-Link kept the
  previous session's start and showed a connection time of 20 hours, though the
  VPN had just been connected. Re-Link now remembers when it last saw the VPN up
  and starts the time afresh when it was away for more than a few minutes. The
  session-limit warning no longer learns from these wrong times either.

### Notes

- The first start after updating shows the time from that moment, once.

## 0.8.0 — 2026-11-23

### New

- **More browsers.** Comet, Opera, Vivaldi and Yandex Browser are watched like
  Chrome and Edge: their network service is restarted when a connection is
  stuck, and their bookmarks count towards the company addresses.
- **Restart Firefox from Re-Link.** Firefox keeps its sockets in the main
  process, so a stuck connection cannot be closed without administrator
  rights. When only a restart helps, the Overview now shows a warning with the
  steps and a **Restart Firefox** button: it ends Firefox and starts it again
  with the same profile, and Firefox restores its tabs. Finish anything you
  are typing or uploading first. The toast says the same.

## 0.7.0 — 2026-11-20

### New

- **Company addresses the VPN never routed.** When FortiClient sets up no
  route for a company service, every connection to it goes over the home
  network and its pages return 403, yet a fresh install saw nothing wrong: it
  could not tell those addresses from public sites. While the VPN is up,
  Re-Link now sends one HTTPS request to watched addresses that go over the
  home network; when the company's load balancer turns it away (its own 403),
  the address is known to belong on the VPN and Re-Link says to reconnect the
  VPN. Public sites are asked again at most every 6 hours. `relink --check`
  does the same, without saving anything.
- **"Check now" says what it checked.** After a press, the Overview shows for a
  few seconds how many browser connections and company addresses were checked
  and what came out.

### Fixes

- The "addresses outside the VPN" texts no longer claim the IP address changed;
  the VPN may also have never set up the route.

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

# Re-Link

A tray app for Windows and Linux that keeps browsers working while a
FortiClient VPN split tunnel is on.

While FortiClient VPN is connected, some internal pages in Chrome, Edge and Firefox
intermittently return **403/400**. Reconnecting the VPN doesn't help; only fully
restarting the browser does. Re-Link watches for this quietly in the tray and fixes it
without closing the browser.

## Why it happens

FortiClient's split tunnel adds `/32` routes for specific hosts only. If the browser
connected to a site **before** such a route existed (or the load balancer's IPs
changed), the open TCP connection stays bound to the home network interface. The
browser keeps reusing it, so requests leave over the home internet instead of the VPN,
and the server doesn't recognise the address and answers 403. Reconnecting the VPN does
not clear the browser's open connections or its DNS cache; closing the browser does.

## What it does

- Every 15 seconds (every second while the VPN is down or still installing its routes) it
  looks at the browsers' open connections. A connection opened from the **home network**
  to an address Windows would now send through the **VPN** counts as stuck. No host list
  is needed for this: it works for private addresses, for the `/32` host routes FortiClient
  adds one by one, and for the addresses of watched hosts. Public addresses that only match
  a broad VPN range (a `/17` CDN range, say) are not counted, because they work over the
  home network too.
- On a stuck connection it shows a notification. Clicking it (or, with *Fix automatically*,
  nothing at all) restarts the **Network Service** process of Chrome/Edge/Brave. Chromium
  brings that process back in a second or two: **tabs stay open**, only the stale
  connections and the browser's DNS cache are dropped.
- It resolves the hosts in `relink.json` and the company hosts found in your bookmarks. If
  one of them does not go through the VPN while it is connected (its IP changed and
  FortiClient has not added the route yet), it asks you to reconnect the VPN, and tidies
  the browsers up automatically once you have.
- No administrator rights, PowerShell or .NET needed. The Windows installer is per user.
- The window shows the status, the browser connections, the watched hosts, settings and an
  activity log; closing it leaves Re-Link in the tray.

The manual equivalent: **Shift+Esc** in Chrome/Edge → *Network Service* → *End process*.

**Firefox (Windows):** its connections are watched and stuck ones are reported, but
Firefox keeps its sockets in its main process and they cannot be closed without
administrator rights. The fix is to restart the browser. On Linux the root helper closes
sockets one by one.

## Download and updates

From the [latest release](https://github.com/denizguzel/relink/releases/latest):

| File | For |
|---|---|
| `relink-<version>-windows-x64-setup.exe` | Windows installer. Installs for your user only (`%LOCALAPPDATA%\Programs\Re-Link`) and **never asks for administrator rights**. |
| `relink-<version>-windows-x64-portable.zip` | Windows without installing. Unzip anywhere; settings stay beside the program. |
| `relink-<version>-linux-x86_64.tar.gz` | Linux (Ubuntu 20.04 or newer). Contains the program, the root helper and `install-linux.sh`. |

Every release has a `SHA256SUMS` file and build provenance attestations. Re-Link tells you
on the Overview page when a newer release exists; **Update and restart** downloads it,
verifies its checksum and replaces itself. You can turn the check off in Settings →
*Check for updates*; it only asks GitHub for the latest version. How releases are made:
[RELEASING.md](RELEASING.md).

Windows SmartScreen may warn about unsigned downloads the first time you run them. The
in-app update is not affected, because Re-Link downloads it itself.

## Using it

1. Run the installer (or unzip the portable zip and start `relink.exe`). An icon appears
   in the tray.
2. Click the icon: **Fix now**, **Check now**, pause notifications, quit. The window has
   the settings (*Fix automatically*, *Start with Windows*, language, check interval,
   notification level, watched hosts and domains, the settings file and the log).

| Icon | Meaning |
|---|---|
| Green | Everything is fine |
| Grey | The VPN is not connected |
| Orange | A connection is stuck and can be fixed |
| Red | A watched host does not go through the VPN: reconnect the VPN |

Command line: `relink.exe --check [-v]` (read-only diagnosis), `relink.exe --dump` (a bug
report: network state plus diagnosis), `relink.exe --fix`, `relink.exe --version`.

**Known side effect:** while the Network Service restarts, downloads in progress and
WebSocket connections drop. Some sites may lose their session cookie and ask you to sign in
again.

## Settings (`relink.json`)

It lives in `%APPDATA%\Re-Link` (Linux: `~/.config/relink`), together with
`relink-state.json` and `relink.log`. For portable use, put `relink.json` next to the
program; everything is then kept there (the portable zip already does this). It is created
on first run; edits made in an editor apply without a restart.

```json
{
  "language": "auto",
  "autoFix": false,
  "notifications": "problems",
  "intervalSeconds": 15,
  "checkUpdates": true,
  "browsers": ["chrome.exe", "msedge.exe", "brave.exe", "chromium.exe", "firefox.exe"],
  "hosts": ["intranet.example.com"],
  "useBookmarks": true,
  "bookmarkDomains": ["etycloudbss.com", "etiyacloud.com", "etiya.com"],
  "vpnAdapterKeywords": ["Fortinet SSL VPN", "FortiClient"]
}
```

`bookmarkDomains` decides which bookmarked hosts count as company hosts; change it to
match your own VPN's domains.

## Linux

The same app runs on Linux (window, tray, notifications). On Linux there is also an
optional **root helper**; once installed:

- Re-Link **adds the missing VPN route itself** for a host whose IP changed
  (`ip route replace <ip>/32 dev <vpn> proto 222`). Turning the VPN off and on (and
  FortiGate's enforced wait) is no longer needed.
- Stuck connections are **closed one by one** (`ss -K`) without restarting the browser;
  this works for Firefox too.

```sh
tar xzf relink-<version>-linux-x86_64.tar.gz && cd relink-<version>-linux-x86_64
sudo ./install-linux.sh        # puts the helper in /usr/local/bin and adds /etc/sudoers.d/relink
./relink                       # the app
./relink --dump > dump.txt     # for a bug report: network state + diagnosis (read-only)
```

Check the download with `sha256sum -c --ignore-missing SHA256SUMS`.

The helper only accepts the commands `probe`, `route-add`, `route-del` and `kill-socket`;
it finds the VPN interface itself and can only delete routes it added. To remove it:
`sudo rm /usr/local/bin/relink-helper /etc/sudoers.d/relink`. The in-app updater never
replaces the helper (it is root-owned on purpose); when a release changes it, the release
notes say so and you rerun `install-linux.sh` from the new archive.

## Trying it

The simplest way to try the fix: with Re-Link running, turn the VPN off and on again.

To try the red state (VPN route missing), add a host that does not go through the VPN
(for example `"www.google.com"`) to the `hosts` list and save; remove it afterwards.

## Development

Rust + [Rune](https://github.com/anomalyco/rune) (the UI toolkit Hex uses). In WSL:

```sh
make test      # unit tests (run on Linux)
make lint      # cargo fmt --check + clippy, what CI runs
make snaps     # render every screen headless to PNG (target/snaps)
make build     # the Windows exe with cargo xwin
make linux     # the Linux files (glibc 2.31, cargo-zigbuild): dist/release/
make package   # every release file that can be built here: dist/release/
make check     # copy the exe to dist/relink-windows and run --check -v
```

Needs: `rustup target add x86_64-pc-windows-msvc`, `cargo install cargo-xwin`,
`cargo install cargo-zigbuild`, `sudo apt install clang lld llvm`. Texts are in
`locales/app/{en,tr}.json`. See [AGENTS.md](AGENTS.md) for the architecture and
[RELEASING.md](RELEASING.md) for releases.

Icons are [Lucide](https://lucide.dev) (ISC licence); see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

---
name: relink-diagnose
description: Investigate a report like "Re-Link says healthy but a page still gives 403", "false alarm", "it did not detect the stuck connection", or "Firefox/Chrome still broken". Contains what was learned about how the bug behaves.
---

# Diagnose a Re-Link report

## Gather evidence first (read-only)

1. Log: `tail -n 60 "$WINHOME/AppData/Roaming/Re-Link/relink.log"` (derive
   `WINHOME` as in the `relink-deploy` skill; never write a user name into the repo).
   Lines: `<unix> <HH:MM:SS> [info|good|warn] text` are events; untagged lines
   are diagnostics (stuck connection details, missing routes).
2. State now: `dist/relink-windows/relink.exe --check -v` (read-only). Shows
   browser connections with route and kind, watched hosts with kind, network
   service processes. Linux: `./relink --check -v`; `--dump` adds raw route tables.
3. Settings: `%APPDATA%\Re-Link\relink.json`. An old settings file may lack newer
   entries in `browsers`; the built-in browsers are always watched anyway.

Compare timestamps: when did the VPN go down / up, when did the stuck
connection appear, which process pid, did Firefox get restarted (pid changes).

## What is known about the bug

- A TCP connection's source address is fixed when it is opened. Opened while the
  VPN is off, or before FortiClient installed its `/32` route, it leaves over
  the home network; the ALB answers 403 and it stays that way.
- Chromium (Chrome/Edge/Brave) has a separate network service process: killing
  it drops those sockets and tabs survive. That is what "Fix now" does.
- **Firefox keeps sockets in its main process** on Windows (no restartable
  socket process). Without administrator rights Re-Link cannot close one
  socket; the connection clears by itself after a few idle minutes or on a
  Firefox restart. The user decided: report honestly, no admin helper.
- On Linux the root helper (`relink-helper`) can close single sockets
  (`ss -K`) and add `/32` routes (`proto 222`), so nothing needs restarting.
- FortiClient installs routes over a while after connecting. Re-Link waits
  (`monitor/settle.rs`) while the missing set keeps shrinking; during that wait
  missing routes and stuck connections are not alarms.
- A public address inside a broad VPN range (e.g. a CDN in a `/17`) is **not**
  stuck: it works over the home network too. Stuck needs a private address, a
  host route (`/32`), or a watched host's address (`diagnose::vpn_only`).
- Windows' DNS cache can keep VPN-off answers; flushing it needs admin.
- Hosts behind one ALB address share its IPs, so one stuck connection can break
  several hosts (likely HTTP/2 connection reuse; not verified).

## Reproduce safely

The user reproduces the bug by turning the VPN off and on while Re-Link runs. Do not run `--fix` or press
"Fix now" against the user's real Chrome/Edge/Firefox without asking.

## Report

Say what the evidence shows, what is certain and what is a hypothesis, and what
output you need from the user (`relink --check -v` while the page is failing).

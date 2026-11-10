---
name: relink-deploy
description: Build Re-Link for Windows, replace the portable copy on the Windows Desktop and start it (`make desktop`). Use after any change the user wants to try on their real machine ("deploy", "run it", "put it on the desktop", or at the end of a change they will test).
---

# Deploy Re-Link to the user's Windows Desktop

The user tests every change by running the exe on their own machine. They have
interrupted this routine twice because commands ran silently, so **say what you
are about to do before each step and what happened after it**, and run the
steps as separate small calls.

Standing permission covers stopping and restarting `relink.exe` only. It does
not cover pressing "Fix now", running `relink --fix`, or touching the user's
real browser.

## Run it

```sh
make desktop
```

It builds, then `scripts/deploy-desktop.sh` closes a running `relink.exe`,
deletes `Desktop\Re-Link-portable`, copies the new exe and an empty
`relink.json` (portable mode: settings stay in that folder, the real
`%APPDATA%` settings are untouched), starts it and prints the new pid. Before it
runs, say in one line what is about to happen; afterwards report what it printed.
Because the folder is recreated, the welcome screen shows every time. The script
derives the Windows profile path itself; the repo must not contain anyone's name
or profile path.

## Rules

- Never use `powershell.exe` or `pwsh.exe` (often blocked on managed PCs; it also
  fails from WSL here).
- Settings and logs live in `%APPDATA%\Re-Link` (`$WINHOME/AppData/Roaming/Re-Link`),
  not beside the exe. A `relink.json` next to the exe switches to portable mode.
- The script deletes only `Desktop\Re-Link-portable`, which it created; it refuses
  any other path. Never delete other files of the user.
- Say plainly what could not be checked: you cannot see the Windows window.

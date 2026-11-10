---
name: relink-deploy
description: Build Re-Link for Windows, stop the running relink.exe, copy the new exe to the Windows Desktop and start it again. Use after any change the user wants to try on their real machine ("deploy", "run it", "put it on the desktop", or at the end of a change they will test).
---

# Deploy Re-Link to the user's Windows Desktop

The user tests every change by running the exe on their own machine. They have
interrupted this routine twice because commands ran silently, so **say what you
are about to do before each step and what happened after it**, and run the
steps as separate small calls.

Standing permission covers stopping and restarting `relink.exe` only. It does
not cover pressing "Fix now", running `relink --fix`, or touching the user's
real browser.

## Find the Windows folders (never hardcode a user name)

```sh
cd /mnt/c    # not a \\wsl$ path, so cmd.exe starts without warnings
WINHOME=$(wslpath "$(cmd.exe /c 'echo %USERPROFILE%' 2>/dev/null | tr -d '\r')")
DESKTOP="$WINHOME/Desktop"
```

The repo must not contain anyone's name or profile path; always derive it.

## Steps

1. **Build.** `make build`. It also copies `relink.exe`
   into `dist/relink-windows/`. Stop if it prints an error or a warning.
2. **Stop the running app.** `taskkill.exe /IM relink.exe /F`, then
   `tasklist.exe /FI "IMAGENAME eq relink.exe"` must say no tasks. Match the
   image name exactly.
3. **Copy.** `cp dist/relink-windows/relink.exe "$DESKTOP/relink.exe"` then
   `cmp` the two files; both must be identical.
4. **Start.**
   `(cmd.exe /c start "" "$(wslpath -w "$DESKTOP/relink.exe")" >/dev/null 2>&1 &); sleep 3; tasklist.exe /FI "IMAGENAME eq relink.exe"`
   and report the new pid.

## Rules

- Never use `powershell.exe` or `pwsh.exe` (often blocked on managed PCs; it also
  fails from WSL here).
- Settings and logs live in `%APPDATA%\Re-Link` (`$WINHOME/AppData/Roaming/Re-Link`),
  not beside the exe. A `relink.json` next to the exe switches to portable mode.
- If the user must delete old files first (an old `relink.json` on the Desktop),
  tell them; do not delete their files.
- Say plainly what could not be checked: you cannot see the Windows window.

#!/bin/sh
# Puts the freshly built Windows exe on the Windows Desktop as a portable copy
# and starts it (WSL only). Run it through `make desktop`, which builds first.
#
# It closes a running relink.exe, deletes the old Desktop\Re-Link-portable
# folder (so the settings start fresh and the welcome screen shows again),
# copies the new exe and an empty relink.json (which makes it portable: all
# settings stay in that folder, your real %APPDATA% settings are untouched) and
# starts it.
set -eu
repo=$(cd "$(dirname "$0")/.." && pwd)
exe="$repo/dist/relink-windows/relink.exe"
[ -f "$exe" ] || { echo "no $exe: run make build first" >&2; exit 1; }

cd /mnt/c   # a Windows path, so cmd.exe starts without warnings
winhome=$(wslpath "$(cmd.exe /c 'echo %USERPROFILE%' 2>/dev/null | tr -d '\r')")
dir="$winhome/Desktop/Re-Link-portable"
case "$dir" in */Desktop/Re-Link-portable) ;; *) echo "unexpected folder: $dir" >&2; exit 1 ;; esac

echo "1/4 closing a running Re-Link"
if taskkill.exe /IM relink.exe /F >/dev/null 2>&1; then echo "    closed"; else echo "    none was running"; fi

echo "2/4 deleting the old Desktop\\Re-Link-portable"
# Windows releases the exe a moment after the process is gone: retry briefly.
for try in 1 2 3 4 5 6 7 8 9 10; do
  rm -rf "$dir" 2>/dev/null && break
  [ "$try" = 1 ] && echo "    the old exe is still locked, retrying"
  [ "$try" = 10 ] && { echo "    could not delete $dir" >&2; exit 1; }
  sleep 0.5
done
mkdir -p "$dir"

echo "3/4 copying the new build ($(du -h "$exe" | cut -f1))"
cp "$exe" "$dir/relink.exe"
printf '{}\n' > "$dir/relink.json"
cmp -s "$exe" "$dir/relink.exe" || { echo "copy differs from the build" >&2; exit 1; }

echo "4/4 starting it"
cmd.exe /c start "" "$(wslpath -w "$dir/relink.exe")" >/dev/null 2>&1 &
for _ in 1 2 3 4 5 6 7 8 9 10; do
  pid=$(tasklist.exe /FI "IMAGENAME eq relink.exe" /FO CSV /NH 2>/dev/null | tr -d '\r' | awk -F'"' '/relink.exe/ {print $4; exit}')
  [ -n "${pid:-}" ] && { echo "    running (pid $pid)"; exit 0; }
  sleep 0.5
done
echo "    did not start within 5 s" >&2
exit 1

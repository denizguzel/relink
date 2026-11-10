#!/bin/sh
# Installs Re-Link's root helper for the current machine (Ubuntu/Linux).
#
#   sudo ./install-linux.sh [path/to/relink-helper]   (default: next to this script)
#
# Copies relink-helper to /usr/local/bin (root-owned, so users can't change
# it) and lets members of the sudo group run exactly that file without a
# password. Undo with: sudo rm /usr/local/bin/relink-helper /etc/sudoers.d/relink
set -eu
src="${1:-$(dirname "$0")/relink-helper}"
[ "$(id -u)" = 0 ] || { echo "run with sudo" >&2; exit 1; }
[ -f "$src" ] || { echo "relink-helper not found at $src" >&2; exit 1; }
install -o root -g root -m 0755 "$src" /usr/local/bin/relink-helper
tmp=$(mktemp)
printf '%%sudo ALL=(root) NOPASSWD: /usr/local/bin/relink-helper\n' > "$tmp"
visudo -cf "$tmp" >/dev/null
install -o root -g root -m 0440 "$tmp" /etc/sudoers.d/relink
rm -f "$tmp"
echo "installed: /usr/local/bin/relink-helper and /etc/sudoers.d/relink"
sudo -u "${SUDO_USER:-root}" sudo -n /usr/local/bin/relink-helper probe

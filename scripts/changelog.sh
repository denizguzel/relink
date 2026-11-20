#!/bin/sh
# Prints a version's section of CHANGELOG.md (without its heading): the notes
# of that release. A pre-release (0.7.0-rc.1) without a section of its own uses
# the section of the version it leads to (0.7.0). Fails when there is none, so
# the release script and workflow stop before a release goes out without notes.
#
#   scripts/changelog.sh 0.6.0
#   scripts/changelog.sh v0.6.0
set -eu
v="${1:?usage: scripts/changelog.sh X.Y.Z[-rc.N]}"
v="${v#v}"
file="$(dirname "$0")/../CHANGELOG.md"

section() {
  awk -v v="$1" '
    /^## / { if (found) exit; split($0, h, " "); if (h[2] == v) { found = 1; next } }
    found { print }
  ' "$file" | sed -e '/./,$!d'
}

blank() { [ -z "$(printf '%s' "$1" | tr -d '[:space:]')" ]; }

notes=$(section "$v")
case "$v" in
  *-*) if blank "$notes"; then notes=$(section "${v%%-*}"); fi ;;
esac
if blank "$notes"; then
  echo "CHANGELOG.md has no section for $v (add \"## $v — <date>\")" >&2
  exit 1
fi
# The file is wrapped at 80 columns; release notes on GitHub keep single line
# breaks, so each paragraph and list item is joined into one line.
printf '%s\n' "$notes" | awk '
  function flush() { if (buf != "") print buf; buf = "" }
  /^[ \t]*$/ { flush(); print; next }
  /^#/ { flush(); print; next }
  /^(- |\* |[0-9]+\. )/ { flush(); buf = $0; next }
  { sub(/^[ \t]+/, ""); buf = (buf == "" ? $0 : buf " " $0) }
  END { flush() }
'

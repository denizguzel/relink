#!/bin/sh
# Prepares a release: checks, bumps the version, commits and tags. It never
# pushes; the last line printed is the command that starts the release.
#
#   scripts/release.sh 0.3.0        a release
#   scripts/release.sh 0.3.0-rc.1   a pre-release
set -eu
v="${1:?usage: scripts/release.sh X.Y.Z[-rc.N]}"
v="${v#v}"
case "$v" in
  [0-9]*.[0-9]*.[0-9]*) ;;
  *) echo "not a version: $v" >&2; exit 1 ;;
esac
cd "$(dirname "$0")/.."
branch=$(git rev-parse --abbrev-ref HEAD)
case "$branch" in master|main) ;; *) echo "release from master/main, not $branch" >&2; exit 1 ;; esac
[ -z "$(git status --porcelain)" ] || { echo "commit or stash your changes first" >&2; exit 1; }
git rev-parse "v$v" >/dev/null 2>&1 && { echo "tag v$v already exists" >&2; exit 1; }

echo "== checks"
make lint
make test

echo "== version $v"
sed -i "0,/^version = \".*\"/s//version = \"$v\"/" Cargo.toml
cargo update --workspace --offline >/dev/null 2>&1 || cargo update --workspace >/dev/null
git add Cargo.toml Cargo.lock
git commit -m "Release v$v"
git tag -a "v$v" -m "Re-Link $v"

echo
echo "Ready. Review with: git show v$v"
echo "Start the release:  git push origin $branch v$v"

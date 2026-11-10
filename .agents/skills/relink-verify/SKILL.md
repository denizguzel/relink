---
name: relink-verify
description: The checklist to run before calling a Re-Link change done: format, tests, snapshots of the affected screens, and both the Windows and Linux builds. Use at the end of any code change.
---

# Verify a Re-Link change

Run in this order and report real numbers. Do not say "done" until all pass.

1. **Format.** `cargo fmt --check` (or `make fmt`). The code uses default rustfmt.
2. **Tests.** `make test`. Report the count ("118 passed"), and any warning is
   a failure: the build must be warning-free. Add tests for new logic: put the
   decision in a small pure function (clock and state passed in) next to the
   code and test it there; UI states are tested through demo scenes
   (`src/ui/tests.rs`).
3. **Snapshots.** `make snaps`, then open the PNG of every screen you touched in
   `target/snaps/` and look at it. A new screen state needs a scene first (see
   the `add-demo-scene` skill). Snapshots run on Linux with lavapipe; scroll
   bars and Windows-only behaviour do not show.
4. **Windows build.** `make build` (cross-compiles with `cargo xwin`). Windows-
   only code (`cfg(windows)`) is only compiled here, so this step catches what
   `make test` cannot.
5. **Linux build.** `make linux` when anything user-facing changed; it
   produces `dist/release/relink-<v>-linux-x86_64{,.tar.gz}` and `SHA256SUMS`.
   Check glibc stays ≤ 2.31 (Ubuntu 20.04+):
   `objdump -T dist/release/relink-<v>-linux-x86_64 | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1`
   and `(cd dist/release && sha256sum -c SHA256SUMS)` says OK. Never send a
   file through file.pizza (it truncated one once); tell the user the expected
   byte size, or better, point to the GitHub Release.

## Report honestly

State what you verified and what you could not: the Windows window, tray menu,
toasts, window-position memory and scroll bars cannot be seen from WSL. Ask the
user to check those and say exactly what to look for.

Never run `--fix`, the tray, or anything that kills the real browser's network
service without asking. Ask the user to reproduce it by toggling the VPN.

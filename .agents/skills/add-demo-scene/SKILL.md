---
name: add-demo-scene
description: Add a sample window state (a "scene") to render headless snapshots and to test the UI. Use when a new screen state needs a screenshot or a test.
---

# Add a demo scene

Scenes in `src/ui/demo.rs` (`SCENES`) build a `Ui` state without the monitor, so
the window can be rendered headless (`make snaps`) and tested.

1. Add `("name", |u| { tr(); publish(u, stuck, missing, vpn_up); … })` to `SCENES`.
   `publish` fills a believable report (addresses mirror a real FortiClient
   split tunnel) and sets `cfg.welcomed = true`; change fields after it
   (`u.page = Page::Hosts`, `u.paused_until = Some(u.now + 600)`,
   `p.report.settling = true`, …). Only the `welcome` scene turns `welcomed` off.
2. Add the name to the `for s in …` list in the `snaps` target of the `Makefile`.
3. `make snaps`, then open `target/snaps/<name>.png` and look at it. Fix layout
   problems before anything else; check narrow text wrapping and long host names.
4. Test it: `src/ui/tests.rs` builds scenes with `scene("name")`; assert the
   status, the text (`diagnostics`, `status_text`) or the page logic. Page logic
   itself belongs in a pure function with a unit test (see `connections/group.rs`,
   `hosts/list.rs`, `activity/days.rs`).
5. `only_the_welcome_scene_shows_the_welcome_screen` iterates all scenes, so a
   scene that forgets `welcomed = true` fails that test.

Snapshots run on Linux with lavapipe; Windows-only behaviour (scroll bars,
toasts, tray, window position) does not appear in them.

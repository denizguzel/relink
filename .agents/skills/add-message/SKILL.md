---
name: add-message
description: Add or change a user-visible text (button, title, notification, status line) in Re-Link. Use whenever a new string appears in the UI or a toast.
---

# Add a message

All text lives in `locales/app/en.json` and `locales/app/tr.json` and is read
with `t!("key", name = value)` from `crate::t`. Keys are checked at compile time.

1. Add the key to **both** files. A missing one fails the test
   `every_message_exists_in_both_languages`.
2. Placeholders are `{{name}}` and must be the same in both languages
   (`placeholders_match_between_languages`). Pass them by name:
   `t!("paused_banner", minutes = n)`.
3. Turkish uses the formal "siz" form, as the rest of the app does. Keep sentences short; say what the user can do.
4. Do not name only Chrome/Edge in text: Firefox is supported too. Say
   "browser" (in both languages) or list the browsers.
5. Toast text must fit about three lines. The title says the problem, the body
   says what to do. Windows toasts have no buttons (rune-notify limitation).
6. Unused keys are not an error but remove ones you replace.
7. Run `make test`; look at the screen with `make snaps` if the text is long
   (descriptions wrap in narrow columns).

Edit the JSON through a script or an editor that keeps `ensure_ascii=False`
and a trailing newline so Turkish characters stay readable.

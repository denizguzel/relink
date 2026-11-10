//! UI strings, from the JSON catalogs in `locales/` (one file per language,
//! generated into Rust by `build.rs` through `rune-i18n`). Keys are checked
//! at compile time by the [`t!`](crate::t) macro.

include!(concat!(env!("OUT_DIR"), "/i18n.rs"));

/// The app-wide translator.
pub static I18N: rune_i18n::I18n = rune_i18n::I18n::new(&CATALOG);

/// A message by key, with optional `{{name}}` arguments:
/// `t!("status_ok")`, `t!("status_stuck", count = 2)`.
#[macro_export]
macro_rules! t {
    ($key:literal) => {{
        const KEY: rune_i18n::Key = rune_i18n::Key::named($crate::i18n::KEYS, $key);
        $crate::i18n::I18N.text(KEY).to_owned()
    }};
    ($key:literal, $($name:ident = $value:expr),+ $(,)?) => {{
        const KEY: rune_i18n::Key = rune_i18n::Key::named($crate::i18n::KEYS, $key);
        $crate::i18n::I18N.format(KEY, &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),+])
    }};
}

/// Applies the "language" setting: "tr", "en", or "auto" (the system's
/// preferred languages, else English).
pub fn apply(setting: &str) {
    let locale = match I18N.find_locale(setting) {
        Some(i) => i,
        None => {
            let langs = rune_i18n::system_languages();
            I18N.negotiate(langs.iter().map(String::as_str))
        }
    };
    I18N.set_locale(locale);
}

/// The current locale id ("en" or "tr").
pub fn current() -> &'static str {
    I18N.info().id()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_translated_with_the_same_placeholders() {
        let tr = I18N.find_locale("tr").expect("tr locale");
        let holes = |s: &str| {
            let mut v: Vec<String> = s
                .split("{{")
                .skip(1)
                .filter_map(|p| p.split("}}").next())
                .map(str::to_owned)
                .collect();
            v.sort();
            v
        };
        for (i, name) in KEYS.iter().enumerate() {
            let key = rune_i18n::Key::named(KEYS, name);
            assert_eq!(key.index(), i);
            let (en, tr) = (I18N.text_in(0, key), I18N.text_in(tr, key));
            assert!(!tr.is_empty() && tr != *name, "{name} has no Turkish text");
            assert_eq!(holes(en), holes(tr), "{name}: placeholders differ");
        }
    }

    #[test]
    fn formats_named_arguments() {
        let key = rune_i18n::Key::named(KEYS, "status_stuck");
        assert_eq!(
            rune_i18n::render(I18N.text_in(0, key), &[("count", &2)]),
            "Stuck connections: 2"
        );
    }

    fn keys(file: &str) -> std::collections::BTreeSet<String> {
        let path = format!("{}/locales/app/{file}", env!("CARGO_MANIFEST_DIR"));
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        value.as_object().unwrap().keys().cloned().collect()
    }

    #[test]
    fn every_message_exists_in_both_languages() {
        let (en, tr) = (keys("en.json"), keys("tr.json"));
        let only_en: Vec<_> = en.difference(&tr).collect();
        let only_tr: Vec<_> = tr.difference(&en).collect();
        assert!(
            only_en.is_empty() && only_tr.is_empty(),
            "only in en: {only_en:?}, only in tr: {only_tr:?}"
        );
    }
}

//! The Settings page: the everyday choices first, the technical ones inside a
//! closed "Advanced" group.

mod list_editor;
mod update;

use rune::{El, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, SegmentedControl, Switch};

use self::list_editor::list_editor;
use super::controls;
use crate::brand;
use crate::t;
use crate::ui::Ui;
use crate::ui::icons;
use crate::ui::parts::{self, page, panel, section, setting_row};

pub fn settings(s: &Ui, k: &Kit) -> El {
    let cfg = &s.cfg;
    let auto_fix = Switch::new(cfg.auto_fix)
        .label(t!("auto_fix"))
        .hide_label()
        .on_toggle(|s: &mut Ui| s.change(|c| c.auto_fix = !c.auto_fix))
        .build(k);
    let startup = controls::startup(s, k);
    let close_to_tray = Switch::new(cfg.close_to_tray)
        .label(t!("close_to_tray"))
        .hide_label()
        .on_toggle(|s: &mut Ui| s.change(|c| c.close_to_tray = !c.close_to_tray))
        .build(k);
    let start_hidden = Switch::new(cfg.start_hidden)
        .label(t!("start_hidden"))
        .hide_label()
        .on_toggle(|s: &mut Ui| s.change(|c| c.start_hidden = !c.start_hidden))
        .build(k);
    const INTERVALS: [u64; 5] = [5, 10, 15, 30, 60];
    let interval = SegmentedControl::new(
        &["5 s", "10 s", "15 s", "30 s", "60 s"],
        INTERVALS.iter().position(|&v| v == cfg.interval_seconds),
    )
    .label(t!("interval"))
    .fit()
    .on_pick(|s: &mut Ui, i: usize, _: &mut Tx| s.change(|c| c.interval_seconds = INTERVALS[i]))
    .build(k);
    let notifications = controls::notifications(s, k);
    let lang = controls::language(s, k);
    let bookmarks = Switch::new(cfg.use_bookmarks)
        .label(t!("use_bookmarks"))
        .hide_label()
        .on_toggle(|s: &mut Ui| s.change(|c| c.use_bookmarks = !c.use_bookmarks))
        .build(k);
    let updates = Switch::new(cfg.check_updates)
        .label(t!("check_updates"))
        .hide_label()
        .on_toggle(|s: &mut Ui| s.change(|c| c.check_updates = !c.check_updates))
        .build(k);
    let updates = row()
        .key("update-controls")
        .gap(12.0)
        .center_y()
        .child(update::check_button(s, k))
        .child(updates);

    let open_cfg = Button::new(t!("open_settings_file"))
        .icon(&icons::EXTERNAL_LINK)
        .variant(ButtonVariant::Neutral)
        .on_press(|s: &mut Ui| {
            if let Some(l) = &s.link {
                crate::platform::open_in_editor(&l.config_path);
            }
        })
        .build(k);
    let open_log = Button::new(t!("open_log_file"))
        .icon(&icons::FILE_TEXT)
        .variant(ButtonVariant::Ghost)
        .on_press(|s: &mut Ui| {
            if let Some(l) = &s.link {
                crate::platform::open_in_editor(&l.log_path);
            }
        })
        .build(k);
    let copied = s.copied_at != 0 && s.now <= s.copied_at + 3;
    let copy = Button::new(if copied {
        t!("diagnostics_copied")
    } else {
        t!("copy_diagnostics")
    })
    .icon(&icons::COPY)
    .variant(ButtonVariant::Ghost)
    .on_press_tx(|s: &mut Ui, tx: &mut Tx| {
        s.copied_at = s.now;
        tx.copy(crate::ui::diagnostics(s));
    })
    .build(k)
    .key("copy-diagnostics");
    let path = s
        .link
        .as_ref()
        .map(|l| l.config_path.display().to_string())
        .unwrap_or_else(|| "relink.json".into());

    let mut behavior = vec![
        setting_row(
            "auto-fix",
            k,
            &icons::ZAP,
            t!("auto_fix"),
            Some(t!("auto_fix_desc")),
            auto_fix,
        ),
        setting_row(
            "startup",
            k,
            &icons::POWER,
            t!("startup"),
            Some(t!("startup_desc")),
            startup,
        ),
        setting_row(
            "start-hidden",
            k,
            &icons::EYE_OFF,
            t!("start_hidden"),
            Some(t!("start_hidden_desc")),
            start_hidden,
        ),
        setting_row(
            "close-to-tray",
            k,
            &icons::PANEL_BOTTOM_CLOSE,
            t!("close_to_tray"),
            Some(t!("close_to_tray_desc")),
            close_to_tray,
        ),
        setting_row(
            "interval",
            k,
            &icons::TIMER,
            t!("interval"),
            Some(t!("interval_desc")),
            interval,
        ),
        setting_row(
            "notifications",
            k,
            &icons::BELL,
            t!("notifications"),
            Some(t!("notifications_desc")),
            notifications,
        ),
        setting_row(
            "updates",
            k,
            &icons::DOWNLOAD,
            t!("check_updates"),
            Some(t!("check_updates_desc")),
            updates,
        ),
    ];
    // A release the check found says so right under its own setting.
    behavior.extend(update::notice(s, k));

    let mut advanced = vec![
        section(k, t!("settings_sources")),
        panel(
            "sources",
            k,
            vec![setting_row(
                "bookmarks",
                k,
                &icons::BOOKMARK,
                t!("use_bookmarks"),
                Some(t!(
                    "use_bookmarks_desc",
                    domains = cfg.bookmark_domains.join(", ")
                )),
                bookmarks,
            )],
        ),
        section(k, t!("settings_domains")),
        list_editor(
            "domains",
            k,
            &cfg.bookmark_domains,
            &s.add_domain,
            t!("domain_placeholder"),
            |c| &mut c.bookmark_domains,
            |s| &mut s.add_domain,
        ),
        section(k, t!("settings_hosts")),
        list_editor(
            "hosts-list",
            k,
            &cfg.hosts,
            &s.add_host,
            t!("host_placeholder"),
            |c| &mut c.hosts,
            |s| &mut s.add_host,
        ),
    ];
    if cfg!(target_os = "linux") {
        let on = s.published.as_ref().is_some_and(|p| p.helper);
        let badge = parts::pill(
            if on { "✓".into() } else { "—".into() },
            parts::rgb(if on { brand::GREEN } else { brand::GRAY }),
            k,
        );
        advanced.push(panel(
            "helper",
            k,
            vec![setting_row(
                "helper",
                k,
                &icons::SHIELD_CHECK,
                t!("helper_title"),
                Some(if on {
                    t!("helper_on")
                } else {
                    t!("helper_off")
                }),
                badge,
            )],
        ));
    }
    advanced.push(panel(
        "advanced",
        k,
        vec![
            col()
                .key("advanced-copy")
                .px(16.0)
                .py(12.0)
                .gap(4.0)
                .child(
                    text(t!("advanced_desc"))
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .wrap()
                        .w_full(),
                )
                .child(
                    text(t!("settings_file_at", path = path))
                        .font(k.ty.small)
                        .color(k.c.text_faint)
                        .wrap()
                        .w_full(),
                ),
            row()
                .key("advanced-actions")
                .px(16.0)
                .py(12.0)
                .gap(8.0)
                .child(open_cfg)
                .child(open_log)
                .child(copy),
        ],
    ));

    let content = vec![
        section(k, t!("settings_behavior")),
        panel("behavior", k, behavior),
        section(k, t!("settings_look")),
        panel(
            "language",
            k,
            vec![
                setting_row(
                    "lang",
                    k,
                    &icons::LANGUAGES,
                    t!("settings_language"),
                    None,
                    lang,
                ),
                setting_row(
                    "theme",
                    k,
                    &icons::SUN_MOON,
                    t!("theme"),
                    Some(t!("theme_desc")),
                    controls::theme(s, k),
                ),
                setting_row(
                    "zoom",
                    k,
                    &icons::ZOOM_IN,
                    t!("zoom"),
                    Some(t!("zoom_desc")),
                    controls::zoom(s, k),
                ),
            ],
        ),
        parts::disclosure(
            "advanced-group",
            k,
            t!("settings_advanced"),
            s.advanced_open,
            |s: &mut Ui| s.advanced_open = !s.advanced_open,
            advanced,
        ),
    ];
    page("settings", k, t!("nav_settings"), None, content)
}

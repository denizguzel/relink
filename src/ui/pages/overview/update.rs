//! The banner for a newer release: one click installs it.

use rune::{El, row, text};
use rune_kit::{Button, ButtonVariant};

use super::Ctx;
use crate::t;
use crate::ui::icons;
use crate::ui::parts::leading_icon;
use crate::ui::{Cmd, Ui};
use crate::update::Stage;

/// What the banner says and offers for the current stage.
enum Offer {
    /// "Update and restart": the release has a file for this platform.
    Install,
    /// The release page, when there is no file for this platform or the
    /// in-app update failed.
    Page,
    /// Busy installing: no button.
    Busy,
}

fn offer(cx: &Ctx) -> Option<(String, Offer)> {
    let found = cx.s.update.as_ref()?;
    let banner = t!(
        "update_banner",
        version = found.version,
        current = cx.s.version
    );
    Some(match &cx.s.update_stage {
        Stage::Installing => (t!("update_installing"), Offer::Busy),
        Stage::Failed(error) => (t!("update_failed", error = error), Offer::Page),
        Stage::Ready if found.download.is_some() => (banner, Offer::Install),
        Stage::Ready => (banner, Offer::Page),
    })
}

/// "A new version is available: 0.3.0 (you have 0.2.0)" with the button that
/// installs it, once the update check found one.
pub(super) fn banner(cx: &Ctx) -> Option<El> {
    let (message, offer) = offer(cx)?;
    let (k, c) = (cx.k, &cx.k.c);
    let url = cx.s.update.as_ref()?.url.clone();
    let button = match offer {
        Offer::Install => Some(
            Button::new(t!("update_install"))
                .icon(&icons::DOWNLOAD)
                .variant(ButtonVariant::Neutral)
                .on_press(|s: &mut Ui| s.send(Cmd::InstallUpdate))
                .build(k)
                .key("update-install"),
        ),
        Offer::Page => Some(
            Button::new(t!("update_download"))
                .icon(&icons::EXTERNAL_LINK)
                .variant(ButtonVariant::Neutral)
                .on_press(move |_: &mut Ui| crate::platform::open_url(&url))
                .build(k)
                .key("update-download"),
        ),
        Offer::Busy => None,
    };
    let mut line = row()
        .key("update-banner")
        .w_full()
        .px(16.0)
        .py(10.0)
        .gap(12.0)
        .center_y()
        .radius(10.0)
        .fill(c.layer_01)
        .border(1.0, c.border_muted)
        .child(leading_icon(&icons::DOWNLOAD, k))
        .child(
            text(message)
                .font(k.ty.ui)
                .color(c.text_muted)
                .grow(1.0)
                .shrink()
                .wrap(),
        );
    if let Some(button) = button {
        line = line.child(button);
    }
    Some(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::demo::SCENES;
    use crate::update::{Available, Download};

    fn scene(name: &str) -> Ui {
        let mut ui = Ui::default();
        (SCENES.iter().find(|(n, _)| *n == name).unwrap().1)(&mut ui);
        ui
    }

    fn kind(ui: &Ui) -> Option<&'static str> {
        let kit = rune_theme_opencode::kit(true);
        offer(&Ctx::new(ui, kit)).map(|(_, o)| match o {
            Offer::Install => "install",
            Offer::Page => "page",
            Offer::Busy => "busy",
        })
    }

    fn found(with_file: bool) -> Available {
        Available {
            version: "0.3.0".into(),
            url: "https://github.com/denizguzel/relink/releases/tag/v0.3.0".into(),
            download: with_file.then(|| Download {
                name: "f".into(),
                asset_url: "a".into(),
                sums_url: "s".into(),
            }),
        }
    }

    #[test]
    fn nothing_shows_until_a_newer_release_was_found() {
        assert_eq!(kind(&scene("ok")), None);
    }

    #[test]
    fn a_release_with_a_file_installs_in_the_app() {
        assert_eq!(kind(&scene("update")), Some("install"));
    }

    #[test]
    fn without_a_file_or_after_a_failure_the_page_is_offered() {
        let mut ui = scene("update");
        ui.update = Some(found(false));
        assert_eq!(kind(&ui), Some("page"));
        ui.update = Some(found(true));
        ui.update_stage = Stage::Failed("no space".into());
        assert_eq!(kind(&ui), Some("page"));
    }

    #[test]
    fn while_installing_there_is_no_button() {
        let mut ui = scene("update");
        ui.update_stage = Stage::Installing;
        assert_eq!(kind(&ui), Some("busy"));
    }
}

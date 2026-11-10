//! The row that says a newer release was found, with the button that installs
//! it. It sits in Settings, under "Check for updates".

use rune::{El, el};
use rune_kit::{Button, ButtonVariant, Kit};

use crate::t;
use crate::ui::icons;
use crate::ui::parts::setting_row;
use crate::ui::{Cmd, Ui};
use crate::update::Stage;

/// What the row offers for the current stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Offer {
    /// "Update and restart": the release has a file for this platform.
    Install,
    /// The release page, when there is no file for this platform or the
    /// in-app update failed.
    Page,
    /// Busy installing: no button.
    Busy,
}

fn offer(s: &Ui) -> Option<(String, Offer)> {
    let found = s.update.as_ref()?;
    let banner = t!(
        "update_banner",
        version = found.version,
        current = s.version
    );
    Some(match &s.update_stage {
        Stage::Installing => (t!("update_installing"), Offer::Busy),
        Stage::Failed(error) => (t!("update_failed", error = error), Offer::Page),
        Stage::Ready if found.download.is_some() => (banner, Offer::Install),
        Stage::Ready => (banner, Offer::Page),
    })
}

/// "A new version is available: 0.3.0 (you have 0.2.0)" with the button that
/// installs it, once the update check found one.
pub(super) fn notice(s: &Ui, k: &Kit) -> Option<El> {
    let (message, offer) = offer(s)?;
    let url = s.update.as_ref()?.url.clone();
    let button = match offer {
        Offer::Install => Button::new(t!("update_install"))
            .icon(&icons::DOWNLOAD)
            .variant(ButtonVariant::Neutral)
            .on_press(|s: &mut Ui| s.send(Cmd::InstallUpdate))
            .build(k)
            .key("update-install"),
        Offer::Page => Button::new(t!("update_download"))
            .icon(&icons::EXTERNAL_LINK)
            .variant(ButtonVariant::Neutral)
            .on_press(move |_: &mut Ui| crate::platform::open_url(&url))
            .build(k)
            .key("update-download"),
        Offer::Busy => el(),
    };
    Some(setting_row(
        "update-available",
        k,
        &icons::DOWNLOAD,
        message,
        None,
        button,
    ))
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
        offer(ui).map(|(_, o)| match o {
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

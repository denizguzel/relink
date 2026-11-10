//! A small picture of the path a browser connection should take: browser,
//! VPN, company address. The link that is wrong takes the problem colour, so
//! the status reads at a glance.

use rune::{Color, El, col, el, row, text};

use super::Ctx;
use crate::diagnose::Status;
use crate::t;
use crate::ui::icons;
use crate::ui::parts::status_color;

/// How one part of the path is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tone {
    /// Working as it should.
    Good,
    /// The part the problem is about.
    Bad,
    /// Not in play (VPN down, nothing known yet).
    Off,
}

/// The VPN node and the two links (browser to VPN, VPN to company address).
fn tones(status: Option<Status>) -> [Tone; 3] {
    use Tone::*;
    match status {
        Some(Status::Ok) => [Good, Good, Good],
        Some(Status::Stuck) => [Good, Bad, Good],
        Some(Status::MissingRoute) => [Good, Good, Bad],
        Some(Status::Settling) => [Good, Good, Off],
        Some(Status::VpnDown) | None => [Off, Off, Off],
    }
}

fn node(key: &str, icon: &'static rune::Icon, label: String, tone: Tone, cx: &Ctx) -> El {
    let (k, c) = (cx.k, &cx.k.c);
    let color = tone_color(tone, cx);
    col()
        .key(key.to_string())
        .gap(6.0)
        .items_center()
        .child(
            el().key(format!("{key}-disc"))
                .size(40.0, 40.0)
                .radius(20.0)
                .fill(color.alpha(0.14))
                .border(1.0, color.alpha(0.5))
                .center()
                .child(rune::icon(icon, 20.0).color(color)),
        )
        .child(text(label).font(k.ty.small).color(c.text_muted))
}

fn tone_color(tone: Tone, cx: &Ctx) -> Color {
    match tone {
        Tone::Good => status_color(Some(Status::Ok)),
        Tone::Bad => status_color(cx.status),
        Tone::Off => cx.k.c.text_faint,
    }
}

fn link(key: &str, tone: Tone, cx: &Ctx) -> El {
    // Aligned with the middle of the discs, not of the labels under them.
    el().key(key.to_string())
        .w(34.0)
        .h(2.0)
        .mt(19.0)
        .radius(1.0)
        .fill(tone_color(tone, cx).alpha(if tone == Tone::Off { 0.5 } else { 1.0 }))
}

pub(super) fn flow(cx: &Ctx) -> El {
    let [vpn, to_vpn, to_company] = tones(cx.status);
    // The browser and the company address are fine unless a link says otherwise.
    let end = if cx.status == Some(Status::MissingRoute) {
        Tone::Bad
    } else {
        to_company
    };
    row()
        .key("flow")
        .gap(6.0)
        .items_start()
        .child(node(
            "flow-browser",
            &icons::GLOBE,
            t!("flow_browser"),
            Tone::Good,
            cx,
        ))
        .child(link("flow-link-1", to_vpn, cx))
        .child(node(
            "flow-vpn",
            &icons::SHIELD_CHECK,
            t!("flow_vpn"),
            vpn,
            cx,
        ))
        .child(link("flow-link-2", to_company, cx))
        .child(node(
            "flow-company",
            &icons::SERVER,
            t!("flow_company"),
            end,
            cx,
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_broken_link_is_the_one_that_is_drawn_as_a_problem() {
        assert_eq!(tones(Some(Status::Stuck))[1], Tone::Bad);
        assert_eq!(tones(Some(Status::MissingRoute))[2], Tone::Bad);
        assert!(tones(Some(Status::Ok)).iter().all(|t| *t == Tone::Good));
        assert!(tones(Some(Status::VpnDown)).iter().all(|t| *t == Tone::Off));
    }
}

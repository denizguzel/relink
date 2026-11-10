//! A panel that edits a list of names (company domains, extra hosts).

use rune::{El, Tx, el, row, text};
use rune_kit::{Button, ButtonVariant, Kit, TextInput};

use crate::t;
use crate::ui::Ui;
use crate::ui::icons;
use crate::ui::parts::panel;

/// A panel that edits a list of names: each entry with a remove button, then
/// an input with an add button. `list` picks the config field, `input` the
/// text field in the window's state.
pub(super) fn list_editor(
    key: &str,
    k: &Kit,
    items: &[String],
    input: &rune::TextEdit,
    placeholder: String,
    list: fn(&mut crate::config::Config) -> &mut Vec<String>,
    field: fn(&mut Ui) -> &mut rune::TextEdit,
) -> El {
    let c = &k.c;
    let mut rows: Vec<El> = items
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let remove = Button::new(t!("remove"))
                .icon(&icons::TRASH_2)
                .variant(ButtonVariant::Ghost)
                .on_press(move |s: &mut Ui| s.change(|cfg| remove_at(list(cfg), i)))
                .build(k)
                .key(format!("{key}-rm-{i}"));
            row()
                .key(format!("{key}-{i}"))
                .w_full()
                .px(16.0)
                .py(6.0)
                .gap(12.0)
                .center_y()
                .child(
                    text(name.clone())
                        .font(k.ty.mono)
                        .color(c.text_base)
                        .grow(1.0)
                        .shrink()
                        .wrap(),
                )
                .child(remove)
        })
        .collect();
    let add = move |s: &mut Ui| {
        if let Some(name) = crate::ui::clean_name(&field(s).text) {
            s.change(|cfg| {
                add_name(list(cfg), name);
            });
            *field(s) = rune::TextEdit::default();
        }
    };
    let input_el = TextInput::new(input)
        .placeholder(placeholder.clone())
        .label(placeholder)
        .on_edit(field)
        .on_submit(move |s: &mut Ui, _: &mut Tx| add(s))
        .build(k)
        .key(format!("{key}-input"));
    let add_btn = Button::new(t!("add"))
        .icon(&icons::PLUS)
        .variant(ButtonVariant::Neutral)
        .on_press(add)
        .build(k)
        .key(format!("{key}-add"));
    rows.push(
        row()
            .key(format!("{key}-new"))
            .w_full()
            .px(16.0)
            .py(10.0)
            .gap(8.0)
            .center_y()
            .child(el().grow(1.0).child(input_el))
            .child(add_btn),
    );
    panel(key, k, rows)
}

/// Adds `name` to a sorted list unless it is already there; whether it was added.
fn add_name(list: &mut Vec<String>, name: String) -> bool {
    if list.contains(&name) {
        return false;
    }
    list.push(name);
    list.sort();
    true
}

/// Removes the entry at `index`, if there is one.
fn remove_at(list: &mut Vec<String>, index: usize) {
    if index < list.len() {
        list.remove(index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn adding_keeps_the_list_sorted_and_unique() {
        let mut l = list(&["b.com", "d.com"]);
        assert!(add_name(&mut l, "c.com".into()));
        assert!(!add_name(&mut l, "b.com".into()));
        assert_eq!(l, list(&["b.com", "c.com", "d.com"]));
    }

    #[test]
    fn removing_ignores_an_index_that_is_gone() {
        let mut l = list(&["a", "b"]);
        remove_at(&mut l, 0);
        remove_at(&mut l, 5);
        assert_eq!(l, list(&["b"]));
    }
}

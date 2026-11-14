//! Unit converter dialog of calculation sheets: pick a kind of value, the
//! units and a value; the dialog shows the result and inserts the matching
//! line (`5 km to mi`) into the sheet.

use crate::calc::{self, LineResult};
use crate::units::{self, Category};
use adw::prelude::*;
use gtk::glib;
use std::cell::Cell;
use std::rc::Rc;

/// Cities offered for time zone conversions.
const ZONES: &[&str] = &[
    "local",
    "UTC",
    "London",
    "Lisbon",
    "Berlin",
    "Paris",
    "Madrid",
    "Rome",
    "Vienna",
    "Zurich",
    "Stockholm",
    "Warsaw",
    "Athens",
    "Helsinki",
    "Istanbul",
    "Moscow",
    "Cairo",
    "Johannesburg",
    "Lagos",
    "Dubai",
    "Kolkata",
    "Bangkok",
    "Singapore",
    "Hong Kong",
    "Shanghai",
    "Seoul",
    "Tokyo",
    "Sydney",
    "Auckland",
    "Honolulu",
    "Anchorage",
    "Los Angeles",
    "Denver",
    "Chicago",
    "New York",
    "Toronto",
    "Mexico City",
    "Sao Paulo",
    "Buenos Aires",
    "Reykjavik",
];

#[derive(Clone, Copy)]
enum Kind {
    Units(Category, &'static [&'static str]),
    Currency,
    Zone,
    Calendar,
}

impl Kind {
    fn all() -> Vec<Kind> {
        let mut kinds: Vec<Kind> = units::DIALOG.iter().map(|(c, list)| Kind::Units(*c, list)).collect();
        kinds.insert(4, Kind::Currency);
        kinds.extend([Kind::Zone, Kind::Calendar]);
        kinds
    }

    fn label(self) -> &'static str {
        match self {
            Kind::Units(c, _) => c.label(),
            Kind::Currency => Category::Currency.label(),
            Kind::Zone => t!("Time Zone"),
            Kind::Calendar => t!("Calendar"),
        }
    }

    /// What goes into the line, and the labels shown.
    fn choices(self) -> Vec<(String, String)> {
        match self {
            Kind::Units(_, list) => list.iter().map(|e| ((*e).to_owned(), units::dialog_label(e))).collect(),
            Kind::Currency => units::CURRENCIES.iter().map(|(code, name, _)| ((*code).to_owned(), format!("{code} ({name})"))).collect(),
            Kind::Zone => ZONES
                .iter()
                .map(|z| ((*z).to_owned(), if *z == "local" { t!("Local time").to_owned() } else { (*z).to_owned() }))
                .collect(),
            Kind::Calendar => calc::DATE_TARGETS
                .iter()
                .map(|(word, info)| ((*word).to_owned(), format!("{} ({word})", crate::i18n::tr(info))))
                .collect(),
        }
    }

    fn default_value(self) -> &'static str {
        match self {
            Kind::Units(..) | Kind::Currency => "1",
            Kind::Zone => "now",
            Kind::Calendar => "today",
        }
    }

    /// Default "from" and "to" choices.
    fn defaults(self) -> (u32, u32) {
        match self {
            Kind::Units(Category::Length, _) => (3, 5),
            Kind::Units(Category::Mass, _) => (2, 5),
            Kind::Units(Category::Speed, _) => (1, 2),
            Kind::Units(Category::Volume, _) => (2, 11),
            Kind::Zone => (0, 26),
            _ => (0, 1),
        }
    }
}

thread_local! {
    static LAST_KIND: Cell<u32> = const { Cell::new(0) };
}

fn combo(title: &str) -> adw::ComboRow {
    let row = adw::ComboRow::builder().title(title).enable_search(true).build();
    row.set_expression(Some(gtk::PropertyExpression::new(
        gtk::StringObject::static_type(),
        None::<gtk::Expression>,
        "string",
    )));
    row
}

/// The calculation line for the chosen values.
fn line(kind: Kind, value: &str, from: &str, to: &str) -> String {
    let value = value.trim();
    match kind {
        Kind::Units(..) | Kind::Currency => format!("{value} {from} to {to}"),
        // A from-zone only applies to a written time or date.
        Kind::Zone if value.starts_with(|c: char| c.is_ascii_digit()) => format!("{value} {from} to {to}"),
        Kind::Zone | Kind::Calendar => format!("{value} to {to}"),
    }
}

/// Opens the dialog; `insert` gets the line to insert into the sheet.
pub fn open(parent: &impl IsA<gtk::Widget>, insert: impl Fn(&str) + 'static) {
    let kinds = Kind::all();
    let dialog = adw::AlertDialog::new(Some(t!("Unit Converter")), None);
    dialog.add_responses(&[("cancel", t!("_Cancel")), ("copy", t!("_Copy Result")), ("insert", t!("_Insert Line"))]);
    dialog.set_response_appearance("insert", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("insert"));
    dialog.set_close_response("cancel");

    let labels: Vec<&str> = kinds.iter().map(|k| k.label()).collect();
    let kind_row = adw::ComboRow::builder().title(t!("Convert")).model(&gtk::StringList::new(&labels)).build();
    let value_row = adw::EntryRow::builder().title(t!("Value")).build();
    let from_row = combo(t!("From"));
    let to_row = combo(t!("To"));
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).build();
    list.add_css_class("boxed-list");
    for row in [kind_row.upcast_ref::<gtk::Widget>(), value_row.upcast_ref(), from_row.upcast_ref(), to_row.upcast_ref()] {
        list.append(row);
    }

    let result = gtk::Label::builder().xalign(0.0).wrap(true).selectable(true).margin_top(12).build();
    result.add_css_class("title-3");
    let hint = gtk::Label::builder().xalign(0.0).wrap(true).selectable(true).build();
    hint.add_css_class("dim-label");
    hint.add_css_class("caption");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
    content.append(&list);
    content.append(&result);
    content.append(&hint);
    dialog.set_extra_child(Some(&content));

    let choices: Rc<std::cell::RefCell<Vec<(String, String)>>> = Rc::default();
    let current = Rc::new(Cell::new(kinds[0]));
    let composed: Rc<std::cell::RefCell<(String, Option<String>)>> = Rc::default();

    let refresh: Rc<dyn Fn()> = {
        let (choices, current, composed) = (choices.clone(), current.clone(), composed.clone());
        let (value_row, from_row, to_row, result, hint) = (value_row.clone(), from_row.clone(), to_row.clone(), result.clone(), hint.clone());
        let dialog = dialog.downgrade();
        Rc::new(move || {
            let kind = current.get();
            let c = choices.borrow();
            let pick = |row: &adw::ComboRow| c.get(row.selected() as usize).map(|(v, _)| v.clone()).unwrap_or_default();
            let text = line(kind, &value_row.text(), &pick(&from_row), &pick(&to_row));
            let outcome = calc::evaluate(&text).pop().unwrap_or(LineResult::Empty);
            let shown = outcome.display();
            match (&outcome, &shown) {
                (_, Some(s)) => result.set_text(&format!("= {s}")),
                (LineResult::Error(e), _) => result.set_text(&format!("⚠ {e}")),
                _ => result.set_text("–"),
            }
            if shown.is_some() {
                result.remove_css_class("error");
            } else {
                result.add_css_class("error");
            }
            let mut note = t!("Line: {}", text);
            if matches!(kind, Kind::Currency) {
                match crate::rates::date() {
                    Some(date) => note.push_str(&format!("\n{}", t!("ECB reference rates of {}", date))),
                    None => note.push_str(&format!("\n{}", t!("Exchange rates of the European Central Bank"))),
                }
            }
            hint.set_text(&note);
            if let Some(d) = dialog.upgrade() {
                d.set_response_enabled("copy", shown.is_some());
                d.set_response_enabled("insert", shown.is_some());
            }
            *composed.borrow_mut() = (text, shown);
        })
    };

    let set_kind: Rc<dyn Fn(u32)> = {
        let (choices, current, refresh) = (choices.clone(), current.clone(), refresh.clone());
        let (value_row, from_row, to_row) = (value_row.clone(), from_row.clone(), to_row.clone());
        let kinds = kinds.clone();
        Rc::new(move |index| {
            let Some(kind) = kinds.get(index as usize).copied() else { return };
            let previous = current.replace(kind);
            let list = kind.choices();
            let labels: Vec<&str> = list.iter().map(|(_, l)| l.as_str()).collect();
            *choices.borrow_mut() = list.clone();
            let model = gtk::StringList::new(&labels);
            from_row.set_model(Some(&model));
            to_row.set_model(Some(&model));
            let (from, to) = kind.defaults();
            let last = list.len().saturating_sub(1) as u32;
            from_row.set_selected(from.min(last));
            to_row.set_selected(to.min(last));
            from_row.set_visible(!matches!(kind, Kind::Calendar));
            let numeric = |k: Kind| matches!(k, Kind::Units(..) | Kind::Currency);
            if value_row.text().is_empty() || numeric(previous) != numeric(kind) || matches!(kind, Kind::Calendar) != matches!(previous, Kind::Calendar) {
                value_row.set_text(kind.default_value());
            }
            if matches!(kind, Kind::Currency) {
                let _ = crate::rates::rate("USD");
                let refresh = refresh.clone();
                crate::rates::refresh(move || refresh());
            }
            LAST_KIND.with(|k| k.set(index));
            refresh();
        })
    };

    {
        let set_kind = set_kind.clone();
        kind_row.connect_selected_notify(move |row| set_kind(row.selected()));
    }
    for row in [&from_row, &to_row] {
        let refresh = refresh.clone();
        row.connect_selected_notify(move |_| refresh());
    }
    {
        let refresh = refresh.clone();
        value_row.connect_changed(move |_| refresh());
    }
    let start = LAST_KIND.with(Cell::get).min(kinds.len() as u32 - 1);
    // set_selected does not notify when the index stays 0.
    kind_row.set_selected(start);
    set_kind(start);

    let composed_for_response = composed.clone();
    dialog.connect_response(None, move |d, response| {
        let (text, shown) = composed_for_response.borrow().clone();
        match response {
            "insert" => insert(&text),
            "copy" => {
                if let Some(s) = shown {
                    d.clipboard().set_text(&s);
                }
            }
            _ => {}
        }
    });
    dialog.present(Some(parent));
    glib::idle_add_local_once(move || {
        value_row.grab_focus();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every unit list entry converts (with a value of 1) to every other.
    #[test]
    fn dialog_lines_evaluate() {
        for kind in Kind::all() {
            let Kind::Units(_, list) = kind else { continue };
            for from in list {
                for to in list {
                    let text = line(kind, "1", from, to);
                    let r = calc::evaluate(&text).pop().unwrap();
                    assert!(matches!(r, LineResult::Shown(_)), "{text}: {r:?}");
                }
            }
        }
        let r = calc::evaluate(&line(Kind::Zone, "2026-01-15 09:30", "New York", "Tokyo")).pop().unwrap();
        assert!(matches!(r, LineResult::Shown(_)), "{r:?}");
        let r = calc::evaluate(&line(Kind::Calendar, "today", "", "hebrew")).pop().unwrap();
        assert!(matches!(r, LineResult::Shown(_)), "{r:?}");
    }

    #[test]
    fn zones_resolve() {
        for z in ZONES {
            assert!(crate::calendar::zone(z).is_some(), "{z}");
        }
    }
}

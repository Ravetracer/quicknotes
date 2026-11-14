//! GtkSourceView completion for presentations (backed by
//! [`crate::slides_assist`]) and calculation sheets ([`crate::calc_assist`]).

use crate::slides_assist::{self, Assist, Context, Item};
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use sourceview5::prelude::*;
use sourceview5::subclass::prelude::*;

glib::wrapper! {
    pub struct Proposal(ObjectSubclass<imp::Proposal>) @implements sourceview5::CompletionProposal;
}

glib::wrapper! {
    pub struct SlidesProvider(ObjectSubclass<imp::SlidesProvider>) @implements sourceview5::CompletionProvider;
}

glib::wrapper! {
    pub struct CalcProvider(ObjectSubclass<imp::CalcProvider>) @implements sourceview5::CompletionProvider;
}

impl Default for CalcProvider {
    fn default() -> Self {
        glib::Object::new()
    }
}

/// Unit and conversion proposals at `iter` in a calculation sheet: the
/// byte offset in the line where the word starts and the items.
fn calc_at(iter: &gtk::TextIter) -> Option<(usize, Vec<Item>)> {
    let buffer = iter.buffer();
    let is_calc = buffer
        .downcast_ref::<sourceview5::Buffer>()
        .and_then(|b| b.language())
        .is_some_and(|l| l.id() == "calc");
    if !is_calc {
        return None;
    }
    let mut line_start = *iter;
    line_start.set_line_offset(0);
    let before = buffer.text(&buffer.start_iter(), &line_start, false);
    let line = buffer.text(&line_start, iter, false);
    crate::calc_assist::proposals(&before, &line)
}

fn calc_proposals(context: &sourceview5::CompletionContext) -> Vec<Proposal> {
    cursor(context).and_then(|i| calc_at(&i)).map(|(_, items)| items.into_iter().map(Proposal::new).collect()).unwrap_or_default()
}

impl Proposal {
    fn new(item: Item) -> Self {
        let p: Self = glib::Object::new();
        *p.imp().item.borrow_mut() = Some(item);
        p
    }

    fn item(&self) -> Option<Item> {
        self.imp().item.borrow().clone()
    }
}

impl Default for SlidesProvider {
    fn default() -> Self {
        glib::Object::new()
    }
}

/// Analysis of the cursor position in a presentation buffer, with the text
/// of the cursor line; `None` for other languages.
pub fn assist_at(iter: &gtk::TextIter) -> Option<(Assist, String)> {
    let buffer = iter.buffer();
    let is_slides = buffer
        .downcast_ref::<sourceview5::Buffer>()
        .and_then(|b| b.language())
        .is_some_and(|l| l.id() == crate::slides::LANGUAGE);
    if !is_slides || !crate::features::on(crate::features::Feature::Presentations) {
        return None;
    }
    let mut line_start = *iter;
    line_start.set_line_offset(0);
    let mut line_end = line_start;
    if !line_end.ends_line() {
        line_end.forward_to_line_end();
    }
    let before = buffer.text(&line_start, iter, false).to_string();
    let line = buffer.text(&line_start, &line_end, false).to_string();
    Some((slides_assist::analyze(&before, in_front_matter(&buffer, iter.line())), line))
}

/// Whether `line` lies inside the front matter block at the top.
fn in_front_matter(buffer: &gtk::TextBuffer, line: i32) -> bool {
    let line_text = |n: i32| {
        let Some(start) = buffer.iter_at_line(n) else { return String::new() };
        let mut end = start;
        if !end.ends_line() {
            end.forward_to_line_end();
        }
        buffer.text(&start, &end, false).trim().to_owned()
    };
    line > 0 && line < 200 && line_text(0) == "---" && (1..line).all(|n| !matches!(line_text(n).as_str(), "---" | "..."))
}

/// Context and title of the cheat sheet at the cursor of a presentation.
pub fn cheat_sheet_at(buffer: &gtk::TextBuffer) -> (String, Vec<(String, String)>) {
    let iter = buffer.iter_at_mark(&buffer.get_insert());
    let context = assist_at(&iter).map_or(Context::Slide, |(a, _)| a.context);
    slides_assist::cheat_sheet(&context)
}

fn cursor(context: &sourceview5::CompletionContext) -> Option<gtk::TextIter> {
    let buffer = context.buffer()?;
    Some(buffer.iter_at_mark(&buffer.get_insert()))
}

fn proposals(context: &sourceview5::CompletionContext) -> Vec<Proposal> {
    let Some((assist, line)) = cursor(context).and_then(|i| assist_at(&i)) else { return Vec::new() };
    assist.proposals(&line).into_iter().map(Proposal::new).collect()
}

mod imp {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct Proposal {
        pub item: RefCell<Option<Item>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Proposal {
        const NAME: &'static str = "QnSlidesProposal";
        type Type = super::Proposal;
        type Interfaces = (sourceview5::CompletionProposal,);
    }

    impl ObjectImpl for Proposal {}
    impl CompletionProposalImpl for Proposal {}

    #[derive(Default)]
    pub struct SlidesProvider;

    #[glib::object_subclass]
    impl ObjectSubclass for SlidesProvider {
        const NAME: &'static str = "QnSlidesProvider";
        type Type = super::SlidesProvider;
        type Interfaces = (sourceview5::CompletionProvider,);
    }

    impl ObjectImpl for SlidesProvider {}

    impl CompletionProviderImpl for SlidesProvider {
        fn title(&self) -> Option<glib::GString> {
            Some(t!("Presentation").into())
        }

        fn is_trigger(&self, iter: &gtk::TextIter, c: char) -> bool {
            // Separators open the list right away (e.g. the space after `:::`).
            matches!(c, ' ' | ':' | '=' | ';' | '-' | '[')
                && assist_at(iter).is_some_and(|(a, line)| !a.proposals(&line).is_empty())
        }

        // Every vfunc is overridden: the bindings' defaults chain up to a
        // parent implementation that interfaces don't have and panic, which
        // aborts the app (e.g. `list_alternates` on cursor keys).

        fn priority(&self, _context: &sourceview5::CompletionContext) -> i32 {
            0
        }

        fn key_activates(
            &self,
            _context: &sourceview5::CompletionContext,
            _proposal: &sourceview5::CompletionProposal,
            _keyval: gtk::gdk::Key,
            _state: gtk::gdk::ModifierType,
        ) -> bool {
            false
        }

        fn list_alternates(
            &self,
            _context: &sourceview5::CompletionContext,
            _proposal: &sourceview5::CompletionProposal,
        ) -> Vec<sourceview5::CompletionProposal> {
            Vec::new()
        }

        fn populate(&self, context: &sourceview5::CompletionContext) -> Result<gio::ListModel, glib::Error> {
            let store = gio::ListStore::new::<super::Proposal>();
            store.extend_from_slice(&proposals(context));
            Ok(store.upcast())
        }

        fn populate_future(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<gio::ListModel, glib::Error>>>> {
            Box::pin(std::future::ready(self.populate(context)))
        }

        fn refilter(&self, context: &sourceview5::CompletionContext, model: &gio::ListModel) {
            if let Some(store) = model.downcast_ref::<gio::ListStore>() {
                store.splice(0, store.n_items(), &proposals(context));
            }
        }

        fn display(
            &self,
            _context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
            cell: &sourceview5::CompletionCell,
        ) {
            let Some(item) = proposal.downcast_ref::<super::Proposal>().and_then(super::Proposal::item) else {
                return;
            };
            match cell.column() {
                sourceview5::CompletionColumn::TypedText => cell.set_text(Some(&item.label)),
                sourceview5::CompletionColumn::Comment | sourceview5::CompletionColumn::Details => {
                    cell.set_text(Some(&item.info))
                }
                _ => cell.set_text(None),
            }
        }

        fn activate(&self, context: &sourceview5::CompletionContext, proposal: &sourceview5::CompletionProposal) {
            let Some(item) = proposal.downcast_ref::<super::Proposal>().and_then(super::Proposal::item) else {
                return;
            };
            let (Some(buffer), Some(mut end)) = (context.buffer(), cursor(context)) else { return };
            let Some((assist, _)) = assist_at(&end) else { return };
            let mut start = end;
            start.set_line_index(assist.start as i32);
            // Replace the rest of the word after the cursor as well.
            while !end.ends_line() && !matches!(end.char(), ' ' | '\t' | ';' | ']' | '=') {
                end.forward_char();
            }
            buffer.begin_user_action();
            buffer.delete(&mut start, &mut end);
            buffer.insert(&mut start, &item.insert);
            buffer.end_user_action();
        }
    }

    #[derive(Default)]
    pub struct CalcProvider;

    #[glib::object_subclass]
    impl ObjectSubclass for CalcProvider {
        const NAME: &'static str = "QnCalcProvider";
        type Type = super::CalcProvider;
        type Interfaces = (sourceview5::CompletionProvider,);
    }

    impl ObjectImpl for CalcProvider {}

    // Every vfunc is overridden, see `SlidesProvider`.
    impl CompletionProviderImpl for CalcProvider {
        fn title(&self) -> Option<glib::GString> {
            Some(t!("Units").into())
        }

        fn is_trigger(&self, iter: &gtk::TextIter, _c: char) -> bool {
            calc_at(iter).is_some()
        }

        fn priority(&self, _context: &sourceview5::CompletionContext) -> i32 {
            0
        }

        fn key_activates(
            &self,
            _context: &sourceview5::CompletionContext,
            _proposal: &sourceview5::CompletionProposal,
            _keyval: gtk::gdk::Key,
            _state: gtk::gdk::ModifierType,
        ) -> bool {
            false
        }

        fn list_alternates(
            &self,
            _context: &sourceview5::CompletionContext,
            _proposal: &sourceview5::CompletionProposal,
        ) -> Vec<sourceview5::CompletionProposal> {
            Vec::new()
        }

        fn populate(&self, context: &sourceview5::CompletionContext) -> Result<gio::ListModel, glib::Error> {
            let store = gio::ListStore::new::<super::Proposal>();
            store.extend_from_slice(&calc_proposals(context));
            Ok(store.upcast())
        }

        fn populate_future(
            &self,
            context: &sourceview5::CompletionContext,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<gio::ListModel, glib::Error>>>> {
            Box::pin(std::future::ready(self.populate(context)))
        }

        fn refilter(&self, context: &sourceview5::CompletionContext, model: &gio::ListModel) {
            if let Some(store) = model.downcast_ref::<gio::ListStore>() {
                store.splice(0, store.n_items(), &calc_proposals(context));
            }
        }

        fn display(
            &self,
            _context: &sourceview5::CompletionContext,
            proposal: &sourceview5::CompletionProposal,
            cell: &sourceview5::CompletionCell,
        ) {
            let Some(item) = proposal.downcast_ref::<super::Proposal>().and_then(super::Proposal::item) else {
                return;
            };
            match cell.column() {
                sourceview5::CompletionColumn::TypedText => cell.set_text(Some(&item.label)),
                sourceview5::CompletionColumn::Comment | sourceview5::CompletionColumn::Details => {
                    cell.set_text(Some(&item.info))
                }
                _ => cell.set_text(None),
            }
        }

        fn activate(&self, context: &sourceview5::CompletionContext, proposal: &sourceview5::CompletionProposal) {
            let Some(item) = proposal.downcast_ref::<super::Proposal>().and_then(super::Proposal::item) else {
                return;
            };
            let (Some(buffer), Some(mut end)) = (context.buffer(), cursor(context)) else { return };
            let Some((start_index, _)) = calc_at(&end) else { return };
            let mut start = end;
            start.set_line_index(start_index as i32);
            // Replace the rest of the word after the cursor as well.
            while !end.ends_line() && (end.char().is_alphanumeric() || end.char() == '_') {
                end.forward_char();
            }
            buffer.begin_user_action();
            buffer.delete(&mut start, &mut end);
            buffer.insert(&mut start, &item.insert);
            buffer.end_user_action();
        }
    }
}

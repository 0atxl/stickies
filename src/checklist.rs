//! Checklist prefixes exist in saved text, never as invisible buffer text.
//! An anchor owns completion and the formatting of its six serialized characters.
use crate::text_editing::Mark;
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

const KEY: &str = "stickies-checklist-prefix";

struct Prefix {
    checked: bool,
    formats: [u8; 6],
}

fn prefix(at: &gtk::TextIter) -> Option<Rc<RefCell<Prefix>>> {
    let anchor = at.child_anchor()?;
    // This private key is written once, below, with exactly this type. Clone the
    // Rc while the anchor is alive; no reference into its object data escapes.
    unsafe {
        anchor
            .data::<Rc<RefCell<Prefix>>>(KEY)
            .map(|data| data.as_ref().clone())
    }
}

pub fn checked(at: &gtk::TextIter) -> Option<bool> {
    prefix(at).map(|prefix| prefix.borrow().checked)
}

pub fn marker(at: &gtk::TextIter) -> Option<&'static str> {
    checked(at).map(|checked| if checked { "- [x] " } else { "- [ ] " })
}

pub fn formats(at: &gtk::TextIter) -> Option<[u8; 6]> {
    prefix(at).map(|prefix| prefix.borrow().formats)
}

pub fn set_formats(at: &gtk::TextIter, formats: [u8; 6]) {
    if let Some(prefix) = prefix(at) {
        prefix.borrow_mut().formats = formats;
    }
}

pub fn set_checked(buffer: &gtk::TextBuffer, at: &gtk::TextIter, checked: bool) {
    let Some(prefix) = prefix(at) else {
        return;
    };
    if prefix.borrow().checked != checked {
        prefix.borrow_mut().checked = checked;
        // Completion is a content edit even though no buffer characters change.
        buffer.emit_by_name::<()>("changed", &[]);
    }
}

pub fn text(buffer: &gtk::TextBuffer, start: &gtk::TextIter, end: &gtk::TextIter) -> String {
    let mut text = String::new();
    let mut at = *start;
    while at < *end {
        if let Some(marker) = marker(&at) {
            text.push_str(marker);
        } else if at.child_anchor().is_none() {
            text.push(at.char());
        }
        at.forward_char();
    }
    debug_assert_eq!(start.buffer(), *buffer);
    text
}

pub fn offset(buffer: &gtk::TextBuffer, at: &gtk::TextIter) -> i32 {
    text(buffer, &buffer.start_iter(), at).chars().count() as i32
}

pub fn iter(buffer: &gtk::TextBuffer, offset: i32) -> gtk::TextIter {
    let mut at = buffer.start_iter();
    let mut remaining = offset;
    while !at.is_end() && remaining > 0 {
        remaining -= if marker(&at).is_some() {
            6
        } else if at.child_anchor().is_some() {
            0
        } else {
            1
        };
        at.forward_char();
    }
    at
}

/// Materialize an anchor when its prefix becomes ordinary text, as in a join
/// or a bullet/numbering edit. Call inside the owning session's edit action.
pub fn expand(buffer: &gtk::TextBuffer, at: &mut gtk::TextIter) {
    let Some(marker) = marker(at) else {
        return;
    };
    let formats = formats(at).unwrap();
    let position = at.offset();
    let mut end = *at;
    end.forward_char();
    buffer.delete(at, &mut end);
    buffer.insert(at, marker);
    let mut start = buffer.iter_at_offset(position);
    for bits in formats {
        let mut end = start;
        end.forward_char();
        for mark in Mark::ALL {
            let tag = mark.tag(buffer);
            if bits & (1 << mark as u8) != 0 {
                buffer.apply_tag(&tag, &start, &end);
            } else {
                buffer.remove_tag(&tag, &start, &end);
            }
        }
        start = end;
    }
}

/// Convert typed/pasted prefixes, preserving formatting and GTK's live marks.
/// The session suppresses history while this presentation-only conversion runs.
pub fn normalize(buffer: &gtk::TextBuffer) {
    let mut at = buffer.start_iter();
    while !at.is_end() {
        if checked(&at).is_some() && !at.starts_line() {
            expand(buffer, &mut at);
        } else {
            at.forward_char();
        }
    }
    for line in 0..buffer.line_count() {
        let Some(mut start) = buffer.iter_at_line(line) else {
            continue;
        };
        if start.child_anchor().is_some() {
            continue;
        }
        let mut end = start;
        end.forward_to_line_end();
        let raw = buffer.text(&start, &end, true);
        let checked = if raw.starts_with("- [ ] ") {
            false
        } else if raw.starts_with("- [x] ") {
            true
        } else {
            continue;
        };
        let mut formats = [0; 6];
        let mut end = start;
        for bits in &mut formats {
            for mark in Mark::ALL {
                if end.has_tag(&mark.tag(buffer)) {
                    *bits |= 1 << mark as u8;
                }
            }
            end.forward_char();
        }
        buffer.delete(&mut start, &mut end);
        let anchor = gtk::TextChildAnchor::new();
        // See prefix(): this key is private, typed consistently, and never reset.
        unsafe {
            anchor.set_data(KEY, Rc::new(RefCell::new(Prefix { checked, formats })));
        }
        buffer.insert_child_anchor(&mut start, &anchor);
    }
}

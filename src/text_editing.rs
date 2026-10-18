//! Inline formatting uses Unicode character offsets in the plain-text body.
//! Checklist anchors expand to their existing textual prefixes in stored ranges.
use crate::note_session::NoteSession;
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mark {
    Bold,
    Italic,
    Underline,
    Strike,
}
impl Mark {
    pub const ALL: [Self; 4] = [Self::Bold, Self::Italic, Self::Underline, Self::Strike];
    fn name(self) -> &'static str {
        match self {
            Self::Bold => "inline-bold",
            Self::Italic => "inline-italic",
            Self::Underline => "inline-underline",
            Self::Strike => "inline-strike",
        }
    }
    pub fn tag(self, buffer: &gtk::TextBuffer) -> gtk::TextTag {
        buffer.tag_table().lookup(self.name()).unwrap_or_else(|| {
            let builder = gtk::TextTag::builder().name(self.name());
            let tag = match self {
                Self::Bold => builder.weight(700).build(),
                Self::Italic => builder.style(gtk::pango::Style::Italic).build(),
                Self::Underline => builder.build(),
                Self::Strike => builder.build(),
            };
            buffer.tag_table().add(&tag);
            tag
        })
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub start: i32,
    pub end: i32,
    pub mark: Mark,
}

pub fn offset(buffer: &gtk::TextBuffer, at: &gtk::TextIter) -> i32 {
    crate::checklist::offset(buffer, at)
}
pub fn iter(buffer: &gtk::TextBuffer, offset: i32) -> gtk::TextIter {
    crate::checklist::iter(buffer, offset)
}
pub fn spans(buffer: &gtk::TextBuffer) -> Vec<Span> {
    let mut spans = Vec::new();
    for mark in Mark::ALL {
        let Some(tag) = buffer.tag_table().lookup(mark.name()) else {
            continue;
        };
        let mut at = buffer.start_iter();
        let mut position = 0;
        let mut start = None;
        loop {
            let flags = if at.is_end() {
                vec![false]
            } else if let Some(formats) = crate::checklist::formats(&at) {
                formats.map(|bits| bits & (1 << mark as u8) != 0).to_vec()
            } else if at.child_anchor().is_some() {
                Vec::new()
            } else {
                vec![at.has_tag(&tag)]
            };
            for tagged in flags {
                match (start, tagged) {
                    (None, true) => start = Some(position),
                    (Some(begin), false) => {
                        spans.push(Span {
                            start: begin,
                            end: position,
                            mark,
                        });
                        start = None;
                    }
                    _ => {}
                }
                position += 1;
            }
            if at.is_end() {
                break;
            }
            at.forward_char();
        }
    }
    spans
}
pub fn apply(buffer: &gtk::TextBuffer, spans: &[Span]) {
    for mark in Mark::ALL {
        let tag = mark.tag(buffer);
        buffer.remove_tag(&tag, &buffer.start_iter(), &buffer.end_iter());
    }
    let mut at = buffer.start_iter();
    while !at.is_end() {
        crate::checklist::set_formats(&at, [0; 6]);
        at.forward_char();
    }
    for span in spans {
        if span.start >= 0 && span.end > span.start {
            buffer.apply_tag(
                &span.mark.tag(buffer),
                &iter(buffer, span.start),
                &iter(buffer, span.end),
            );
            format_prefixes(buffer, span.start, span.end, span.mark, true);
        }
    }
}

// Prefix characters have no GTK text positions, but their existing saved
// formatting must still round-trip, including spans ending inside a prefix.
fn format_prefixes(buffer: &gtk::TextBuffer, start: i32, end: i32, mark: Mark, active: bool) {
    let mut at = buffer.start_iter();
    let mut position = 0;
    while !at.is_end() {
        if let Some(mut formats) = crate::checklist::formats(&at) {
            for bits in &mut formats {
                if (start..end).contains(&position) {
                    if active {
                        *bits |= 1 << mark as u8;
                    } else {
                        *bits &= !(1 << mark as u8);
                    }
                }
                position += 1;
            }
            crate::checklist::set_formats(&at, formats);
        } else if at.child_anchor().is_none() {
            position += 1;
        }
        at.forward_char();
    }
}

pub fn format_selection_prefixes(
    buffer: &gtk::TextBuffer,
    start: &gtk::TextIter,
    end: &gtk::TextIter,
    mark: Mark,
    active: bool,
) {
    format_prefixes(
        buffer,
        offset(buffer, start),
        offset(buffer, end),
        mark,
        active,
    );
}

/// A mixed selection is inactive; anchors and hidden checklist markers don't count.
pub fn selection_has_mark(buffer: &gtk::TextBuffer, mark: Mark) -> bool {
    let Some((mut at, end)) = buffer.selection_bounds() else {
        return false;
    };
    let Some(tag) = buffer.tag_table().lookup(mark.name()) else {
        return false;
    };
    let mut text = false;
    while at < end {
        if at.child_anchor().is_none()
            && at.char() != '\n'
            && !at.tags().iter().any(|tag| tag.is_invisible())
        {
            if !at.has_tag(&tag) {
                return false;
            }
            text = true;
        }
        at.forward_char();
    }
    text
}

/// Keep body history shortcuts attached when All Notes replaces its buffer.
pub fn bind_history(view: &gtk::TextView, session: impl Fn() -> Option<Rc<NoteSession>> + 'static) {
    let session: SessionGetter = Rc::new(session);
    bind_context_history(view.upcast_ref(), session.clone());
    bind_edit_boundaries(view.upcast_ref(), session.clone());
    let getter = session.clone();
    view.connect_move_cursor(move |_, _, _, _| {
        if let Some(session) = getter() {
            session.break_typing();
        }
    });
    for signal in ["paste-clipboard", "cut-clipboard"] {
        let getter = session.clone();
        view.connect_local(signal, false, move |_| {
            if let Some(session) = getter() {
                session.isolate_next_edit();
            }
            None
        });
    }
    let getter = session.clone();
    view.connect_local("cut-clipboard", true, move |_| {
        if let Some(session) = getter() {
            session.finish_isolated_edit();
        }
        None
    });
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak_view = view.downgrade();
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if !modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || !weak_view.upgrade().is_some_and(|view| view.is_editable())
        {
            return gtk::glib::Propagation::Proceed;
        }
        let Some(session) = session() else {
            return gtk::glib::Propagation::Proceed;
        };
        if key.to_lower() == gtk::gdk::Key::a {
            session.break_typing();
        }
        match key.to_lower() {
            gtk::gdk::Key::z => {
                session.undo(modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK))
            }
            gtk::gdk::Key::y => session.undo(true),
            gtk::gdk::Key::b => session.toggle_mark(Mark::Bold),
            gtk::gdk::Key::i => session.toggle_mark(Mark::Italic),
            gtk::gdk::Key::u => session.toggle_mark(Mark::Underline),
            _ => return gtk::glib::Propagation::Proceed,
        }
        gtk::glib::Propagation::Stop
    });
    view.add_controller(keys);
}

type SessionGetter = Rc<dyn Fn() -> Option<Rc<NoteSession>>>;

// GTK creates its text menus lazily. Configure the actual native popover rather
// than adding a second menu or extra Undo/Redo entries.
fn observe_text_popovers(widget: &gtk::Widget, added: impl Fn(&gtk::Widget) + 'static) {
    let children = widget.observe_children();
    for index in 0..children.n_items() {
        if let Some(child) = children.item(index).and_downcast::<gtk::Widget>() {
            added(&child);
        }
    }
    let id = children.connect_items_changed(move |children, position, _, count| {
        for index in position..position + count {
            if let Some(child) = children.item(index).and_downcast::<gtk::Widget>() {
                added(&child);
            }
        }
    });
    // The model borrows its owner; retain it only for the widget's lifetime.
    let id = std::cell::RefCell::new(Some(id));
    widget.connect_destroy(move |_| {
        if let Some(id) = id.borrow_mut().take() {
            children.disconnect(id);
        }
    });
}

fn text_is_editable(widget: &gtk::Widget) -> bool {
    widget.downcast_ref::<gtk::TextView>().map_or_else(
        || {
            widget
                .downcast_ref::<gtk::Text>()
                .is_some_and(|text| text.is_editable())
        },
        |view| view.is_editable(),
    )
}

fn bind_context_history(widget: &gtk::Widget, session: SessionGetter) {
    let source = widget.downgrade();
    observe_text_popovers(widget, move |child| {
        let Some(menu) = child.downcast_ref::<gtk::PopoverMenu>() else {
            return;
        };
        let actions = gtk::gio::SimpleActionGroup::new();
        let undo = gtk::gio::SimpleAction::new("undo", None);
        let redo = gtk::gio::SimpleAction::new("redo", None);
        for (action, is_redo) in [(&undo, false), (&redo, true)] {
            action.set_enabled(false);
            let source = source.clone();
            let getter = session.clone();
            action.connect_activate(move |_, _| {
                if source
                    .upgrade()
                    .is_some_and(|source| text_is_editable(&source))
                    && let Some(session) = getter()
                {
                    session.undo(is_redo);
                }
            });
            actions.add_action(action);
        }
        // A group on the popover takes precedence over its parent TextView/Text's
        // built-in text.undo/text.redo actions, which use the disabled GTK history.
        menu.insert_action_group("text", Some(&actions));
        let source = source.clone();
        let getter = session.clone();
        let refresh: Rc<dyn Fn()> = Rc::new(move || {
            let editable = source
                .upgrade()
                .is_some_and(|source| text_is_editable(&source));
            let session = getter();
            undo.set_enabled(editable && session.as_ref().is_some_and(|s| s.can_undo(false)));
            redo.set_enabled(editable && session.as_ref().is_some_and(|s| s.can_undo(true)));
        });
        let subscription = Rc::new(std::cell::RefCell::new(None::<Rc<dyn Fn()>>));
        let getter = session.clone();
        let keep = subscription.clone();
        menu.connect_map(move |_| {
            if let Some(session) = getter() {
                session.break_typing();
                let refresh = refresh.clone();
                keep.replace(Some(session.subscribe(move || refresh())));
            }
            refresh();
        });
        menu.connect_closed(move |_| {
            subscription.borrow_mut().take();
        });
    });
}

/// Reposition GTK's existing chooser beside the sheet, including title entry use.
pub fn bind_emoji_placement(
    title: &gtk::Entry,
    body: &gtk::TextView,
    sheet: &gtk::Overlay,
    footer: &[gtk::Popover],
    position: impl Fn() -> gtk::PositionType + 'static,
) {
    let position: Rc<dyn Fn() -> gtk::PositionType> = Rc::new(position);
    let mut sources = vec![body.clone().upcast::<gtk::Widget>()];
    if let Some(text) = title
        .delegate()
        .and_then(|d| d.downcast::<gtk::Text>().ok())
    {
        sources.push(text.upcast());
    }
    for source in sources {
        let sheet = sheet.downgrade();
        let source_weak = source.downgrade();
        let footer: Vec<_> = footer.iter().map(|p| p.downgrade()).collect();
        let position = position.clone();
        observe_text_popovers(&source, move |child| {
            let Some(chooser) = child.downcast_ref::<gtk::EmojiChooser>() else {
                return;
            };
            chooser.add_css_class("note-popover");
            chooser.add_css_class("note-emoji");
            compact_emoji_contents(chooser.upcast_ref());
            let picked = Rc::new(std::cell::Cell::new(false));
            let selected = picked.clone();
            chooser.connect_emoji_picked(move |_, _| {
                selected.set(true);
                let selected = selected.clone();
                // Ctrl-selection already keeps GTK's chooser open.
                gtk::glib::idle_add_local_once(move || selected.set(false));
            });
            let source = source_weak.clone();
            chooser.connect_closed(move |chooser| {
                // GTK hides after emoji-picked. Restore only that closure, leaving
                // Escape and outside-click dismissal to the native popover.
                if !picked.replace(false)
                    || !source
                        .upgrade()
                        .is_some_and(|source| source.is_mapped() && text_is_editable(&source))
                {
                    return;
                }
                let mut widgets = vec![chooser.clone().upcast::<gtk::Widget>()];
                let mut entries = Vec::new();
                let mut scrolls = Vec::new();
                while let Some(widget) = widgets.pop() {
                    if let Ok(entry) = widget.clone().dynamic_cast::<gtk::Editable>() {
                        entries.push((entry.clone(), entry.text()));
                    }
                    if let Some(scroll) = widget.downcast_ref::<gtk::ScrolledWindow>() {
                        let adjustment = scroll.vadjustment();
                        scrolls.push((adjustment.clone(), adjustment.value()));
                    }
                    let mut child = widget.first_child();
                    while let Some(widget) = child {
                        child = widget.next_sibling();
                        widgets.push(widget);
                    }
                }
                chooser.popup();
                for (entry, query) in entries {
                    entry.set_text(&query);
                }
                for (adjustment, value) in &scrolls {
                    adjustment.set_value(*value);
                }
                let weak = chooser.downgrade();
                gtk::glib::idle_add_local_once(move || {
                    if let Some(chooser) = weak.upgrade()
                        && chooser.is_visible()
                    {
                        // GTK's popdown returns focus to the editor after closed.
                        chooser.child_focus(gtk::DirectionType::TabForward);
                        for (adjustment, value) in scrolls {
                            adjustment.set_value(value);
                        }
                    }
                });
            });
            let sheet = sheet.clone();
            let source = source_weak.clone();
            let footer = footer.clone();
            let position = position.clone();
            chooser.connect_show(move |chooser| {
                let (Some(sheet), Some(source)) = (sheet.upgrade(), source.upgrade()) else {
                    return;
                };
                // The right-click menu and footer panels close before the chooser.
                let mut child = source.first_child();
                while let Some(widget) = child {
                    if let Some(menu) = widget.downcast_ref::<gtk::PopoverMenu>() {
                        menu.popdown();
                    }
                    child = widget.next_sibling();
                }
                for panel in &footer {
                    if let Some(panel) = panel.upgrade() {
                        panel.popdown();
                    }
                }
                let Some(bounds) = sheet.compute_bounds(&source) else {
                    return;
                };
                let side = position();
                let (x, y) = match side {
                    gtk::PositionType::Left => (bounds.x(), bounds.y() + bounds.height() / 2.),
                    gtk::PositionType::Right => (
                        bounds.x() + bounds.width(),
                        bounds.y() + bounds.height() / 2.,
                    ),
                    gtk::PositionType::Top => (bounds.x() + bounds.width() / 2., bounds.y()),
                    _ => (
                        bounds.x() + bounds.width() / 2.,
                        bounds.y() + bounds.height(),
                    ),
                };
                chooser.set_position(side);
                chooser.set_pointing_to(Some(&gtk::gdk::Rectangle::new(
                    x.round() as i32,
                    y.round() as i32,
                    1,
                    1,
                )));
            });
        });
    }
}

fn compact_emoji_contents(widget: &gtk::Widget) {
    if let Some(scroll) = widget.downcast_ref::<gtk::ScrolledWindow>() {
        scroll.set_min_content_height(120);
        scroll.set_max_content_height(180);
        scroll.set_propagate_natural_height(true);
    }
    if let Some(grid) = widget.downcast_ref::<gtk::FlowBox>()
        && !grid.has_css_class("emoji-toolbar")
    {
        grid.set_min_children_per_line(8);
        grid.set_max_children_per_line(8);
    }
    let mut child = widget.first_child();
    while let Some(widget) = child {
        compact_emoji_contents(&widget);
        child = widget.next_sibling();
    }
}

fn bind_edit_boundaries(widget: &gtk::Widget, session: SessionGetter) {
    let events = gtk::EventControllerLegacy::new();
    events.set_propagation_phase(gtk::PropagationPhase::Capture);
    let getter = session.clone();
    events.connect_event(move |_, event| {
        if let Some(session) = getter() {
            if event.event_type() == gtk::gdk::EventType::DropStart
                || event
                    .downcast_ref::<gtk::gdk::ButtonEvent>()
                    .is_some_and(|e| e.button() == 2)
            {
                session.isolate_next_edit();
            } else if event.event_type() == gtk::gdk::EventType::ButtonPress {
                session.break_typing();
            }
        }
        gtk::glib::Propagation::Proceed
    });
    widget.add_controller(events);
    widget.connect_has_focus_notify(move |_| {
        if let Some(session) = session() {
            session.break_typing();
        }
    });
}

pub fn bind_title_history(
    view: &gtk::Entry,
    session: impl Fn() -> Option<Rc<NoteSession>> + 'static,
) {
    view.set_enable_undo(false);
    let session: SessionGetter = Rc::new(session);
    bind_edit_boundaries(view.upcast_ref(), session.clone());
    let getter = session.clone();
    view.connect_activate(move |_| {
        if let Some(session) = getter() {
            session.break_typing();
        }
    });
    if let Some(text) = view.delegate().and_then(|d| d.downcast::<gtk::Text>().ok()) {
        bind_context_history(text.upcast_ref(), session.clone());
        let getter = session.clone();
        text.connect_insert_text(move |_, _, position| {
            if let Some(session) = getter() {
                session.note_change(crate::edit_history::Field::Title, *position);
            }
        });
        let getter = session.clone();
        text.connect_delete_text(move |_, start, _| {
            if let Some(session) = getter() {
                session.note_change(crate::edit_history::Field::Title, start);
            }
        });
        let getter = session.clone();
        text.connect_move_cursor(move |_, _, _, _| {
            if let Some(session) = getter() {
                session.break_typing();
            }
        });
        for signal in ["paste-clipboard", "cut-clipboard"] {
            let getter = session.clone();
            text.connect_local(signal, false, move |_| {
                if let Some(session) = getter() {
                    session.isolate_next_edit();
                }
                None
            });
        }
        let getter = session.clone();
        text.connect_local("cut-clipboard", true, move |_| {
            if let Some(session) = getter() {
                session.finish_isolated_edit();
            }
            None
        });
    }
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak_view = view.downgrade();
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if !modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || !weak_view.upgrade().is_some_and(|view| view.is_editable())
        {
            return gtk::glib::Propagation::Proceed;
        }
        let Some(session) = session() else {
            return gtk::glib::Propagation::Proceed;
        };
        match key.to_lower() {
            gtk::gdk::Key::z => {
                session.undo(modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK))
            }
            gtk::gdk::Key::y => session.undo(true),
            gtk::gdk::Key::a => {
                session.break_typing();
                return gtk::glib::Propagation::Proceed;
            }
            _ => return gtk::glib::Propagation::Proceed,
        }
        gtk::glib::Propagation::Stop
    });
    view.add_controller(keys);
}

pub fn copy_text(body: &gtk::TextView) {
    let buffer = body.buffer();
    let (start, end) = buffer.bounds();
    body.clipboard()
        .set_text(&crate::checklist::text(&buffer, &start, &end));
}

/// Keep GTK's in-process formatting transfer while exporting canonical plain text.
pub fn copy_selection(body: &gtk::TextView) -> Option<(gtk::TextIter, gtk::TextIter)> {
    let buffer = body.buffer();
    // GTK also supports copying/cutting from its keyboard "anchor" mark when
    // there is no ordinary selection. Preserve that native range convention.
    let (mut start, mut end) = buffer.selection_bounds().or_else(|| {
        buffer.mark("anchor").map(|anchor| {
            (
                buffer.iter_at_mark(&buffer.get_insert()),
                buffer.iter_at_mark(&anchor),
            )
        })
    })?;
    start.order(&mut end);
    if start == end {
        return None;
    }
    let first = offset(&buffer, &start);
    let last = offset(&buffer, &end);
    let contents = gtk::TextBuffer::new(Some(&buffer.tag_table()));
    contents.set_text(&crate::checklist::text(&buffer, &start, &end));
    let formatting = spans(&buffer)
        .into_iter()
        .filter_map(|span| {
            let start = span.start.max(first);
            let end = span.end.min(last);
            (start < end).then_some(Span {
                start: start - first,
                end: end - first,
                mark: span.mark,
            })
        })
        .collect::<Vec<_>>();
    apply(&contents, &formatting);
    contents.select_range(&contents.start_iter(), &contents.end_iter());
    body.clipboard()
        .set_content(Some(&gtk::gdk::ContentProvider::for_value(
            &contents.to_value(),
        )))
        .ok()?;
    Some((start, end))
}

/// A fixed footer page follows the current buffer's selection, without a popup.
pub struct SelectionTools {
    pub row: gtk::Box,
    view: gtk::glib::WeakRef<gtk::TextView>,
}
impl SelectionTools {
    pub fn new(
        view: &gtk::TextView,
        pages: &gtk::Stack,
        session: impl Fn() -> Option<Rc<NoteSession>> + 'static,
        footer: &[gtk::Popover],
    ) -> Self {
        let session: SessionGetter = Rc::new(session);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.add_css_class("selection-tools");
        row.set_halign(gtk::Align::Center);
        let mut buttons = Vec::new();
        for (mark, label, tooltip) in [
            (Mark::Bold, "B", "Bold"),
            (Mark::Italic, "I", "Italic"),
            (Mark::Underline, "U", "Underline"),
            (Mark::Strike, "S", "Strikethrough"),
        ] {
            let button = gtk::ToggleButton::with_label(label);
            button.set_tooltip_text(Some(tooltip));
            button.update_property(&[gtk::accessible::Property::Label(tooltip)]);
            button.set_focus_on_click(false);
            let getter = session.clone();
            let weak = view.downgrade();
            button.connect_clicked(move |_| {
                if weak.upgrade().is_some_and(|view| view.is_editable())
                    && let Some(session) = getter()
                {
                    session.toggle_mark(mark);
                }
            });
            row.append(&button);
            buttons.push((mark, button.downgrade()));
        }
        pages.add_named(&row, Some("selection"));
        let weak_view = view.downgrade();
        let weak_pages = pages.downgrade();
        let panels: Vec<_> = footer.iter().map(|p| p.downgrade()).collect();
        let update: Rc<dyn Fn()> = Rc::new(move || {
            let (Some(view), Some(pages)) = (weak_view.upgrade(), weak_pages.upgrade()) else {
                return;
            };
            let selected = view.is_editable() && view.buffer().has_selection();
            for (mark, button) in &buttons {
                if let Some(button) = button.upgrade() {
                    button.set_active(selected && selection_has_mark(&view.buffer(), *mark));
                }
            }
            if selected {
                for panel in &panels {
                    if let Some(panel) = panel.upgrade()
                        && panel.is_visible()
                    {
                        panel.popdown();
                    }
                }
            }
            let name = if selected { "selection" } else { "settings" };
            if pages.visible_child_name().as_deref() != Some(name) {
                pages.set_visible_child_name(name);
            }
        });
        let handler = Rc::new(std::cell::RefCell::new(
            None::<(gtk::TextBuffer, Vec<gtk::glib::SignalHandlerId>)>,
        ));
        let bind = {
            let update = update.clone();
            move |view: &gtk::TextView| {
                if let Some((buffer, ids)) = handler.borrow_mut().take() {
                    for id in ids {
                        buffer.disconnect(id);
                    }
                }
                let buffer = view.buffer();
                let refresh = update.clone();
                let mut ids = vec![buffer.connect_has_selection_notify(move |_| refresh())];
                let refresh = update.clone();
                ids.push(buffer.connect_mark_set(move |_, _, mark| {
                    if matches!(mark.name().as_deref(), Some("insert" | "selection_bound")) {
                        refresh();
                    }
                }));
                for signal in ["apply-tag", "remove-tag"] {
                    let refresh = update.clone();
                    ids.push(buffer.connect_local(signal, true, move |_| {
                        refresh();
                        None
                    }));
                }
                handler.replace(Some((buffer, ids)));
                update();
            }
        };
        bind(view);
        view.connect_buffer_notify(move |view| bind(view));
        let refresh = update.clone();
        view.connect_editable_notify(move |_| refresh());
        Self {
            row,
            view: view.downgrade(),
        }
    }
    pub fn dismiss(&self) -> bool {
        let Some(view) = self.view.upgrade() else {
            return false;
        };
        let buffer = view.buffer();
        let selected = buffer.has_selection();
        if selected {
            buffer.place_cursor(&buffer.iter_at_mark(&buffer.get_insert()));
        }
        selected
    }
}

#[cfg(test)]
pub(crate) fn capture_native(name: &str) {
    if let Ok(path) = std::env::var("STICKIES_NATIVE_CAPTURES") {
        let path = std::path::Path::new(&path);
        std::fs::create_dir_all(path).unwrap();
        assert!(
            std::process::Command::new("grim")
                .arg(path.join(format!("{name}.png")))
                .status()
                .unwrap()
                .success()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a GTK display; uses a disposable database"]
    fn formatting_survives_unicode_checklists_history_and_reopen() {
        gtk::init().unwrap();
        let root = std::env::temp_dir().join(format!("stickies-formatting-{}", std::process::id()));
        let storage = Rc::new(crate::storage::Storage::open(&root.join("notes.db")).unwrap());
        let note = storage
            .create_note(
                "Format",
                "- [ ] café 東京 👩🏽‍💻",
                crate::app::NoteColor::Yellow,
            )
            .unwrap();
        let id = note.id;
        let session = NoteSession::new(note, storage.clone(), || {});
        let view = gtk::TextView::with_buffer(&session.body);
        session.attach_view(&view);
        let weak_session = Rc::downgrade(&session);
        bind_history(&view, move || weak_session.upgrade());
        let keys = view
            .observe_controllers()
            .iter::<gtk::glib::Object>()
            .flatten()
            .find_map(|controller| controller.downcast::<gtk::EventControllerKey>().ok())
            .unwrap();
        let key =
            |key, modifiers| keys.emit_by_name::<bool>("key-pressed", &[&key, &0u32, &modifiers]);

        session
            .body
            .select_range(&iter(&session.body, 6), &iter(&session.body, 10));
        session.toggle_mark(Mark::Underline);
        let expected = vec![Span {
            start: 6,
            end: 10,
            mark: Mark::Underline,
        }];
        assert_eq!(spans(&session.body), expected);
        assert!(!key(gtk::gdk::Key::f, gtk::gdk::ModifierType::CONTROL_MASK));

        session.body.insert(&mut iter(&session.body, 8), "é");
        assert_eq!(spans(&session.body)[0].end, 11);
        assert!(key(gtk::gdk::Key::z, gtk::gdk::ModifierType::CONTROL_MASK));
        assert_eq!(spans(&session.body), expected);
        assert!(key(gtk::gdk::Key::z, gtk::gdk::ModifierType::CONTROL_MASK));
        assert!(spans(&session.body).is_empty());
        assert!(key(
            gtk::gdk::Key::z,
            gtk::gdk::ModifierType::CONTROL_MASK | gtk::gdk::ModifierType::SHIFT_MASK
        ));
        assert_eq!(spans(&session.body), expected);
        session.body.place_cursor(&iter(&session.body, 8));
        crate::editor::toggle_line(&view, "- [ ] ");
        assert_eq!(
            spans(&session.body),
            expected,
            "prefix changes preserve formatting"
        );
        assert!(session.flush());
        assert_eq!(storage.load_note(id).unwrap().body, "- [x] café 東京 👩🏽‍💻");
        assert_eq!(storage.load_formatting(id).unwrap(), expected);
        let reopened = NoteSession::new(storage.load_note(id).unwrap(), storage.clone(), || {});
        assert_eq!(spans(&reopened.body), expected);
        // A rejected update must preserve both text and its formatting metadata.
        let db = rusqlite::Connection::open(root.join("notes.db")).unwrap();
        db.execute_batch("CREATE TRIGGER reject BEFORE UPDATE ON notes BEGIN SELECT RAISE(FAIL, 'write rejected'); END;").unwrap();
        session.body.set_text("replacement");
        assert!(!session.flush());
        assert_eq!(storage.load_formatting(id).unwrap(), expected);
        assert!(storage.load_note(id).unwrap().body.starts_with("- [x]"));
        db.execute_batch("DROP TRIGGER reject;").unwrap();
        assert!(session.flush());
        assert!(storage.load_formatting(id).unwrap().is_empty());
        session.body.set_text("Keyboard formatting");
        session
            .body
            .select_range(&iter(&session.body, 0), &iter(&session.body, 8));
        for key_name in [gtk::gdk::Key::b, gtk::gdk::Key::i, gtk::gdk::Key::u] {
            assert!(key(key_name, gtk::gdk::ModifierType::CONTROL_MASK));
        }
        assert_eq!(spans(&session.body).len(), 3);
        assert!(key(gtk::gdk::Key::z, gtk::gdk::ModifierType::CONTROL_MASK));
        assert_eq!(spans(&session.body).len(), 2);
        assert!(key(gtk::gdk::Key::y, gtk::gdk::ModifierType::CONTROL_MASK));
        assert_eq!(spans(&session.body).len(), 3);
        assert!(session.flush());
        drop(db);
        drop(reopened);
        drop(view);
        drop(session);
        drop(storage);
        std::fs::remove_dir_all(root).unwrap();
    }
}

//! Native text editing with readable underline/strike strokes for every bundled font.
//! Tags and stored spans stay unchanged; only decoration painting differs.
use gtk::{glib, prelude::*, subclass::prelude::*};

mod imp {
    use super::*;
    #[derive(Default)]
    pub struct NoteText {
        buffer_handlers: std::cell::RefCell<Option<(gtk::TextBuffer, Vec<glib::SignalHandlerId>)>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for NoteText {
        const NAME: &'static str = "StickiesNoteText";
        type Type = super::NoteText;
        type ParentType = gtk::TextView;
    }
    impl ObjectImpl for NoteText {
        fn constructed(&self) {
            self.parent_constructed();
            self.obj()
                .connect_buffer_notify(|view| view.imp().bind_buffer());
            self.bind_buffer();
        }
        fn dispose(&self) {
            self.disconnect_buffer();
        }
    }
    impl NoteText {
        fn disconnect_buffer(&self) {
            if let Some((buffer, handlers)) = self.buffer_handlers.borrow_mut().take() {
                for handler in handlers {
                    buffer.disconnect(handler);
                }
            }
        }
        fn bind_buffer(&self) {
            self.disconnect_buffer();
            let buffer = self.obj().buffer();
            // These tags carry stored spans, without a native decoration property.
            // Repaint after applying/removing one, including history restoration.
            let handlers = ["apply-tag", "remove-tag"].map(|signal| {
                let weak = self.obj().downgrade();
                buffer.connect_local(signal, true, move |_| {
                    if let Some(view) = weak.upgrade() {
                        view.queue_draw();
                    }
                    None
                })
            });
            self.buffer_handlers
                .replace(Some((buffer, handlers.into())));
            self.obj().queue_draw();
        }
    }
    impl WidgetImpl for NoteText {}
    impl TextViewImpl for NoteText {
        fn copy_clipboard(&self) {
            let view = self.obj();
            if crate::text_editing::copy_selection(view.upcast_ref()).is_none() {
                self.parent_copy_clipboard();
            }
        }
        fn cut_clipboard(&self) {
            let view = self.obj();
            if !view.is_editable() {
                return;
            }
            let buffer = view.buffer();
            if let Some((mut start, mut end)) =
                crate::text_editing::copy_selection(view.upcast_ref())
            {
                buffer.begin_user_action();
                buffer.delete_interactive(&mut start, &mut end, true);
                buffer.end_user_action();
            } else {
                self.parent_cut_clipboard();
            }
        }
        fn snapshot_layer(&self, layer: gtk::TextViewLayer, snapshot: gtk::Snapshot) {
            self.parent_snapshot_layer(layer, snapshot.clone());
            if layer == gtk::TextViewLayer::AboveText {
                super::decorate(self.obj().upcast_ref(), &snapshot);
            }
        }
    }
}
glib::wrapper! {
    pub struct NoteText(ObjectSubclass<imp::NoteText>)
        @extends gtk::TextView, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Scrollable;
}

pub fn view(buffer: Option<&gtk::TextBuffer>) -> gtk::TextView {
    let view: NoteText = glib::Object::new();
    if let Some(buffer) = buffer {
        view.set_buffer(Some(buffer));
    }
    view.upcast()
}

fn decoration_bounds(view: &gtk::TextView, at: &gtk::TextIter) -> gtk::gdk::Rectangle {
    if at.line_index() == at.visible_line_index() {
        return view.iter_location(at);
    }
    // iter_location indexes the laid-out line with the raw byte offset, including
    // hidden checklist prefixes. Cursor locations account for invisible text.
    let (start, weak_start) = view.cursor_locations(Some(at));
    let mut end = *at;
    end.forward_cursor_position();
    let (finish, weak_finish) = view.cursor_locations(Some(&end));
    let pair = [start, weak_start]
        .into_iter()
        .flat_map(|start| [finish, weak_finish].map(|finish| (start, finish)))
        .filter(|(start, finish)| start.y() == finish.y() && start.x() != finish.x())
        .min_by_key(|(start, finish)| start.x().abs_diff(finish.x()));
    if let Some((start, finish)) = pair {
        return gtk::gdk::Rectangle::new(
            start.x().min(finish.x()),
            start.y(),
            start.x().abs_diff(finish.x()) as i32,
            start.height(),
        );
    }
    // At a soft wrap the next cursor belongs to the next line. Measure only the
    // final grapheme, preserving its font emphasis; whitespace has no stroke.
    let text = at.buffer().text(at, &end, false);
    let layout = view.create_pango_layout(Some(&text));
    let attrs = gtk::pango::AttrList::new();
    for tag in at.tags() {
        match tag.name().as_deref() {
            Some("inline-bold") => {
                attrs.insert(gtk::pango::AttrInt::new_weight(gtk::pango::Weight::Bold))
            }
            Some("inline-italic") => {
                attrs.insert(gtk::pango::AttrInt::new_style(gtk::pango::Style::Italic))
            }
            _ => (),
        }
    }
    layout.set_attributes(Some(&attrs));
    let width = if text.trim().is_empty() {
        0
    } else {
        layout.pixel_size().0
    };
    let rtl = layout.direction(0) == gtk::pango::Direction::Rtl;
    gtk::gdk::Rectangle::new(
        start.x() - if rtl { width } else { 0 },
        start.y(),
        width,
        start.height(),
    )
}

fn decorate(view: &gtk::TextView, snapshot: &gtk::Snapshot) {
    let buffer = view.buffer();
    let table = buffer.tag_table();
    let underline = table.lookup("inline-underline");
    let strike = table.lookup("inline-strike");
    let completed = table.lookup("completed");
    if underline.is_none() && strike.is_none() && completed.is_none() {
        return;
    }
    let visible = view.visible_rect();
    // Use the logical line directly. Hit-testing an empty/hidden-marker line
    // can feed GTK an out-of-range visible byte index during list edits.
    let (mut at, _) = view.line_at_y(visible.y());
    let ink = view.color();
    let metrics = view.pango_context().metrics(None, None);
    let ascent = metrics.ascent() as f32 / gtk::pango::SCALE as f32;
    let descent = metrics.descent() as f32 / gtk::pango::SCALE as f32;
    let em = (ascent + descent).max(1.);
    // Pango's strike/underline thickness can round below a pixel in handwriting fonts.
    let thickness = (em * 0.065).clamp(1.25, 3.);
    while !at.is_end() {
        if at.starts_line() && view.line_yrange(&at).0 > visible.y() + visible.height() {
            break;
        }
        let has = |tag: &Option<gtk::TextTag>| tag.as_ref().is_some_and(|tag| at.has_tag(tag));
        let completed_here = has(&completed);
        let underlined = has(&underline);
        let struck = has(&strike) || completed_here;
        if (!underlined && !struck)
            || at.child_anchor().is_some()
            || at.char() == '\n'
            || at.tags().iter().any(|tag| tag.is_invisible())
        {
            at.forward_char();
            continue;
        }
        let rect = decoration_bounds(view, &at);
        if rect.y() > visible.y() + visible.height() {
            break;
        }
        if rect.width() != 0 {
            let color = if completed_here {
                completed
                    .as_ref()
                    .and_then(|tag| tag.foreground_rgba())
                    .unwrap_or(ink)
            } else {
                ink
            };
            let baseline = rect.y() as f32 + (rect.height() as f32 - em).max(0.) / 2. + ascent;
            let x = rect.x().min(rect.x() + rect.width()) as f32;
            let width = rect.width().unsigned_abs() as f32;
            if underlined {
                let y =
                    (baseline + descent * 0.25).min((rect.y() + rect.height()) as f32 - thickness);
                snapshot.append_color(&color, &gtk::graphene::Rect::new(x, y, width, thickness));
            }
            if struck {
                let y = baseline - ascent * 0.36;
                snapshot.append_color(&color, &gtk::graphene::Rect::new(x, y, width, thickness));
            }
        }
        at.forward_cursor_position();
    }
}

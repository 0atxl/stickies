#[cfg(test)]
use crate::app::NoteColor;
use crate::edge_surface::{Edge, KeyboardMode, Layer, LayerShell};
use crate::{
    Ui,
    app::{Action, NoteId},
    appearance,
    config::{self, Config, ScreenEdge},
    editor, motion,
    storage::NoteCollection,
    ui_assets,
};
use gtk::{glib, prelude::*, subclass::prelude::ObjectSubclassIsExt};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

// Keep transformed children out of the tab group's measurement. GtkFixed uses
// its layout manager directly, so overriding Widget::measure alone has no effect.
mod stacked_tabs {
    use gtk::{glib, prelude::*, subclass::prelude::*};
    #[derive(Default)]
    pub struct TabLayout {
        pub pointer: std::cell::Cell<f64>,
        pub front: std::cell::Cell<Option<usize>>,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for TabLayout {
        const NAME: &'static str = "StickiesTabLayout";
        type Type = super::TabLayout;
        type ParentType = gtk::Fixed;
        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<super::TabMeasure>();
        }
    }
    impl ObjectImpl for TabLayout {}
    impl WidgetImpl for TabLayout {
        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let mut children = Vec::new();
            let mut child = widget.first_child();
            while let Some(tab) = child {
                child = tab.next_sibling();
                children.push(tab);
            }
            let front = self.front.get();
            for index in super::tab_draw_order(children.len(), front) {
                widget.snapshot_child(&children[index], snapshot);
            }
        }
    }
    impl FixedImpl for TabLayout {}

    #[derive(Default)]
    pub struct TabMeasure;
    #[glib::object_subclass]
    impl ObjectSubclass for TabMeasure {
        const NAME: &'static str = "StickiesTabMeasure";
        type Type = super::TabMeasure;
        type ParentType = gtk::LayoutManager;
    }
    impl ObjectImpl for TabMeasure {}
    impl LayoutManagerImpl for TabMeasure {
        fn layout_child_type() -> Option<glib::Type> {
            Some(gtk::FixedLayoutChild::static_type())
        }
        fn measure(
            &self,
            widget: &gtk::Widget,
            orientation: gtk::Orientation,
            _for_size: i32,
        ) -> (i32, i32, i32, i32) {
            let size = if orientation == gtk::Orientation::Horizontal {
                widget.width_request()
            } else {
                widget.height_request()
            }
            .max(0);
            (size, size, -1, -1)
        }
        fn allocate(&self, widget: &gtk::Widget, _width: i32, _height: i32, _baseline: i32) {
            let mut child = widget.first_child();
            while let Some(tab) = child {
                child = tab.next_sibling();
                let layout = self
                    .obj()
                    .layout_child(&tab)
                    .downcast::<gtk::FixedLayoutChild>()
                    .unwrap();
                tab.allocate(
                    tab.width_request(),
                    tab.height_request(),
                    -1,
                    layout.transform(),
                );
            }
        }
    }
}
glib::wrapper! {
    pub struct TabLayout(ObjectSubclass<stacked_tabs::TabLayout>)
        @extends gtk::Fixed, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
glib::wrapper! {
    pub struct TabMeasure(ObjectSubclass<stacked_tabs::TabMeasure>) @extends gtk::LayoutManager;
}

const WIDTH: i32 = 420;
const HEIGHT: i32 = 500;
const TAB_GAP: i32 = -8;
fn tab_gap(config: &Config) -> i32 {
    match config.tab_layout {
        config::TabLayout::Classic => 4,
        config::TabLayout::Stacked => TAB_GAP,
    }
}
// Covers the largest horizontal wave at either end, including its shadow.
const TAB_INSET: i32 = 24;
const TAB_MAGNIFICATION: f64 = 0.24;

// Draw toward the hovered tab from either end. A neighbor must not be trapped
// behind both the hovered tab and a more distant tab. Clicks use the reverse.
fn tab_draw_order(count: usize, front: Option<usize>) -> Vec<usize> {
    let mut order: Vec<_> = (0..count).collect();
    if let Some(front) = front {
        order.sort_by_key(|index| std::cmp::Reverse(index.abs_diff(front)));
    }
    order
}

fn tab_hover_target(index: usize, position: f64, layout: config::TabLayout) -> f64 {
    if !position.is_finite() {
        return 0.;
    }
    if layout == config::TabLayout::Classic {
        return if index == position.round() as usize {
            1.
        } else {
            0.
        };
    }
    let distance = ((index as f64 - position).abs() / 1.5).min(1.);
    (1. + (distance * std::f64::consts::PI).cos()) / 2.
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum EdgeState {
    Dormant,
    Tabs,
    Search,
    Docked(NoteId),
    Reposition,
}

impl EdgeState {
    pub(super) fn is_revealed(self) -> bool {
        self != Self::Dormant
    }
    pub(super) fn is_searching(self) -> bool {
        self == Self::Search
    }
    pub(super) fn is_repositioning(self) -> bool {
        self == Self::Reposition
    }
    pub(super) fn docked_id(self) -> Option<NoteId> {
        if let Self::Docked(id) = self {
            Some(id)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum EdgeEvent {
    Reveal,
    Collapse,
    Search,
    CloseSearch,
    Dock(NoteId),
    CloseDocked,
    Reposition,
    FinishReposition,
    Refresh,
    FocusWindow,
    FocusStrip,
    WindowFocused,
    StripFocused,
    Visibility,
}

#[derive(Clone, Copy)]
struct Policy {
    transient_visible: bool,
    hide_in_fullscreen: bool,
    window_active: bool,
    strip_active: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StripInput {
    Marker,
    Tabs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Effects {
    marker_visible: bool,
    tabs_visible: bool,
    panel: Option<&'static str>,
    pointer_visible: bool,
    strip_visible: bool,
    window_visible: bool,
    strip_keyboard: KeyboardMode,
    window_keyboard: KeyboardMode,
    strip_input: StripInput,
    panel_input: bool,
    strip_layer: Layer,
    window_layer: Layer,
}

// Save/close checks happen in the coordinator before it submits a transition.
// This function needs no GTK objects or display.
fn transition(state: EdgeState, event: EdgeEvent, policy: Policy) -> (EdgeState, Effects) {
    use EdgeEvent as Event;
    use EdgeState as State;
    let state = match (state, event) {
        (State::Dormant, Event::Reveal) => State::Tabs,
        (State::Dormant | State::Tabs, Event::Collapse) => State::Dormant,
        (State::Dormant | State::Tabs | State::Search, Event::Search) => State::Search,
        (State::Search, Event::CloseSearch) | (State::Docked(_), Event::CloseDocked) => State::Tabs,
        (State::Dormant | State::Tabs | State::Search | State::Docked(_), Event::Dock(id)) => {
            State::Docked(id)
        }
        (State::Dormant | State::Tabs, Event::Reposition) => State::Reposition,
        (State::Reposition, Event::FinishReposition) => State::Dormant,
        _ => state,
    };
    let panel = match state {
        State::Search => Some("search"),
        State::Docked(_) => Some("editor"),
        State::Reposition => Some("reposition"),
        State::Dormant | State::Tabs => None,
    };
    let transient_layer = if policy.hide_in_fullscreen {
        Layer::Top
    } else {
        Layer::Overlay
    };
    let mut effects = Effects {
        marker_visible: state == State::Dormant,
        tabs_visible: state.is_revealed(),
        panel,
        pointer_visible: matches!(state, State::Search | State::Docked(_)),
        strip_visible: policy.transient_visible,
        window_visible: state.docked_id().is_some() || policy.transient_visible,
        strip_keyboard: if state == State::Tabs {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::None
        },
        window_keyboard: if panel.is_some() {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::None
        },
        strip_input: if state == State::Dormant {
            StripInput::Marker
        } else {
            StripInput::Tabs
        },
        panel_input: panel.is_some(),
        strip_layer: transient_layer,
        window_layer: if state.docked_id().is_some() {
            Layer::Overlay
        } else {
            transient_layer
        },
    };
    match event {
        Event::FocusWindow if !policy.window_active || state == State::Reposition => {
            effects.window_keyboard = KeyboardMode::Exclusive
        }
        Event::FocusStrip if !policy.strip_active => {
            effects.strip_keyboard = KeyboardMode::Exclusive
        }
        Event::WindowFocused => effects.window_keyboard = KeyboardMode::OnDemand,
        Event::StripFocused => effects.strip_keyboard = KeyboardMode::OnDemand,
        _ => (),
    }
    (state, effects)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StripPhase {
    Settled,
    Revealing,
    Collapsing,
}

pub(super) struct Strip {
    monitor: RefCell<Option<gtk::gdk::Monitor>>,
    monitor_handlers: RefCell<Vec<(glib::Object, glib::SignalHandlerId)>>,
    monitor_list_handler: RefCell<Option<(gtk::gio::ListModel, glib::SignalHandlerId)>>,
    monitor_update: RefCell<Option<glib::SourceId>>,
    layout_update: RefCell<Option<glib::SourceId>>,
    layout_handlers: RefCell<[Vec<(glib::Object, glib::SignalHandlerId)>; 2]>,
    layout_size: Cell<(i32, i32)>,
    recovery_focus: RefCell<Option<(EdgeState, glib::WeakRef<gtk::Widget>)>>,
    search_popup_request: Cell<Option<bool>>,
    search_size_dirty: Cell<bool>,
    search_selection_pending: Cell<bool>,
    search_size: Cell<(i32, i32)>,
    search_placement: Cell<Option<(i32, i32, i32, i32)>>,
    remapping: Cell<bool>,
    monitor_layout_dirty: Cell<bool>,
    monitor_stopped: Cell<bool>,
    pub(super) window: gtk::ApplicationWindow,
    pub(super) root: gtk::Overlay,
    pub(super) strip_window: gtk::ApplicationWindow,
    pub(super) strip_root: gtk::Overlay,
    pub(super) strip_offset: Cell<Option<i32>>,
    pub(super) horizontal_tab_width: Cell<i32>,
    pub(super) reposition_error: gtk::Label,
    pub(super) strip_bounds: Cell<(i32, i32, i32, i32)>,
    pub(super) panel_bounds: Cell<(i32, i32, i32, i32)>,
    pub(super) stack: gtk::Stack,
    pub(super) marker: gtk::Box,
    pub(super) tabs: gtk::Box,
    pub(super) edge_search: gtk::SearchEntry,
    search_popover: gtk::Popover,
    search_panel: gtk::Box,
    pub(super) search_results: gtk::ListBox,
    search_scroll: gtk::ScrolledWindow,
    pub(super) search_ids: RefCell<Vec<NoteId>>,
    pub(super) search_create: gtk::Button,
    pub(super) pointer: gtk::DrawingArea,
    pub(super) pointer_origin: Cell<Option<i32>>,
    pub(super) docked: RefCell<Option<Rc<editor::Editor>>>,
    pub(super) docked_offset: Cell<Option<(ScreenEdge, i32)>>,
    delete_toast: gtk::Box,
    undo_delete: gtk::Button,
    toast_note: Cell<Option<NoteId>>,
    toast_bounds: Cell<(i32, i32, i32, i32)>,
    toast_timeout: RefCell<Option<glib::SourceId>>,
    toast_motion: motion::Motion,
    toast_generation: Cell<u64>,
    pub(super) cancel_reposition: gtk::Button,
    pub(super) done_reposition: gtk::Button,
    state: Cell<EdgeState>,
    generation: Cell<u64>,
    reveal_armed: Cell<bool>,
    hide_armed: Cell<bool>,
    pointer_inside_edge: Cell<bool>,
    preview_offset: Cell<f64>,
    strip_phase: Cell<StripPhase>,
    pending_motion: Cell<Option<EdgeState>>,
    deferred_hide: Cell<Option<u64>>,
    refresh_pending: Cell<bool>,
    marker_bounds: Cell<(i32, i32, i32, i32)>,
    tab_hover: RefCell<Vec<Rc<motion::Motion>>>,
    strip_motion: motion::Motion,
    panel_motion: motion::Motion,
    outgoing_panel: RefCell<Option<gtk::Widget>>,
    outgoing_origin: Cell<Option<i32>>,
    outgoing_session: RefCell<Option<Rc<crate::note_session::NoteSession>>>,
}

fn usable_monitor(monitor: &gtk::gdk::Monitor) -> bool {
    let geometry = monitor.geometry();
    monitor.is_valid() && geometry.width() > 0 && geometry.height() > 0
}

impl Drop for Strip {
    fn drop(&mut self) {
        self.disconnect_monitors();
    }
}

impl Strip {
    pub(super) fn output_suspended(&self) -> bool {
        self.remapping.get() || !self.monitor.borrow().as_ref().is_some_and(usable_monitor)
    }

    fn cancel_search_popup(&self) {
        self.search_popup_request.set(None);
    }

    fn disconnect_layout(&self, index: usize) {
        for (object, handler) in self.layout_handlers.borrow_mut()[index].drain(..) {
            object.disconnect(handler);
        }
    }

    pub(super) fn disconnect_monitors(&self) {
        self.monitor_stopped.set(true);
        self.cancel_search_popup();
        if let Some(source) = self.layout_update.borrow_mut().take() {
            source.remove();
        }
        for index in 0..2 {
            self.disconnect_layout(index);
        }
        self.recovery_focus.borrow_mut().take();
        if let Some(source) = self.monitor_update.borrow_mut().take() {
            source.remove();
        }
        for (object, handler) in self.monitor_handlers.borrow_mut().drain(..) {
            object.disconnect(handler);
        }
        if let Some((list, handler)) = self.monitor_list_handler.borrow_mut().take() {
            list.disconnect(handler);
        }
    }

    pub(super) fn new(application: &gtk::Application) -> Self {
        let (_, initial) = transition(
            EdgeState::Dormant,
            EdgeEvent::Refresh,
            Policy {
                transient_visible: true,
                hide_in_fullscreen: false,
                window_active: false,
                strip_active: false,
            },
        );
        let window = gtk::ApplicationWindow::builder()
            .application(application)
            .decorated(false)
            .resizable(false)
            .default_width(WIDTH)
            .default_height(HEIGHT)
            .build();
        window.add_css_class("edge-window");
        window.init_layer_shell();
        window.set_namespace(Some("stickies"));
        // Position against the screen bounds used by our clamps, without reserving space.
        window.set_exclusive_zone(-1);
        window.set_keyboard_mode(initial.window_keyboard);
        let strip_window = gtk::ApplicationWindow::builder()
            .application(application)
            .decorated(false)
            .resizable(false)
            .build();
        strip_window.add_css_class("edge-window");
        strip_window.init_layer_shell();
        strip_window.set_namespace(Some("stickies-strip"));
        strip_window.set_exclusive_zone(-1);
        strip_window.set_keyboard_mode(initial.strip_keyboard);
        let monitor = gtk::prelude::WidgetExt::display(&window)
            .monitors()
            .iter::<gtk::gdk::Monitor>()
            .filter_map(Result::ok)
            .find(usable_monitor);
        for window in [&window, &strip_window] {
            window.set_monitor(monitor.as_ref());
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Right, false);
            window.set_anchor(Edge::Bottom, false);
        }
        let strip_root = gtk::Overlay::new();
        strip_root.set_child(Some(&gtk::Box::new(gtk::Orientation::Vertical, 0)));
        let root = gtk::Overlay::new();
        // Keep a transparent render node even when the last panel is hidden.
        // An empty snapshot can leave the previous buffer visible on the compositor.
        let hit_area = gtk::DrawingArea::new();
        hit_area.set_draw_func(|_, cr, _, _| {
            cr.set_operator(gtk::cairo::Operator::Clear);
            let _ = cr.paint();
        });
        hit_area.add_css_class("hit-area");
        root.set_child(Some(&hit_area));
        let stack = gtk::Stack::new();
        stack.set_hhomogeneous(false);
        stack.set_vhomogeneous(false);
        // Different child sizes must switch together with the Layer Shell allocation.
        stack.set_transition_type(gtk::StackTransitionType::None);
        stack.set_transition_duration(0);
        stack.set_visible(initial.panel.is_some());
        let s = crate::edge_surface::scale_px;
        let tabs = gtk::Box::new(gtk::Orientation::Vertical, s(10));
        tabs.set_visible(initial.tabs_visible);
        root.add_overlay(&stack);
        let search_panel = gtk::Box::new(gtk::Orientation::Vertical, s(6));
        search_panel.add_css_class("edge-search");
        search_panel.set_width_request(s(224));
        let edge_search = gtk::SearchEntry::new();
        edge_search.set_placeholder_text(Some("Search active notes"));
        edge_search.set_hexpand(true);
        search_panel.append(&edge_search);
        let search_results = gtk::ListBox::new();
        search_results.set_activate_on_single_click(true);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vscrollbar_policy(gtk::PolicyType::External)
            .propagate_natural_height(false)
            .min_content_height(s(128))
            .max_content_height(s(128))
            .child(&search_results)
            .build();
        // Reserve the full results area for every query. The create action
        // occupies that same space rather than adding another row below it.
        let search_content = gtk::Overlay::new();
        search_content.set_child(Some(&scroll));
        search_panel.append(&search_content);
        let search_create = gtk::Button::with_label("No active notes");
        search_create.add_css_class("edge-create-result");
        search_create.set_valign(gtk::Align::Start);
        search_content.add_overlay(&search_create);
        // Keep the existing panel geometry and motion clock. The native popup
        // supplies outside-click dismissal without treating pointer focus as a click.
        let search_anchor = gtk::Box::new(gtk::Orientation::Vertical, 0);
        stack.add_named(&search_anchor, Some("search"));
        let search_popover = gtk::Popover::builder()
            .has_arrow(false)
            .autohide(true)
            .position(gtk::PositionType::Bottom)
            .halign(gtk::Align::Center)
            .valign(gtk::Align::Start)
            .child(&search_panel)
            .build();
        search_popover.add_css_class("edge-search-popover");
        root.add_overlay(&search_popover);
        stack
            .bind_property("opacity", &search_popover, "opacity")
            .sync_create()
            .build();
        let reposition_panel = gtk::Box::new(gtk::Orientation::Vertical, s(12));
        reposition_panel.add_css_class("edge-search");
        reposition_panel.set_width_request(s(240));
        let instruction = gtk::Label::new(Some("Drag the strip along the edge."));
        instruction.set_wrap(true);
        reposition_panel.append(&instruction);
        let reposition_error = gtk::Label::new(None);
        reposition_error.set_wrap(true);
        reposition_error.set_max_width_chars(28);
        reposition_error.add_css_class("save-error");
        reposition_error.set_visible(false);
        reposition_panel.append(&reposition_error);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, s(8));
        let cancel_reposition = gtk::Button::with_label("Cancel");
        let done_reposition = gtk::Button::with_label("Done");
        for button in [&cancel_reposition, &done_reposition] {
            button.set_hexpand(true);
            actions.append(button);
        }
        reposition_panel.append(&actions);
        stack.add_named(&reposition_panel, Some("reposition"));
        let marker = gtk::Box::new(gtk::Orientation::Vertical, s(6));
        marker.add_css_class("edge-marker");

        marker.set_can_target(false);
        strip_root.add_overlay(&marker);
        strip_root.add_overlay(&tabs);
        let delete_toast = gtk::Box::new(gtk::Orientation::Horizontal, s(10));
        delete_toast.add_css_class("edge-delete-toast");
        delete_toast.set_halign(gtk::Align::Start);
        delete_toast.set_valign(gtk::Align::Start);
        delete_toast.append(&gtk::Label::new(Some("Note deleted")));
        let undo_delete = gtk::Button::with_label("Undo");
        delete_toast.append(&undo_delete);
        delete_toast.set_visible(false);
        delete_toast.set_opacity(0.);
        strip_root.add_overlay(&delete_toast);
        let pointer = gtk::DrawingArea::new();
        pointer.set_can_target(false);
        pointer.set_hexpand(true);
        pointer.set_vexpand(true);
        pointer.set_visible(initial.pointer_visible);
        root.add_overlay(&pointer);
        // CSS ancestry must exist before the first layout measurement.
        strip_window.set_child(Some(&strip_root));
        window.set_child(Some(&root));
        Self {
            monitor: RefCell::new(monitor),
            monitor_handlers: RefCell::new(Vec::new()),
            monitor_list_handler: RefCell::new(None),
            monitor_update: RefCell::new(None),
            layout_update: RefCell::new(None),
            layout_handlers: RefCell::new([Vec::new(), Vec::new()]),
            layout_size: Cell::new((0, 0)),
            recovery_focus: RefCell::new(None),
            search_popup_request: Cell::new(None),
            search_size_dirty: Cell::new(true),
            search_selection_pending: Cell::new(false),
            search_size: Cell::new((s(224), 1)),
            search_placement: Cell::new(None),
            remapping: Cell::new(false),
            monitor_layout_dirty: Cell::new(false),
            monitor_stopped: Cell::new(false),
            window,
            root,
            strip_window,
            strip_root,
            strip_offset: Cell::new(None),
            horizontal_tab_width: Cell::new(0),
            reposition_error,
            strip_bounds: Cell::new((0, 0, 0, 0)),
            panel_bounds: Cell::new((0, 0, 0, 0)),
            stack,
            marker,
            tabs,
            edge_search,
            search_popover,
            search_panel,
            search_results,
            search_scroll: scroll,
            search_ids: RefCell::new(Vec::new()),
            search_create,
            pointer,
            pointer_origin: Cell::new(None),
            docked: RefCell::new(None),
            docked_offset: Cell::new(None),
            delete_toast,
            undo_delete,
            toast_note: Cell::new(None),
            toast_bounds: Cell::new((0, 0, 0, 0)),
            toast_timeout: RefCell::new(None),
            toast_motion: motion::Motion::new(0.),
            toast_generation: Cell::new(0),
            cancel_reposition,
            done_reposition,
            state: Cell::new(EdgeState::Dormant),
            generation: Cell::new(0),
            reveal_armed: Cell::new(false),
            hide_armed: Cell::new(false),
            pointer_inside_edge: Cell::new(false),
            preview_offset: Cell::new(0.),
            strip_phase: Cell::new(StripPhase::Settled),
            pending_motion: Cell::new(None),
            deferred_hide: Cell::new(None),
            refresh_pending: Cell::new(false),
            marker_bounds: Cell::new((0, 0, 0, 0)),
            tab_hover: RefCell::new(Vec::new()),
            strip_motion: motion::Motion::new(0.),
            panel_motion: motion::Motion::new(0.),
            outgoing_panel: RefCell::new(None),
            outgoing_origin: Cell::new(None),
            outgoing_session: RefCell::new(None),
        }
    }
    pub(super) fn state(&self) -> EdgeState {
        self.state.get()
    }
    pub(super) fn reposition_offset(&self) -> Option<f64> {
        self.state()
            .is_repositioning()
            .then(|| self.preview_offset.get())
    }
    pub(super) fn cancel_timers(&self) -> u64 {
        self.reveal_armed.set(false);
        self.hide_armed.set(false);
        let generation = self.generation.get().wrapping_add(1);
        self.generation.set(generation);
        generation
    }
    fn policy(&self, config: &Config, dnd: bool) -> Policy {
        Policy {
            transient_visible: !(config.hide_in_dnd && dnd),
            hide_in_fullscreen: config.hide_in_fullscreen,
            window_active: self.window.is_active(),
            strip_active: self.strip_window.is_active(),
        }
    }
    fn effects(&self, config: &Config, dnd: bool) -> Effects {
        transition(self.state(), EdgeEvent::Refresh, self.policy(config, dnd)).1
    }
    fn apply(&self, effects: Effects) {
        let moving = self.strip_phase.get() != StripPhase::Settled;
        self.marker.set_visible(effects.marker_visible || moving);
        self.tabs.set_visible(effects.tabs_visible || moving);
        if let Some(panel) = effects.panel {
            self.stack.set_visible_child_name(panel);
        }
        let outgoing = self.outgoing_panel.borrow().is_some();
        let retiring_search =
            self.outgoing_panel.borrow().as_ref() == self.stack.child_by_name("search").as_ref();
        if effects.panel != Some("search") && !retiring_search {
            self.search_popover.popdown();
        }
        self.stack.set_visible(effects.panel.is_some() || outgoing);
        self.pointer
            .set_visible(effects.pointer_visible || outgoing);
        self.tabs.set_sensitive(effects.panel != Some("reposition"));
        self.strip_root
            .set_cursor_from_name((effects.panel == Some("reposition")).then_some("grab"));
        self.strip_window
            .set_layer(if self.toast_note.get().is_some() {
                Layer::Overlay
            } else {
                effects.strip_layer
            });
        self.window.set_layer(effects.window_layer);
        self.strip_window.set_keyboard_mode(effects.strip_keyboard);
        self.window.set_keyboard_mode(effects.window_keyboard);
    }
}

impl Ui {
    // Use the sizes that build the strip, before hidden children have their first
    // allocation. Native button padding must not become the reveal's geometry.
    fn strip_size(&self) -> (i32, i32) {
        let horizontal = matches!(
            self.config.borrow().edge,
            ScreenEdge::Top | ScreenEdge::Bottom
        );
        let fixed = self.edge.tabs.first_child().and_downcast::<gtk::Fixed>();
        let (fw, fh) = fixed.as_ref().map_or((0, 0), |fixed| {
            (fixed.width_request(), fixed.height_request())
        });
        let mut widths = Vec::new();
        let mut heights = Vec::new();
        let group = self.edge.tabs.last_child();
        let mut child = group.as_ref().and_then(|group| group.first_child());
        while let Some(button) = child {
            widths.push(button.width_request());
            heights.push(button.height_request());
            child = button.next_sibling();
        }
        let gap = group
            .and_downcast::<gtk::Box>()
            .map_or(0, |group| group.spacing())
            * (widths.len() as i32 - 1).max(0);
        let group_gap = if fixed.is_some() {
            self.edge.tabs.spacing()
        } else {
            0
        };
        if horizontal {
            (
                fw + group_gap + widths.iter().sum::<i32>() + gap,
                fh.max(heights.into_iter().max().unwrap_or(0)),
            )
        } else {
            (
                fw.max(widths.into_iter().max().unwrap_or(0)),
                fh + group_gap + heights.iter().sum::<i32>() + gap,
            )
        }
    }
    #[cfg(test)]
    pub(super) fn edge_motion_running(&self) -> bool {
        self.edge.strip_phase.get() != StripPhase::Settled || self.edge.panel_motion.is_running()
    }
    fn motion_enabled(&self) -> bool {
        !self.config.borrow().reduced_motion
            && gtk::Settings::default().is_some_and(|settings| settings.is_gtk_enable_animations())
    }
    fn note_switching(&self) -> bool {
        self.edge.stack.child_by_name("outgoing-editor").is_some()
    }
    pub(super) fn begin_note_switch(&self, outgoing: &gtk::Widget) {
        self.finish_panel_motion();
        self.edge.stack.page(outgoing).set_name("outgoing-editor");
        self.edge.outgoing_panel.replace(Some(outgoing.clone()));
        // The old view has passed its save/release guards. It remains only for
        // the stack's crossfade and must not accept input while retiring.
        outgoing.set_can_target(false);
    }
    pub(super) fn finish_note_switch(&self) {
        if self.note_switching() {
            self.finish_panel_motion();
        }
    }
    fn prepare_motion(self: &Rc<Self>, previous: EdgeState, state: EdgeState) {
        let enabled = self.motion_enabled();
        match (previous, state) {
            (EdgeState::Dormant, EdgeState::Tabs) if enabled => {
                self.edge.strip_phase.set(StripPhase::Revealing)
            }
            (EdgeState::Tabs, EdgeState::Dormant) if enabled => {
                self.edge.strip_phase.set(StripPhase::Collapsing)
            }
            _ => self.finish_strip_motion(),
        }
        if matches!(
            (previous, state),
            (EdgeState::Docked(_), EdgeState::Docked(_))
        ) && self.note_switching()
        {
            // Keep the sheet opaque and stationary while GtkStack blends its
            // two children. The common panel clock fades the new arrow and
            // releases the outgoing child when the transition finishes.
            self.edge.panel_motion.jump(0.);
            self.edge.stack.set_opacity(1.);
            self.edge.pointer.set_opacity(if enabled { 0. } else { 1. });
            self.edge.stack.set_transition_duration(motion::PANEL_MS);
            self.edge.stack.set_transition_type(if enabled {
                gtk::StackTransitionType::Crossfade
            } else {
                gtk::StackTransitionType::None
            });
        } else if matches!(previous, EdgeState::Docked(_) | EdgeState::Search)
            && state == EdgeState::Tabs
        {
            self.edge.panel_motion.cancel();
            self.edge
                .outgoing_origin
                .set(self.edge.pointer_origin.get());
            self.edge
                .outgoing_panel
                .replace(self.edge.stack.visible_child());
            self.edge.outgoing_session.replace(
                previous
                    .docked_id()
                    .and_then(|id| self.sessions.borrow().get(&id).and_then(Weak::upgrade)),
            );
            if !enabled {
                self.finish_panel_motion();
            }
        } else if previous == EdgeState::Tabs
            && state == EdgeState::Search
            && enabled
            && self
                .edge
                .outgoing_panel
                .borrow()
                .as_ref()
                .is_some_and(|outgoing| {
                    self.edge.stack.child_by_name("search").as_ref() == Some(outgoing)
                })
        {
            // Reopening the same child reverses its fade from the painted value.
            self.edge.panel_motion.cancel();
            self.edge.outgoing_panel.borrow_mut().take();
            self.edge.outgoing_session.borrow_mut().take();
            self.edge.outgoing_origin.set(None);
        } else {
            self.finish_panel_motion();
            if matches!(state, EdgeState::Docked(_) | EdgeState::Search) && enabled {
                self.edge.panel_motion.jump(0.);
                self.edge.stack.set_opacity(0.);
                self.edge.pointer.set_opacity(0.);
            }
        }
    }
    fn animate_transition(self: &Rc<Self>, state: EdgeState) {
        if self.edge.strip_phase.get() != StripPhase::Settled {
            self.animate_strip();
        }
        if matches!(state, EdgeState::Docked(_) | EdgeState::Search)
            || self.edge.outgoing_panel.borrow().is_some()
        {
            let to = if matches!(state, EdgeState::Docked(_) | EdgeState::Search) {
                1.
            } else {
                0.
            };
            let update = Rc::downgrade(self);
            let done = Rc::downgrade(self);
            self.edge.panel_motion.animate(
                &self.edge.window,
                to,
                motion::PANEL_MS,
                self.motion_enabled(),
                move |value| {
                    if let Some(ui) = update.upgrade() {
                        if !ui.note_switching() {
                            ui.edge.stack.set_opacity(value);
                        }
                        ui.edge.pointer.set_opacity(value);
                    }
                },
                move || {
                    if let Some(ui) = done.upgrade() {
                        ui.finish_panel_motion();
                    }
                },
            );
        }
    }
    fn paint_tab(
        fixed: &gtk::Fixed,
        tab: &gtk::Widget,
        config: &Config,
        index: usize,
        progress: f64,
        hovers: &[Rc<motion::Motion>],
    ) {
        let gap = tab_gap(config);
        let edge = config.edge;
        let length = if matches!(edge, ScreenEdge::Top | ScreenEdge::Bottom) {
            tab.width_request()
        } else {
            tab.height_request()
        };
        // Derive spacing from painted sizes, never from the raw pointer. The
        // stack keeps its overlap while growing around its fixed center.
        let spread: f64 = hovers
            .iter()
            .enumerate()
            .map(|(i, hover)| {
                let direction = match i.cmp(&index) {
                    std::cmp::Ordering::Less => 1.,
                    std::cmp::Ordering::Equal => 0.,
                    std::cmp::Ordering::Greater => -1.,
                };
                direction * hover.value() * TAB_MAGNIFICATION * f64::from(length) / 2.
            })
            .sum();
        let stacked = config.tab_layout == config::TabLayout::Stacked;
        let inset = TAB_INSET as f32 + if stacked { spread as f32 } else { 0. };
        let distance = (24. * (1. - progress)) as f32;
        let pull = if stacked {
            0.
        } else {
            6. * hovers.get(index).map_or(0., |hover| hover.value()) as f32
        };
        let (x, y) = match edge {
            ScreenEdge::Left => (
                -distance + pull - if stacked { 0. } else { 6. },
                inset + index as f32 * (tab.height_request() + gap) as f32,
            ),
            ScreenEdge::Right => (
                distance - pull,
                inset + index as f32 * (tab.height_request() + gap) as f32,
            ),
            ScreenEdge::Top => (
                inset + index as f32 * (tab.width_request() + gap) as f32,
                -distance + pull - if stacked { 0. } else { 6. },
            ),
            ScreenEdge::Bottom => (
                inset + index as f32 * (tab.width_request() + gap) as f32,
                distance - pull,
            ),
        };
        let (width, height) = (tab.width_request() as f32, tab.height_request() as f32);
        let (cx, cy) = match edge {
            ScreenEdge::Left => (0., height / 2.),
            ScreenEdge::Right => (width, height / 2.),
            ScreenEdge::Top => (width / 2., 0.),
            ScreenEdge::Bottom => (width / 2., height),
        };
        let hover = hovers.get(index).map_or(0., |hover| hover.value());
        let scale = if stacked {
            (1. + TAB_MAGNIFICATION * hover) as f32
        } else {
            1.
        };
        let transform = gtk::gsk::Transform::new()
            .translate(&gtk::graphene::Point::new(x, y))
            .translate(&gtk::graphene::Point::new(cx, cy))
            .scale(scale, scale)
            .translate(&gtk::graphene::Point::new(-cx, -cy));
        fixed.set_child_transform(tab, Some(&transform));
        tab.set_opacity(progress);
    }
    fn animate_tab_hover(self: &Rc<Self>, index: usize, target: f64) {
        let Some(hover) = self.edge.tab_hover.borrow().get(index).cloned() else {
            return;
        };
        let enabled = self.motion_enabled() && !self.edge.state().is_repositioning();
        let weak = Rc::downgrade(self);
        hover.follow(&self.edge.strip_window, target, enabled, move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.paint_hover_tabs();
            }
        });
    }
    fn paint_hover_tabs(&self) {
        let Some(fixed) = self.edge.tabs.first_child().and_downcast::<gtk::Fixed>() else {
            return;
        };
        let hovers = self.edge.tab_hover.borrow();
        let layout = fixed.downcast_ref::<TabLayout>().unwrap().imp();
        if !layout.pointer.get().is_finite() && hovers.iter().all(|hover| hover.value() == 0.) {
            layout.front.set(None);
            fixed.queue_draw();
        }
        let mut child = fixed.first_child();
        for index in 0..hovers.len() {
            let Some(tab) = child else { break };
            child = tab.next_sibling();
            Self::paint_tab(
                &fixed,
                &tab,
                &self.config.borrow(),
                index,
                self.edge.strip_motion.value(),
                &hovers,
            );
        }
    }
    fn finish_hover_motion(self: &Rc<Self>) {
        if let Some(fixed) = self.edge.tabs.first_child().and_downcast::<TabLayout>() {
            let position =
                if self.edge.state().is_revealed() && !self.edge.state().is_repositioning() {
                    fixed.imp().pointer.get()
                } else {
                    fixed.imp().pointer.set(f64::NAN);
                    f64::NAN
                };
            for (index, hover) in self.edge.tab_hover.borrow().iter().enumerate() {
                hover.jump(tab_hover_target(
                    index,
                    position,
                    self.config.borrow().tab_layout,
                ));
            }
            self.paint_hover_tabs();
        }
    }
    fn paint_strip(&self, progress: f64) {
        self.paint_hover_tabs();
        self.edge.marker.set_opacity(1. - progress);
        if let Some(utilities) = self.edge.tabs.last_child() {
            utilities.set_opacity(progress);
        }
    }
    fn animate_strip(self: &Rc<Self>) {
        let revealing = self.edge.strip_phase.get() == StripPhase::Revealing;
        let update = Rc::downgrade(self);
        let done = Rc::downgrade(self);
        self.edge.strip_motion.animate(
            &self.edge.strip_window,
            if revealing { 1. } else { 0. },
            if revealing {
                motion::REVEAL_MS
            } else {
                motion::COLLAPSE_MS
            },
            self.motion_enabled(),
            move |progress| {
                if let Some(ui) = update.upgrade() {
                    ui.paint_strip(progress);
                }
            },
            move || {
                if let Some(ui) = done.upgrade() {
                    ui.finish_strip_motion();
                }
            },
        );
    }
    fn finish_strip_motion(self: &Rc<Self>) {
        let revealed = self.edge.state().is_revealed();
        let progress = if revealed { 1. } else { 0. };
        self.edge.strip_phase.set(StripPhase::Settled);
        if !revealed || self.edge.state().is_repositioning() {
            for hover in self.edge.tab_hover.borrow().iter() {
                hover.jump(0.);
            }
            if let Some(fixed) = self.edge.tabs.first_child().and_downcast::<TabLayout>() {
                fixed.imp().pointer.set(f64::NAN);
                fixed.imp().front.set(None);
                fixed.queue_draw();
            }
        }
        self.edge.strip_motion.jump(progress);
        self.paint_strip(progress);
        self.edge.marker.set_visible(!revealed);
        self.edge.tabs.set_visible(revealed);
        if !revealed {
            self.edge.strip_bounds.set(self.edge.marker_bounds.get());
            self.edge.strip_offset.set(None);
        }
        self.set_input();
        if self.edge.refresh_pending.get() {
            self.refresh_tabs();
        }
        if let Some(generation) = self.edge.deferred_hide.take()
            && self.edge.generation.get() == generation
            && revealed
        {
            self.arm_hide(generation);
        }
    }
    pub(super) fn finish_panel_motion(&self) {
        self.edge.panel_motion.cancel();
        let outgoing = self.edge.outgoing_panel.borrow_mut().take();
        self.edge
            .stack
            .set_transition_type(gtk::StackTransitionType::None);
        if let Some(outgoing) = outgoing
            && (self.edge.stack.child_by_name("editor").as_ref() == Some(&outgoing)
                || self.edge.stack.child_by_name("outgoing-editor").as_ref() == Some(&outgoing))
        {
            self.edge.stack.remove(&outgoing);
        }
        self.edge.outgoing_session.borrow_mut().take();
        self.edge.outgoing_origin.set(None);
        let effects = self.edge.effects(&self.config.borrow(), self.dnd.get());
        let opacity = if effects.panel.is_some() { 1. } else { 0. };
        self.edge.panel_motion.jump(opacity);
        self.edge.stack.set_opacity(opacity);
        self.edge.pointer.set_opacity(opacity);
        self.edge.stack.set_visible(effects.panel.is_some());
        if !self.edge.state().is_searching() {
            self.edge.search_popover.popdown();
        }
        self.edge.pointer.set_visible(effects.pointer_visible);
        self.edge.root.queue_draw();
    }
    fn clear_tab_hover(self: &Rc<Self>) {
        if let Some(fixed) = self.edge.tabs.first_child().and_downcast::<TabLayout>()
            && fixed.imp().pointer.get().is_finite()
        {
            fixed.imp().pointer.set(f64::NAN);
            fixed.queue_draw();
            let count = self.edge.tab_hover.borrow().len();
            for i in 0..count {
                self.animate_tab_hover(i, 0.);
            }
        }
    }
    pub(super) fn finish_edge_motion(self: &Rc<Self>) {
        self.edge.pending_motion.set(None);
        self.finish_strip_motion();
        self.finish_hover_motion();
        self.finish_panel_motion();
    }
    pub(super) fn configure_docked_editor(self: &Rc<Self>, editor: &Rc<editor::Editor>) {
        let s = crate::edge_surface::scale_px;
        editor.set_docked_size(s(editor::WIDTH), s(editor::HEIGHT));
        #[cfg(target_os = "linux")]
        {
            let drag = gtk::GestureDrag::new();
            drag.set_button(1);
            drag.set_propagation_phase(gtk::PropagationPhase::Capture);
            let start = Rc::new(Cell::new((s(editor::WIDTH), s(editor::HEIGHT))));
            let pointer_start = Rc::new(Cell::new((0., 0.)));
            let initial = start.clone();
            let initial_pointer = pointer_start.clone();
            let weak = Rc::downgrade(self);
            let weak_editor = Rc::downgrade(editor);
            drag.connect_drag_begin(move |gesture, _, _| {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                if let Some(editor) = weak_editor.upgrade()
                    && let Some(ui) = weak.upgrade()
                    && let Some(pointer) = ui.docked_pointer(gesture)
                {
                    initial.set(editor.docked_size());
                    initial_pointer.set(pointer);
                    editor.activity();
                }
            });
            let weak = Rc::downgrade(self);
            let weak_editor = Rc::downgrade(editor);
            drag.connect_drag_update(move |gesture, _, _| {
                if let Some(ui) = weak.upgrade()
                    && let Some(editor) = weak_editor.upgrade()
                    && let Some((x, y)) = ui.docked_pointer(gesture)
                {
                    let (start_x, start_y) = pointer_start.get();
                    let (dx, dy) = (x - start_x, y - start_y);
                    let edge = ui.config.borrow().edge;
                    let (width, height) = start.get();
                    let dx = if edge == ScreenEdge::Right { -dx } else { dx };
                    let dy = if edge == ScreenEdge::Bottom { -dy } else { dy };
                    let (max_width, max_height) = ui.note_limits();
                    editor.set_docked_size(
                        (width + dx as i32)
                            .clamp(s(editor::MIN_WIDTH), max_width.max(s(editor::MIN_WIDTH))),
                        (height + dy as i32)
                            .clamp(s(editor::MIN_HEIGHT), max_height.max(s(editor::MIN_HEIGHT))),
                    );
                    ui.place_edge();
                }
            });
            editor.resize.add_controller(drag);
        }
        #[cfg(target_os = "windows")]
        {
            let begin_editor = Rc::downgrade(editor);
            let update_ui = Rc::downgrade(self);
            let update_editor = Rc::downgrade(editor);
            crate::edge_surface::connect_resize_drag(
                &editor.resize,
                &self.edge.window,
                move || {
                    if let Some(editor) = begin_editor.upgrade() {
                        editor.activity();
                        editor.docked_size()
                    } else {
                        (s(editor::WIDTH), s(editor::HEIGHT))
                    }
                },
                move |width, height, dx, dy| {
                    if let (Some(ui), Some(editor)) = (update_ui.upgrade(), update_editor.upgrade())
                    {
                        let edge = ui.config.borrow().edge;
                        let dx = if edge == ScreenEdge::Right { -dx } else { dx };
                        let dy = if edge == ScreenEdge::Bottom { -dy } else { dy };
                        let (max_width, max_height) = ui.note_limits();
                        editor.set_docked_size(
                            (width + dx)
                                .clamp(s(editor::MIN_WIDTH), max_width.max(s(editor::MIN_WIDTH))),
                            (height + dy).clamp(
                                s(editor::MIN_HEIGHT),
                                max_height.max(s(editor::MIN_HEIGHT)),
                            ),
                        );
                        editor.activity();
                        ui.place_edge();
                    }
                },
            );
        }
        let weak = Rc::downgrade(self);
        editor.connect_size_changed(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_input();
            }
        });
    }
    fn watch_selected_monitor(self: &Rc<Self>) {
        for (object, handler) in self.edge.monitor_handlers.borrow_mut().drain(..) {
            object.disconnect(handler);
        }
        let Some(monitor) = self.edge.monitor.borrow().clone() else {
            return;
        };
        for property in ["geometry", "scale-factor", "scale"] {
            // Fractional scale is a property on GTK 4.14+. Keep the GTK 4.10 floor.
            if monitor.find_property(property).is_none() {
                continue;
            }
            let weak = Rc::downgrade(self);
            let handler = monitor.connect_notify_local(Some(property), move |_, _| {
                if let Some(ui) = weak.upgrade() {
                    ui.queue_monitor_update(true);
                }
            });
            self.edge
                .monitor_handlers
                .borrow_mut()
                .push((monitor.clone().upcast(), handler));
        }
        let weak = Rc::downgrade(self);
        let handler = monitor.connect_invalidate(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.queue_monitor_update(false);
            }
        });
        self.edge
            .monitor_handlers
            .borrow_mut()
            .push((monitor.upcast(), handler));
    }

    fn queue_monitor_update(self: &Rc<Self>, layout_changed: bool) {
        if self.edge.monitor_stopped.get() {
            return;
        }
        self.edge
            .monitor_layout_dirty
            .set(self.edge.monitor_layout_dirty.get() || layout_changed);
        if self.edge.monitor_update.borrow().is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        self.edge
            .monitor_update
            .replace(Some(glib::idle_add_local_once(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.edge.monitor_update.borrow_mut().take();
                    ui.recover_monitor();
                }
            })));
    }

    fn recover_monitor(self: &Rc<Self>) {
        let monitors = gtk::prelude::WidgetExt::display(&self.edge.window).monitors();
        let available: Vec<_> = monitors
            .iter::<gtk::gdk::Monitor>()
            .filter_map(Result::ok)
            .filter(usable_monitor)
            .collect();
        let previous = self.edge.monitor.borrow().clone();
        let selected = previous
            .as_ref()
            .filter(|monitor| available.contains(monitor))
            .cloned()
            .or_else(|| available.first().cloned());
        let changed = previous != selected;
        let layout_changed = self.edge.monitor_layout_dirty.replace(false);
        if !changed && !layout_changed && !self.edge.remapping.get() {
            return;
        }

        if changed || selected.is_none() || self.edge.remapping.get() {
            self.edge.cancel_search_popup();
        }
        self.edge.cancel_timers();
        self.finish_edge_motion();
        // Keep widget focus locally. Never issue an exclusive keyboard request
        // merely because an output changed, even if another app took focus meanwhile.
        if !self.edge.remapping.get() && changed && self.edge.window.is_active() {
            self.edge.recovery_focus.replace(
                gtk::prelude::GtkWindowExt::focus(&self.edge.window)
                    .map(|focus| (self.edge.state(), focus.downgrade())),
            );
        }
        if changed || selected.is_none() || self.edge.remapping.get() {
            self.edge.remapping.set(true);
            self.edge.search_popover.popdown();
            self.edge.window.set_visible(false);
            self.edge.strip_window.set_visible(false);
        }
        self.edge.monitor.replace(selected.clone());
        if changed {
            self.watch_selected_monitor();
            // Both are hidden while their outputs change. No compositor-chosen
            // output is used during a temporary gap in the monitor list.
            self.edge.window.set_monitor(selected.as_ref());
            self.edge.strip_window.set_monitor(selected.as_ref());
        }
        if selected.is_none() {
            if let Some(source) = self.edge.layout_update.borrow_mut().take() {
                source.remove();
            }
            return;
        }
        self.edge.search_size_dirty.set(true);
        self.refresh_tabs_inner(false);
        self.transition_edge(EdgeEvent::Visibility);
        self.queue_edge_layout();
    }

    // One idle pass per burst of content/configure/allocation events. A pass
    // never schedules itself; subsequent work requires an actual layout change.
    fn queue_edge_layout(self: &Rc<Self>) {
        if self.edge.monitor_stopped.get() || self.edge.layout_update.borrow().is_some() {
            return;
        }
        let weak = Rc::downgrade(self);
        self.edge
            .layout_update
            .replace(Some(glib::idle_add_local_once(move || {
                let Some(ui) = weak.upgrade() else { return };
                ui.edge.layout_update.borrow_mut().take();
                if !ui
                    .edge
                    .monitor
                    .borrow()
                    .as_ref()
                    .is_some_and(usable_monitor)
                {
                    return;
                }
                ui.apply_edge_classes(&ui.config.borrow());
                if ui.edge.state().is_searching() && ui.edge.search_size_dirty.replace(false) {
                    ui.measure_edge_search();
                }
                let size = ui.note_limits();
                if ui.edge.layout_size.replace(size) != size {
                    ui.refresh_tabs_inner(false);
                } else {
                    ui.place_edge();
                }
                if ui.edge.remapping.get()
                    && ui.edge.monitor_update.borrow().is_none()
                    && ui.edge_allocation_ready()
                {
                    ui.edge.remapping.set(false);
                    if let Some(editor) = ui.edge.docked.borrow().as_ref() {
                        editor.activity();
                    }
                    if ui.edge.state().is_searching() {
                        ui.edge.search_popup_request.set(Some(false));
                    }
                    let focus = ui.edge.recovery_focus.borrow_mut().take();
                    if let Some((state, focus)) = focus
                        && ui.edge.state() == state
                        && let Some(focus) = focus.upgrade()
                        && focus.is_ancestor(&ui.edge.root)
                    {
                        gtk::prelude::GtkWindowExt::set_focus(&ui.edge.window, Some(&focus));
                    }
                }
                if ui.edge.state().is_searching()
                    && !ui.edge.output_suspended()
                    && let Some(request_focus) = ui.edge.search_popup_request.get()
                    && ui.popup_edge_search(request_focus)
                {
                    ui.edge.search_popup_request.set(None);
                }
            })));
    }

    fn measure_edge_search(&self) {
        // The results area always reserves 128 logical pixels, including when
        // the create action is shown. Query changes do not change panel size.
        let (_, width, _, _) = self
            .edge
            .search_panel
            .measure(gtk::Orientation::Horizontal, -1);
        let (_, height, _, _) = self
            .edge
            .search_panel
            .measure(gtk::Orientation::Vertical, width);
        self.edge.search_size.set((width, height));
    }

    fn edge_allocation_ready(&self) -> bool {
        let (width, height) = self.note_limits();
        [
            (&self.edge.window, &self.edge.root),
            (&self.edge.strip_window, &self.edge.strip_root),
        ]
        .into_iter()
        .all(|(window, root)| {
            !window.is_visible()
                || (window.is_mapped()
                    && window.surface().is_some_and(|surface| {
                        // Oversized minimum layouts are allowed, but the native parent
                        // must agree with its widget allocation before a popup is mapped.
                        root.width() >= width
                            && root.height() >= height
                            && surface.width() == root.width()
                            && surface.height() == root.height()
                    }))
        })
    }

    pub(super) fn connect_edge(self: &Rc<Self>) {
        let monitors = gtk::prelude::WidgetExt::display(&self.edge.window).monitors();
        let weak = Rc::downgrade(self);
        let handler = monitors.connect_items_changed(move |_, _, _, _| {
            if let Some(ui) = weak.upgrade() {
                ui.queue_monitor_update(false);
            }
        });
        self.edge
            .monitor_list_handler
            .replace(Some((monitors, handler)));
        self.watch_selected_monitor();
        // Older gtk4-layer-shell releases forward compositor closure to GTK.
        // Output removal must not destroy the editor or either strip window.
        for window in [&self.edge.window, &self.edge.strip_window] {
            let weak = Rc::downgrade(self);
            window.connect_close_request(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.queue_monitor_update(false);
                }
                glib::Propagation::Stop
            });
        }

        let weak = Rc::downgrade(self);
        self.edge.undo_delete.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade()
                && let Some(id) = ui.edge.toast_note.get()
            {
                match ui.storage.undo_delete(id) {
                    Ok(()) => {
                        ui.finish_change(id, crate::manager::ManagerChange::Restore);
                        ui.hide_delete_toast();
                    }
                    Err(error) => ui.error(&format!("Could not undo deletion: {error}")),
                }
            }
        });

        if let Some(settings) = gtk::Settings::default() {
            let weak = Rc::downgrade(self);
            settings.connect_gtk_enable_animations_notify(move |settings| {
                if !settings.is_gtk_enable_animations()
                    && let Some(ui) = weak.upgrade()
                {
                    ui.finish_edge_motion();
                }
            });
        }
        let ui = self;
        let window = self.edge.window.clone();
        let strip_window = self.edge.strip_window.clone();
        let root = self.edge.root.clone();
        let strip_root = self.edge.strip_root.clone();
        for (button, save) in [
            (&ui.edge.cancel_reposition, false),
            (&ui.edge.done_reposition, true),
        ] {
            let weak = Rc::downgrade(ui);
            button.connect_clicked(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.finish_reposition(save).ok();
                }
            });
        }
        let weak = Rc::downgrade(ui);
        ui.edge.edge_search.connect_changed(move |_| {
            if let Some(ui) = weak.upgrade()
                && ui.edge.state().is_searching()
                && ui.edge.pending_motion.get().is_none()
                && ui.edge.panel_motion.is_running()
            {
                ui.finish_panel_motion();
            }
        });
        let weak = Rc::downgrade(ui);
        ui.edge.edge_search.connect_search_changed(move |_| {
            if let Some(ui) = weak.upgrade()
                && ui.edge.state().is_searching()
            {
                ui.refresh_edge_search();
            }
        });
        let weak = Rc::downgrade(ui);
        ui.edge.edge_search.connect_activate(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.activate_edge_search();
            }
        });
        let weak = Rc::downgrade(ui);
        ui.edge.search_popover.connect_closed(move |_| {
            if let Some(ui) = weak.upgrade()
                && ui.edge.state().is_searching()
                && !ui.edge.output_suspended()
            {
                ui.close_edge_search();
                ui.schedule_hide();
            }
        });
        let weak = Rc::downgrade(ui);
        ui.edge.search_create.connect_clicked(move |_| {
            if let Some(ui) = weak.upgrade() {
                ui.activate_edge_search();
            }
        });
        let weak = Rc::downgrade(ui);
        ui.edge.search_results.connect_row_activated(move |_, row| {
            if let Some(ui) = weak.upgrade() {
                let id = ui
                    .edge
                    .search_ids
                    .borrow()
                    .get(row.index() as usize)
                    .copied();
                if let Some(id) = id {
                    ui.open_docked(id);
                }
            }
        });
        for (window, event) in [
            (&ui.edge.window, EdgeEvent::WindowFocused),
            (&ui.edge.strip_window, EdgeEvent::StripFocused),
        ] {
            let weak = Rc::downgrade(ui);
            window.connect_is_active_notify(move |window| {
                // Request focus only for the explicit opening/closing action. Return
                // to on-demand immediately so another window can take focus normally.
                if window.is_active()
                    && window.keyboard_mode() == KeyboardMode::Exclusive
                    && let Some(ui) = weak.upgrade()
                {
                    ui.transition_edge(event);
                }
                // Native popup dismissal handles outside clicks. Pointer focus
                // changes and the strip/panel handoff do not dismiss search.
            });
        }
        let weak = Rc::downgrade(ui);
        ui.edge.pointer.set_draw_func(move |_, cr, _, _| {
            if let Some(ui) = weak.upgrade() {
                ui.draw_pointer(cr);
            }
        });
        ui.place_edge();
        ui.refresh_tabs();
        for (area, target_window) in [(&root, &window), (&strip_root, &strip_window)] {
            let motion = gtk::EventControllerMotion::new();
            {
                let weak = Rc::downgrade(ui);
                let enter_window = target_window.clone();
                motion.connect_enter(move |_, _, _| {
                    if cfg!(target_os = "windows")
                        && !crate::edge_surface::is_pointer_in_window(&enter_window)
                    {
                        return;
                    }
                    if let Some(ui) = weak.upgrade()
                        && ui.config.borrow().edge_hover_enabled
                        && ui.edge.toast_note.get().is_none()
                    {
                        ui.edge.pointer_inside_edge.set(true);
                        ui.schedule_reveal();
                    }
                });
            }
            {
                let weak = Rc::downgrade(ui);
                let leave_window = target_window.clone();
                motion.connect_leave(move |_| {
                    if cfg!(target_os = "windows")
                        && crate::edge_surface::is_pointer_in_window(&leave_window)
                    {
                        return;
                    }
                    if let Some(ui) = weak.upgrade() {
                        ui.schedule_hide();
                    }
                });
            }
            area.add_controller(motion);
        }
        {
            let weak = Rc::downgrade(ui);
            crate::edge_surface::connect_hover_fallback(&strip_window, move |in_strip| {
                let Some(ui) = weak.upgrade() else {
                    return;
                };
                let in_panel = crate::edge_surface::is_pointer_in_window(&ui.edge.window);
                if !in_strip {
                    ui.clear_tab_hover();
                }
                if in_strip || in_panel {
                    ui.edge.pointer_inside_edge.set(true);
                    if ui.edge.hide_armed.get() || ui.edge.deferred_hide.get().is_some() {
                        ui.edge.cancel_timers();
                        ui.edge.deferred_hide.set(None);
                    }
                    if in_strip
                        && ui.config.borrow().edge_hover_enabled
                        && ui.edge.toast_note.get().is_none()
                        && (ui.edge.state() == EdgeState::Dormant
                            || ui.edge.strip_phase.get() == StripPhase::Collapsing)
                        && !ui.edge.reveal_armed.get()
                    {
                        ui.schedule_reveal();
                    }
                } else {
                    if ui.edge.reveal_armed.get() {
                        ui.edge.cancel_timers();
                    }
                    if ui.edge.pointer_inside_edge.get() {
                        if ui.edge.state() == EdgeState::Tabs {
                            if ui.edge.strip_phase.get() != StripPhase::Collapsing
                                && !ui.edge.hide_armed.get()
                                && ui.edge.deferred_hide.get().is_none()
                            {
                                ui.schedule_hide();
                            }
                        } else {
                            ui.edge.pointer_inside_edge.set(false);
                        }
                    }
                }
            });
        }
        let click = gtk::GestureClick::new();
        {
            let weak = Rc::downgrade(ui);
            click.connect_released(move |_, _, _, _| {
                if let Some(ui) = weak.upgrade() {
                    ui.reveal();
                }
            });
        }
        strip_root.add_controller(click);
        let drag = gtk::GestureDrag::new();
        drag.set_propagation_phase(gtk::PropagationPhase::Capture);
        let initial = Rc::new(Cell::new(None));
        {
            let weak = Rc::downgrade(ui);
            let initial = initial.clone();
            drag.connect_drag_begin(move |gesture, _, _| {
                initial.set(None);
                if let Some(ui) = weak.upgrade() {
                    if !ui.edge.state().is_repositioning() && ui.edge.state().is_revealed() {
                        gesture.set_state(gtk::EventSequenceState::Denied);
                        return;
                    }
                    ui.edge.cancel_timers();
                    initial.set(Some(
                        ui.edge
                            .reposition_offset()
                            .unwrap_or(ui.config.borrow().marker_offset),
                    ));
                }
            });
        }
        {
            let weak = Rc::downgrade(ui);
            let initial = initial.clone();
            drag.connect_drag_update(move |gesture, x, y| {
                if let Some(ui) = weak.upgrade()
                    && let Some(start) = initial.get()
                    && ui.edge.state().is_repositioning()
                {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    ui.preview_reposition(start, x, y);
                }
            });
        }
        {
            let weak = Rc::downgrade(ui);
            drag.connect_drag_end(move |_, x, y| {
                let Some(start) = initial.take() else {
                    return;
                };
                if let Some(ui) = weak.upgrade()
                    && ui.config_valid
                {
                    if ui.edge.state().is_repositioning() {
                        ui.preview_reposition(start, x, y);
                    } else if ui.edge.state().docked_id().is_none() {
                        let mut config = ui.config.borrow().clone();
                        config.marker_offset = ui.dragged_offset(start, x, y);
                        if let Err(error) = ui.change_settings(config) {
                            ui.error(&format!("Could not save position: {error}"));
                        }
                    }
                }
            });
        }
        strip_root.add_controller(drag);
        for target in [
            window.upcast_ref::<gtk::Widget>(),
            strip_window.upcast_ref(),
            ui.edge.search_popover.upcast_ref(),
        ] {
            let keys = gtk::EventControllerKey::new();
            keys.set_propagation_phase(gtk::PropagationPhase::Capture);
            {
                let weak = Rc::downgrade(ui);
                keys.connect_key_pressed(move |_, key, _, _| {
                    if key == gtk::gdk::Key::Escape {
                        if let Some(ui) = weak.upgrade() {
                            if ui.edge.state().is_repositioning() {
                                ui.finish_reposition(false).ok();
                            } else if ui.edge.state().docked_id().is_some() {
                                let editor = ui.edge.docked.borrow().clone();
                                if let Some(editor) = editor {
                                    editor.escape();
                                }
                            } else if ui.edge.state().is_searching() {
                                ui.close_edge_search();
                                ui.transition_edge(EdgeEvent::FocusStrip);
                            } else {
                                ui.dormant();
                            }
                        }
                        glib::Propagation::Stop
                    } else if matches!(key, gtk::gdk::Key::Down | gtk::gdk::Key::Up) {
                        if let Some(ui) = weak.upgrade()
                            && ui.edge.state().is_searching()
                        {
                            let current = ui
                                .edge
                                .search_results
                                .selected_row()
                                .map_or(0, |row| row.index());
                            let count = ui.edge.search_ids.borrow().len() as i32;
                            let next = (current + if key == gtk::gdk::Key::Down { 1 } else { -1 })
                                .clamp(0, (count - 1).max(0));
                            ui.edge
                                .search_results
                                .select_row(ui.edge.search_results.row_at_index(next).as_ref());
                            ui.keep_search_selection_visible();
                            return glib::Propagation::Stop;
                        }
                        glib::Propagation::Proceed
                    } else {
                        glib::Propagation::Proceed
                    }
                });
            }
            target.add_controller(keys);
        }
        for (index, window) in [&window, &strip_window].into_iter().enumerate() {
            let weak = Rc::downgrade(ui);
            window.connect_map(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.queue_edge_layout();
                }
            });
            let weak = Rc::downgrade(ui);
            window.connect_realize(move |window| {
                let Some(ui) = weak.upgrade() else { return };
                ui.edge.disconnect_layout(index);
                if let Some(surface) = window.surface() {
                    let last_size = Cell::new((surface.width(), surface.height()));
                    let weak = weak.clone();
                    let handler = surface.connect_layout(move |_, width, height| {
                        if last_size.replace((width, height)) != (width, height)
                            && let Some(ui) = weak.upgrade()
                        {
                            ui.queue_edge_layout();
                        }
                    });
                    ui.edge.layout_handlers.borrow_mut()[index].push((surface.upcast(), handler));
                }
                if let Some(clock) = window.frame_clock() {
                    let last_size = Cell::new((0, 0));
                    let weak = weak.clone();
                    let handler = clock.connect_after_paint(move |_| {
                        if let Some(ui) = weak.upgrade() {
                            let root = if index == 0 {
                                &ui.edge.root
                            } else {
                                &ui.edge.strip_root
                            };
                            let size = (root.width(), root.height());
                            if last_size.replace(size) != size {
                                ui.queue_edge_layout();
                            }
                            if index == 0
                                && ui.edge.search_selection_pending.get()
                                && !ui.edge.output_suspended()
                                && ui.keep_search_selection_visible()
                            {
                                ui.edge.search_selection_pending.set(false);
                            }
                        }
                    });
                    ui.edge.layout_handlers.borrow_mut()[index].push((clock.upcast(), handler));
                }
            });
            let weak = Rc::downgrade(ui);
            window.connect_unrealize(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.edge.disconnect_layout(index);
                }
            });
        }
    }

    pub(super) fn transition_edge(self: &Rc<Self>, event: EdgeEvent) {
        let previous = self.edge.state();
        let (state, effects) = transition(
            previous,
            event,
            self.edge.policy(&self.config.borrow(), self.dnd.get()),
        );
        self.edge.state.set(state);
        if state != previous {
            self.prepare_motion(previous, state);
            self.edge.pending_motion.set(Some(state));
        }
        if previous.is_searching() && !state.is_searching() {
            self.edge.cancel_search_popup();
            self.edge.search_selection_pending.set(false);
        }
        if self.edge.output_suspended()
            && matches!(event, EdgeEvent::FocusWindow | EdgeEvent::FocusStrip)
        {
            return;
        }
        match event {
            EdgeEvent::FocusWindow | EdgeEvent::WindowFocused => {
                self.edge.window.set_keyboard_mode(effects.window_keyboard)
            }
            EdgeEvent::FocusStrip | EdgeEvent::StripFocused => self
                .edge
                .strip_window
                .set_keyboard_mode(effects.strip_keyboard),
            EdgeEvent::Visibility => {
                self.edge.strip_window.set_visible(
                    self.edge
                        .monitor
                        .borrow()
                        .as_ref()
                        .is_some_and(usable_monitor)
                        && (effects.strip_visible || self.edge.toast_note.get().is_some()),
                );
                self.edge
                    .strip_window
                    .set_layer(if self.edge.toast_note.get().is_some() {
                        Layer::Overlay
                    } else {
                        effects.strip_layer
                    });
                self.edge.window.set_visible(
                    self.edge
                        .monitor
                        .borrow()
                        .as_ref()
                        .is_some_and(usable_monitor)
                        && effects.window_visible,
                );
            }
            _ => self.edge.apply(effects),
        }
    }
    pub(super) fn refresh_tabs(self: &Rc<Self>) {
        self.refresh_tabs_inner(true);
    }

    fn refresh_tabs_inner(self: &Rc<Self>, refresh_search: bool) {
        // Saves can arrive during hover motion. Keep the current widgets and input
        // bounds until that motion settles, then apply the latest note data once.
        if self.edge.strip_phase.get() != StripPhase::Settled {
            self.edge.refresh_pending.set(true);
            return;
        }
        self.edge.refresh_pending.set(false);
        let focused_control = gtk::prelude::GtkWindowExt::focus(&self.edge.strip_window)
            .filter(|widget| widget.is_ancestor(&self.edge.tabs))
            .map(|widget| widget.widget_name());
        self.edge.tab_hover.borrow_mut().clear();
        while let Some(child) = self.edge.tabs.first_child() {
            self.edge.tabs.remove(&child);
        }
        while let Some(child) = self.edge.marker.first_child() {
            self.edge.marker.remove(&child);
        }
        let config = self.config.borrow().clone();
        let notes = match self
            .storage
            .load_deck_notes(config.visible_notes, self.edge_sort.get())
        {
            Ok(notes) => notes,
            Err(error) => {
                self.error(&format!("Could not load edge notes: {error}"));
                return;
            }
        };
        // The deck retains the open note during editing. Render the saved query
        // directly so changing the slot count also works with a note docked.
        self.deck.borrow_mut().replace_notes(notes.clone());
        let horizontal = matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom);
        let orientation = if horizontal {
            gtk::Orientation::Horizontal
        } else {
            gtk::Orientation::Vertical
        };
        self.edge.tabs.set_orientation(orientation);
        self.edge.marker.set_orientation(orientation);
        let s = crate::edge_surface::scale_px;
        let depth = s(24);
        // Measure the upright titles, but cap their contribution so a long title
        // cannot widen the whole strip. Both orientations use a slim 24px depth.
        let tab_width = if horizontal {
            let font_desc =
                gtk::pango::FontDescription::from_string(&format!("Inter Semi-Bold {}px", s(11)));
            let title_width = notes
                .iter()
                .map(|note| {
                    let layout = self
                        .edge
                        .tabs
                        .create_pango_layout(Some(&note.title.to_uppercase()));
                    layout.set_font_description(Some(&font_desc));
                    layout.pixel_size().0
                })
                .max()
                .unwrap_or(0);
            let (screen_width, _) = self.note_limits();
            let count = config.visible_notes as i32;
            let reserved = (count - 1) * tab_gap(&config)
                + 2 * TAB_INSET
                + self.edge.tabs.spacing()
                + 3 * depth
                + 2 * motion::HOVER_PAD;
            let available = ((screen_width - reserved) / count).max(depth);
            let width =
                if self.edge.state().is_revealed() && self.edge.horizontal_tab_width.get() > 0 {
                    self.edge.horizontal_tab_width.get()
                } else {
                    (title_width + s(20)).clamp(s(80), s(112))
                }
                .min(available);
            self.edge.horizontal_tab_width.set(width);
            width
        } else {
            self.edge.horizontal_tab_width.set(0);
            depth
        };
        let tab_height = if horizontal {
            depth
        } else {
            let (_, screen_height) = self.note_limits();
            let count = config.visible_notes as i32;
            let reserved = (count - 1) * tab_gap(&config)
                + 2 * TAB_INSET
                + self.edge.tabs.spacing()
                + 3 * depth
                + 2 * motion::HOVER_PAD;
            ((screen_height - reserved) / count).clamp(depth, s(80))
        };
        let fixed = glib::Object::new::<TabLayout>().upcast::<gtk::Fixed>();
        fixed
            .downcast_ref::<TabLayout>()
            .unwrap()
            .imp()
            .pointer
            .set(f64::NAN);
        let gap = tab_gap(&config);
        let inset = 2 * TAB_INSET;

        fixed.set_overflow(gtk::Overflow::Visible);
        fixed.set_size_request(
            if horizontal {
                (notes.len() as i32 * (tab_width + gap) - gap + inset).max(0)
            } else {
                tab_width
            },
            if horizontal {
                tab_height
            } else {
                (notes.len() as i32 * (tab_height + gap) - gap + inset).max(0)
            },
        );
        for (index, note) in notes.iter().enumerate() {
            let tab = gtk::Button::new();
            tab.add_css_class("note-tab");
            tab.set_widget_name(&format!("note-{:?}", note.id));
            appearance::set_color(&tab, note.color);
            let title = if note.title.trim().is_empty() {
                "Untitled note"
            } else {
                &note.title
            };
            tab.update_property(&[gtk::accessible::Property::Label(title)]);
            let label = gtk::Label::new(Some(&title.to_uppercase()));
            label.add_css_class("tab-title");
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_width_chars(1);
            // Let the tab's allocation determine ellipsis, including the fixed
            // 60 px rotated label. Natural text width must not escape its inset.
            label.set_max_width_chars(1);
            label.set_xalign(0.5);
            if horizontal {
                label.set_margin_start(s(9));
                label.set_margin_end(s(9));
                tab.set_child(Some(&label));
            } else {
                let lh = s(16);
                let tw = s(22);
                label.set_size_request(tab_height - s(20), lh);
                let text = gtk::Fixed::new();
                text.set_size_request(tw, tab_height - s(2));
                text.put(&label, 0., 0.);
                let transform = if config.edge == ScreenEdge::Right {
                    gtk::gsk::Transform::new()
                        .translate(&gtk::graphene::Point::new(
                            (tw + lh) as f32 / 2.0,
                            s(9) as f32,
                        ))
                        .rotate(90.)
                } else {
                    gtk::gsk::Transform::new()
                        .translate(&gtk::graphene::Point::new(
                            (tw - lh) as f32 / 2.0,
                            (tab_height - s(11)) as f32,
                        ))
                        .rotate(-90.)
                };
                text.set_child_transform(&label, Some(&transform));
                tab.set_child(Some(&text));
            }
            let extension = if config.tab_layout == config::TabLayout::Classic {
                s(6)
            } else {
                0
            };
            tab.set_size_request(
                tab_width + if horizontal { 0 } else { extension },
                tab_height + if horizontal { extension } else { 0 },
            );
            // Match paint_tab's resting inset before any hover frame arrives.
            let (x, y) = if horizontal {
                (
                    f64::from(TAB_INSET) + index as f64 * f64::from(tab_width + gap),
                    0.,
                )
            } else {
                (
                    0.,
                    f64::from(TAB_INSET) + index as f64 * f64::from(tab_height + gap),
                )
            };
            fixed.put(&tab, x, y);
            self.edge
                .tab_hover
                .borrow_mut()
                .push(Rc::new(motion::Motion::new(0.)));
            let id = note.id;
            let weak = Rc::downgrade(self);
            tab.connect_clicked(move |tab| {
                if let Some(ui) = weak.upgrade() {
                    let editor = ui.edge.docked.borrow().clone();
                    if editor.is_some_and(|editor| !editor.flush()) {
                        return;
                    }
                    ui.remember_origin(tab.upcast_ref());
                    ui.open_docked(id);
                }
            });
        }
        self.edge
            .marker
            .set_spacing(if config.marker_style == config::MarkerStyle::Dots {
                s(5)
            } else {
                0
            });
        match config.marker_style {
            config::MarkerStyle::Bar => {
                let line = gtk::Box::new(orientation, 0);
                line.add_css_class("marker-line");
                line.set_size_request(
                    if horizontal { s(40) } else { s(4) },
                    if horizontal { s(4) } else { s(40) },
                );
                self.edge.marker.append(&line);
            }
            config::MarkerStyle::Dots => {
                // One pill per visible saved note; the empty strip keeps one neutral pill.
                for index in 0..notes.len().max(1) {
                    let dot = gtk::Box::new(orientation, 0);
                    dot.add_css_class("marker-pill");
                    dot.set_size_request(
                        if horizontal { s(14) } else { s(5) },
                        if horizontal { s(5) } else { s(14) },
                    );
                    if let Some(note) = notes.get(index) {
                        dot.add_css_class("edge-result-color");
                        appearance::set_color(&dot, note.color);
                    }
                    self.edge.marker.append(&dot);
                }
            }
        }
        {
            let motion = gtk::EventControllerMotion::new();
            let weak = Rc::downgrade(self);
            let weak_fixed = fixed.downgrade();
            let count = notes.len();
            let update: Rc<dyn Fn(f64, f64)> = Rc::new(move |x, y| {
                let (Some(ui), Some(fixed)) = (weak.upgrade(), weak_fixed.upgrade()) else {
                    return;
                };
                if !crate::edge_surface::is_pointer_in_window(&ui.edge.strip_window) {
                    return;
                }
                if !ui.edge.state().is_revealed() || ui.edge.state().is_repositioning() {
                    return;
                }
                let size = if horizontal { tab_width } else { tab_height };
                // Use resting centers so the growing tabs cannot move their own
                // hover thresholds and oscillate under a stationary pointer.
                let position = (((if horizontal { x } else { y })
                    - f64::from(TAB_INSET)
                    - f64::from(size) / 2.)
                    / f64::from(size + gap))
                .clamp(0., count.saturating_sub(1) as f64);
                let layout = fixed.downcast_ref::<TabLayout>().unwrap().imp();
                layout.pointer.set(position);
                let front = Some(position.round() as usize);
                if layout.front.replace(front) != front {
                    fixed.queue_draw();
                }
                for i in 0..count {
                    ui.animate_tab_hover(
                        i,
                        tab_hover_target(i, position, ui.config.borrow().tab_layout),
                    );
                }
            });
            let enter = update.clone();
            motion.connect_enter(move |_, x, y| enter(x, y));
            motion.connect_motion(move |_, x, y| update(x, y));
            let weak = Rc::downgrade(self);
            let weak_fixed = fixed.downgrade();
            motion.connect_leave(move |_| {
                if let Some(fixed) = weak_fixed.upgrade() {
                    fixed
                        .downcast_ref::<TabLayout>()
                        .unwrap()
                        .imp()
                        .pointer
                        .set(f64::NAN);
                    fixed.queue_draw();
                }
                if let Some(ui) = weak.upgrade() {
                    for i in 0..count {
                        ui.animate_tab_hover(i, 0.);
                    }
                }
            });
            fixed.add_controller(motion);
            // Route overlap clicks to the same frontmost tab used by snapshot.
            let click = gtk::GestureClick::new();
            click.set_button(1);
            click.set_propagation_phase(gtk::PropagationPhase::Capture);
            let weak_fixed = fixed.downgrade();
            click.connect_released(move |gesture, _, x, y| {
                let Some(fixed) = weak_fixed.upgrade() else {
                    return;
                };
                let front = fixed.downcast_ref::<TabLayout>().unwrap().imp().front.get();
                let mut tabs = Vec::new();
                let mut child = fixed.first_child();
                while let Some(tab) = child {
                    child = tab.next_sibling();
                    tabs.push(tab);
                }
                for i in tab_draw_order(tabs.len(), front).into_iter().rev() {
                    if let Some(tab) = tabs.get(i)
                        && tab.compute_bounds(&fixed).is_some_and(|bounds| {
                            bounds.contains_point(&gtk::graphene::Point::new(x as f32, y as f32))
                        })
                    {
                        if let Some(button) = tab.downcast_ref::<gtk::Button>() {
                            button.emit_clicked();
                            gesture.set_state(gtk::EventSequenceState::Claimed);
                        }
                        break;
                    }
                }
            });
            fixed.add_controller(click);
        }
        if !notes.is_empty() {
            self.edge.tabs.append(&fixed);
        }
        let utilities = gtk::Box::new(orientation, 0);
        utilities.add_css_class("edge-utilities");
        // GTK has no CSS overflow property. Clip utility feedback to the capsule.
        utilities.set_overflow(gtk::Overflow::Hidden);
        for (name, label) in [
            ("plus", "New note"),
            ("search", "Search notes"),
            ("grid", "All Notes"),
        ] {
            let button = ui_assets::edge_icon(name, label);
            button.set_widget_name(name);
            button.add_css_class("strip-control");
            button.set_size_request(depth, depth);
            let weak = Rc::downgrade(self);
            button.connect_clicked(move |button| {
                if let Some(ui) = weak.upgrade() {
                    let editor = ui.edge.docked.borrow().clone();
                    if editor.is_some_and(|editor| !editor.flush()) {
                        return;
                    }
                    match name {
                        "plus" => {
                            ui.remember_origin(button.upcast_ref());
                            ui.create_note();
                        }
                        "search" => {
                            if ui.edge.state().is_searching() {
                                ui.close_edge_search();
                            } else {
                                ui.remember_origin(button.upcast_ref());
                                ui.show_edge_search();
                            }
                        }
                        _ => ui.show_manager(),
                    }
                }
            });
            utilities.append(&button);
        }
        self.edge.tabs.append(&utilities);
        if let Some(name) = focused_control {
            let mut group = self.edge.tabs.first_child();
            while let Some(container) = group {
                let mut child = container.first_child();
                while let Some(widget) = child {
                    if widget.widget_name() == name {
                        gtk::prelude::GtkWindowExt::set_focus(
                            &self.edge.strip_window,
                            Some(&widget),
                        );
                    }
                    child = widget.next_sibling();
                }
                group = container.next_sibling();
            }
        }
        ui_assets::refresh_icons(
            self.edge.tabs.upcast_ref(),
            config.theme == config::Theme::Playful,
            false,
        );
        self.edge
            .strip_motion
            .jump(if self.edge.state().is_revealed() {
                1.
            } else {
                0.
            });
        self.paint_strip(self.edge.strip_motion.value());
        if refresh_search && self.edge.state().is_searching() {
            self.refresh_edge_search();
        }
        self.place_edge();
    }
    pub(super) fn remember_origin(&self, widget: &gtk::Widget) {
        if let Some(bounds) = widget.compute_bounds(&self.edge.strip_root) {
            let horizontal = matches!(
                self.config.borrow().edge,
                ScreenEdge::Top | ScreenEdge::Bottom
            );
            self.edge.pointer_origin.set(Some(if horizontal {
                (bounds.x() + bounds.width() / 2.) as i32
            } else {
                (bounds.y() + bounds.height() / 2.) as i32
            }));
        }
    }
    pub(super) fn show_edge_search(self: &Rc<Self>) {
        self.finish_reposition(false).ok();
        let origin = self.edge.pointer_origin.get();
        if !self.close_docked() {
            return;
        }
        self.edge.pointer_origin.set(origin);
        self.reveal();
        self.transition_edge(EdgeEvent::Search);
        self.edge.edge_search.set_text("");
        self.refresh_edge_search();
        self.queue_search_popup(true);
    }

    fn queue_search_popup(self: &Rc<Self>, request_focus: bool) {
        self.edge.cancel_search_popup();
        if self.edge.output_suspended() {
            return;
        }
        self.edge.search_popup_request.set(Some(request_focus));
        self.queue_edge_layout();
    }

    fn popup_edge_search(self: &Rc<Self>, request_focus: bool) -> bool {
        let Some(surface) = self.edge.window.surface() else {
            return false;
        };
        let (x, y, width, height) = self.edge.panel_bounds.get();
        // Preserve the allocation -> placement -> focus -> popup sequence.
        if !self.edge.window.is_mapped()
            || self.edge.search_size_dirty.get()
            || self.edge.root.width() < x + width
            || self.edge.root.height() < y + height
            || surface.width() < x + width
            || surface.height() < y + height
        {
            return false;
        }
        self.place_edge();
        if request_focus {
            self.transition_edge(EdgeEvent::FocusWindow);
        }
        self.edge.search_popover.popup();
        if request_focus {
            self.edge.edge_search.grab_focus();
        }
        true
    }

    pub(super) fn close_edge_search(self: &Rc<Self>) {
        self.transition_edge(EdgeEvent::CloseSearch);
        self.edge.pointer_origin.set(None);
        self.place_edge();
    }

    pub(super) fn refresh_edge_search(self: &Rc<Self>) {
        let query = self.edge.edge_search.text();
        let notes = match self.storage.search_notes(
            NoteCollection::Active,
            query.as_str(),
            self.edge_sort.get(),
        ) {
            Ok(notes) => notes,
            Err(e) => {
                self.error(&format!("Could not search notes: {e}"));
                return;
            }
        };
        let previous = self.edge.search_results.selected_row().and_then(|row| {
            self.edge
                .search_ids
                .borrow()
                .get(row.index() as usize)
                .copied()
        });
        while let Some(child) = self.edge.search_results.first_child() {
            self.edge.search_results.remove(&child);
        }
        let mut ids = Vec::new();
        for note in &notes {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
            dot.add_css_class("edge-result-color");
            dot.set_valign(gtk::Align::Center);
            appearance::set_color(&dot, note.color);
            let title = gtk::Label::new(Some(if note.title.trim().is_empty() {
                "Untitled note"
            } else {
                &note.title
            }));
            title.add_css_class("edge-result-title");
            title.set_xalign(0.);
            title.set_ellipsize(gtk::pango::EllipsizeMode::End);
            title.set_width_chars(1);
            title.set_hexpand(true);
            row.append(&dot);
            row.append(&title);
            self.edge.search_results.append(&row);
            ids.push(note.id);
        }
        let selected = previous
            .and_then(|id| ids.iter().position(|i| *i == id))
            .unwrap_or(0);
        self.edge.search_ids.replace(ids);
        self.edge.search_results.select_row(
            self.edge
                .search_results
                .row_at_index(selected as i32)
                .as_ref(),
        );
        self.edge.search_create.set_visible(notes.is_empty());
        self.edge
            .search_create
            .set_sensitive(!query.trim().is_empty());
        self.edge
            .search_create
            .set_label(if query.trim().is_empty() {
                "No active notes"
            } else {
                "No matches. Create note"
            });
        self.edge.search_size_dirty.set(true);
        self.queue_edge_layout();
        // The existing frame clock's after-paint observer scrolls after GTK
        // allocates the rebuilt rows. Do not run a tick loop or queue stale idles.
        self.edge
            .search_selection_pending
            .set(self.edge.search_results.selected_row().is_some());
    }
    fn keep_search_selection_visible(&self) -> bool {
        if !self.edge.state().is_searching() {
            return true;
        }
        let Some(row) = self.edge.search_results.selected_row() else {
            return true;
        };
        let Some(bounds) = row.compute_bounds(&self.edge.search_results) else {
            return false;
        };
        if bounds.height() <= 0. {
            return false;
        }
        let adjustment = self.edge.search_scroll.vadjustment();
        let top = f64::from(bounds.y());
        let bottom = f64::from(bounds.y() + bounds.height());
        if adjustment.page_size() <= 0. || adjustment.upper() < bottom {
            return false;
        }
        adjustment.clamp_page(top, bottom);
        true
    }
    pub(super) fn activate_edge_search(self: &Rc<Self>) {
        self.refresh_edge_search();
        let id = self.edge.search_results.selected_row().and_then(|row| {
            self.edge
                .search_ids
                .borrow()
                .get(row.index() as usize)
                .copied()
        });
        if let Some(id) = id {
            self.open_docked(id);
        } else {
            let title = self.edge.edge_search.text();
            if !title.trim().is_empty() {
                self.edge.pointer_origin.set(None);
                self.create_note_with_title(title.trim());
            }
        }
    }
    pub(super) fn schedule_reveal(self: &Rc<Self>) {
        let generation = self.edge.cancel_timers();
        self.edge.deferred_hide.set(None);
        if self.edge.strip_phase.get() == StripPhase::Collapsing {
            self.reveal();
            return;
        }
        self.edge.reveal_armed.set(true);
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(140), move || {
            if let Some(ui) = weak.upgrade()
                && ui.edge.generation.get() == generation
            {
                ui.edge.reveal_armed.set(false);
                if ui.config.borrow().edge_hover_enabled && ui.edge.state() == EdgeState::Dormant {
                    ui.reveal();
                }
            }
        });
    }

    pub(super) fn reveal(self: &Rc<Self>) {
        if self.edge.state().docked_id().is_some() {
            return;
        }
        self.edge.cancel_timers();
        self.deck.borrow_mut().dispatch(Action::ShowTabs);
        self.transition_edge(EdgeEvent::Reveal);
        self.place_edge();
        self.set_input();
    }

    pub(super) fn dormant(self: &Rc<Self>) {
        let (state, _) = transition(
            self.edge.state(),
            EdgeEvent::Collapse,
            self.edge.policy(&self.config.borrow(), self.dnd.get()),
        );
        if state != EdgeState::Dormant {
            return;
        }
        self.deck.borrow_mut().dispatch(Action::CollapseDeck);
        self.transition_edge(EdgeEvent::Collapse);
        if self.edge.strip_phase.get() == StripPhase::Settled {
            self.edge.strip_offset.set(None);
        }
        self.place_edge();
        self.set_input();
    }

    pub(super) fn schedule_hide(self: &Rc<Self>) {
        let generation = self.edge.cancel_timers();
        if self.edge.strip_phase.get() == StripPhase::Revealing {
            self.edge.deferred_hide.set(Some(generation));
            return;
        }
        self.arm_hide(generation);
    }
    fn arm_hide(self: &Rc<Self>, generation: u64) {
        self.edge.hide_armed.set(true);
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(350), move || {
            if let Some(ui) = weak.upgrade()
                && ui.edge.generation.get() == generation
            {
                ui.edge.hide_armed.set(false);
                ui.dormant();
            }
        });
    }

    pub(super) fn begin_reposition(self: &Rc<Self>) -> Result<(), String> {
        if !self.config_valid {
            return Err("Correct the invalid configuration before repositioning.".into());
        }
        if self.config.borrow().hide_in_dnd && self.dnd.get() {
            return Err("Turn off Do Not Disturb before repositioning the strip.".into());
        }
        if self.edge.state().is_repositioning() {
            return Ok(());
        }
        if !self.close_docked() {
            return Err(
                "The open note could not be saved. Retry saving it before repositioning.".into(),
            );
        }
        self.close_edge_search();
        self.edge
            .preview_offset
            .set(self.config.borrow().marker_offset);
        self.edge.reposition_error.set_visible(false);
        self.edge.pointer_origin.set(None);
        self.edge.docked_offset.set(None);
        self.reveal();
        self.transition_edge(EdgeEvent::Reposition);
        self.place_edge();
        self.update_visibility();
        self.transition_edge(EdgeEvent::FocusWindow);
        if let Some(done) = self
            .edge
            .stack
            .child_by_name("reposition")
            .and_then(|panel| panel.last_child())
            .and_then(|buttons| buttons.last_child())
        {
            done.grab_focus();
        }
        Ok(())
    }
    pub(super) fn finish_reposition(self: &Rc<Self>, save: bool) -> Result<(), String> {
        let Some(offset) = self.edge.reposition_offset() else {
            return Ok(());
        };
        if save {
            let mut config = self.config.borrow().clone();
            config.marker_offset = offset;
            if let Err(error) = config.save(&config::config_path()) {
                let message = format!("Could not save position: {error}");
                self.edge.reposition_error.set_text(&message);
                self.edge.reposition_error.set_visible(true);
                self.place_edge();
                return Err(message);
            }
            self.config.replace(config);
        }
        self.edge.strip_offset.set(None);
        self.transition_edge(EdgeEvent::FinishReposition);
        self.place_edge();
        if let Some(manager) = self.manager.borrow().as_ref() {
            manager.refresh_appearance();
        }
        self.dormant();
        Ok(())
    }
    pub(super) fn dragged_offset(&self, start: f64, x: f64, y: f64) -> f64 {
        let horizontal = matches!(
            self.config.borrow().edge,
            ScreenEdge::Top | ScreenEdge::Bottom
        );
        let (width, height) = self.note_limits();
        let (tab_width, tab_height) = self.strip_size();
        let available = if horizontal {
            width - tab_width
        } else {
            height - tab_height
        };
        if available <= 0 {
            return start;
        }
        (start + if horizontal { x } else { y } / f64::from(available)).clamp(0., 1.)
    }
    pub(super) fn preview_reposition(self: &Rc<Self>, start: f64, x: f64, y: f64) {
        if self.edge.state().is_repositioning() {
            self.edge
                .preview_offset
                .set(self.dragged_offset(start, x, y));
            self.edge.strip_offset.set(None);
            self.place_edge();
        }
    }
    pub(super) fn update_visibility(self: &Rc<Self>) {
        if self.config.borrow().hide_in_dnd && self.dnd.get() {
            self.finish_reposition(false).ok();
        }
        self.transition_edge(EdgeEvent::Visibility);
    }

    pub(super) fn apply_edge_classes(&self, config: &Config) {
        let edge = match config.edge {
            ScreenEdge::Left => "edge-left",
            ScreenEdge::Right => "edge-right",
            ScreenEdge::Top => "edge-top",
            ScreenEdge::Bottom => "edge-bottom",
        };
        let theme = match config.theme {
            config::Theme::Paper => "edge-paper",
            config::Theme::Playful => "edge-playful",
            config::Theme::Minimal if appearance::dark(config) => "edge-minimal-dark",
            config::Theme::Minimal => "edge-minimal",
        };
        for window in [&self.edge.window, &self.edge.strip_window] {
            for class in [
                "edge-left",
                "edge-right",
                "edge-top",
                "edge-bottom",
                "edge-paper",
                "edge-playful",
                "edge-minimal",
                "edge-minimal-dark",
            ] {
                if class != edge && class != theme {
                    window.remove_css_class(class);
                }
            }
            if config.tab_layout == config::TabLayout::Classic {
                window.add_css_class("edge-classic");
            } else {
                window.remove_css_class("edge-classic");
            }
            window.add_css_class(edge);
            window.add_css_class(theme);
        }
    }

    pub(super) fn place_edge(self: &Rc<Self>) {
        if self.edge.state().is_searching() && self.edge.search_size_dirty.get() {
            self.queue_edge_layout();
            return;
        }
        if !self
            .edge
            .monitor
            .borrow()
            .as_ref()
            .is_some_and(usable_monitor)
        {
            return;
        }
        let config = self.config.borrow();
        self.apply_edge_classes(&config);
        let effects = self.edge.effects(&config, self.dnd.get());
        let (screen_width, screen_height) = self.note_limits();
        let horizontal = matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom);
        // Both layer surfaces keep their origin and extent across every state.
        // Only child allocations and their disjoint input regions change.
        for (window, root) in [
            (&self.edge.window, &self.edge.root),
            (&self.edge.strip_window, &self.edge.strip_root),
        ] {
            if root.width_request() != screen_width || root.height_request() != screen_height {
                root.set_size_request(screen_width, screen_height);
                window.set_default_size(screen_width, screen_height);
            }
            crate::edge_surface::sync_surface_geometry(
                window,
                self.edge.monitor.borrow().as_ref(),
                screen_width,
                screen_height,
            );
        }
        let measure = |widget: &gtk::Widget| {
            let visible = widget.get_visible();
            widget.set_visible(true);
            let (_, width, _, _) = widget.measure(gtk::Orientation::Horizontal, -1);
            let (_, height, _, _) = widget.measure(gtk::Orientation::Vertical, width);
            widget.set_visible(visible);
            (
                width - widget.margin_start() - widget.margin_end(),
                height - widget.margin_top() - widget.margin_bottom(),
            )
        };
        let (tw, th) = self.strip_size();
        let available = if horizontal {
            screen_width - tw
        } else {
            screen_height - th
        }
        .max(0);
        let offset = self
            .edge
            .strip_offset
            .get()
            .unwrap_or(
                (f64::from(available)
                    * self
                        .edge
                        .reposition_offset()
                        .unwrap_or(config.marker_offset)) as i32,
            )
            .clamp(0, available);
        if self.edge.state().is_revealed() {
            self.edge.strip_offset.set(Some(offset));
        }
        let (tx, ty) = match config.edge {
            ScreenEdge::Left => (0, offset),
            ScreenEdge::Right => (screen_width - tw, offset),
            ScreenEdge::Top => (offset, 0),
            ScreenEdge::Bottom => (offset, screen_height - th),
        };
        let position = |widget: &gtk::Widget, x: i32, y: i32| {
            widget.set_halign(gtk::Align::Start);
            widget.set_valign(gtk::Align::Start);
            widget.set_margin_start(x.max(0));
            widget.set_margin_top(y.max(0));
        };
        // Measure without placement margins, which otherwise feed back into size.
        position(self.edge.tabs.upcast_ref(), tx, ty);
        let (mw, mh) = measure(self.edge.marker.upcast_ref());
        let (mx, my) = match config.edge {
            ScreenEdge::Left => (
                0,
                (offset + (th - mh) / 2).clamp(0, (screen_height - mh).max(0)),
            ),
            ScreenEdge::Right => (
                screen_width - mw,
                (offset + (th - mh) / 2).clamp(0, (screen_height - mh).max(0)),
            ),
            ScreenEdge::Top => (
                (offset + (tw - mw) / 2).clamp(0, (screen_width - mw).max(0)),
                0,
            ),
            ScreenEdge::Bottom => (
                (offset + (tw - mw) / 2).clamp(0, (screen_width - mw).max(0)),
                screen_height - mh,
            ),
        };
        position(self.edge.marker.upcast_ref(), mx, my);
        self.edge.marker_bounds.set((mx, my, mw, mh));
        self.edge.strip_bounds.set(
            if effects.strip_input == StripInput::Tabs
                || self.edge.strip_phase.get() == StripPhase::Collapsing
            {
                (tx, ty, tw, th)
            } else {
                (mx, my, mw, mh)
            },
        );
        let docked = self.edge.docked.borrow();
        // Use the same tab sizes and spacing as refresh_tabs. Newly rebuilt tabs
        // may not have allocated positions yet. Overflow keeps its Search origin.
        if let Some(id) = self.edge.state().docked_id()
            && let Some(fixed) = self.edge.tabs.first_child().and_downcast::<gtk::Fixed>()
        {
            let mut child = fixed.first_child();
            let mut index = 0;
            while let Some(tab) = child {
                if tab.widget_name() == format!("note-{id:?}") {
                    let length = if horizontal {
                        tab.width_request()
                    } else {
                        tab.height_request()
                    };
                    self.edge.pointer_origin.set(Some(
                        offset + TAB_INSET + index * (length + tab_gap(&config)) + length / 2,
                    ));
                    break;
                }
                index += 1;
                child = tab.next_sibling();
            }
        } else if self.edge.state().is_searching()
            && let Some(utilities) = self.edge.tabs.last_child()
        {
            let (w, h) = measure(&utilities);
            self.edge.pointer_origin.set(Some(
                offset + if horizontal { tw - w / 2 } else { th - h / 2 },
            ));
        }
        if let Some(panel) = effects.panel {
            let gap = 10;
            let (max_width, max_height) = if horizontal {
                (screen_width, screen_height - th - gap)
            } else {
                (screen_width - tw - gap, screen_height)
            };
            let (width, height) = if let (EdgeState::Docked(_), Some(editor)) =
                (self.edge.state(), docked.as_ref())
            {
                let (w, h) = editor.docked_size();
                editor.set_docked_size(w.min(max_width), h.min(max_height));
                editor.set_resize_edge(match config.edge {
                    ScreenEdge::Right => gtk::gdk::SurfaceEdge::SouthWest,
                    ScreenEdge::Bottom => gtk::gdk::SurfaceEdge::NorthEast,
                    _ => gtk::gdk::SurfaceEdge::SouthEast,
                });
                crate::edge_surface::sync_resize_grip(&editor.resize);
                editor.docked_size()
            } else if self.edge.state().is_searching() {
                let size = self.edge.search_size.get();
                self.edge
                    .stack
                    .child_by_name("search")
                    .unwrap()
                    .set_size_request(size.0, size.1);
                size
            } else {
                measure(self.edge.stack.child_by_name(panel).unwrap().upcast_ref())
            };
            let origin = self
                .edge
                .pointer_origin
                .get()
                .unwrap_or(offset + if horizontal { tw / 2 } else { th / 2 });
            let along = self
                .edge
                .docked_offset
                .get()
                .filter(|(edge, _)| *edge == config.edge)
                .map_or(
                    origin - if horizontal { width / 2 } else { height / 2 },
                    |(_, offset)| offset,
                );
            // Keep an in-place switch when the arrow can reach its tab. Otherwise
            // move the sheet just enough to avoid pointing at a different note.
            let panel_length = if horizontal { width } else { height };
            let along = along.clamp(origin - panel_length + 30, origin - 30);
            let along = along.clamp(
                0,
                if horizontal {
                    screen_width - width
                } else {
                    screen_height - height
                }
                .max(0),
            );
            if self.edge.state().docked_id().is_some() {
                self.edge.docked_offset.set(Some((config.edge, along)));
            }
            let (x, y) = match config.edge {
                ScreenEdge::Left => (tw + gap, along),
                ScreenEdge::Right => (screen_width - tw - gap - width, along),
                ScreenEdge::Top => (along, th + gap),
                ScreenEdge::Bottom => (along, screen_height - th - gap - height),
            };
            let (x, y) = (x.max(0), y.max(0));
            position(self.edge.stack.upcast_ref(), x, y);
            self.edge.panel_bounds.set((x, y, width, height));
            if self.edge.state().is_searching()
                && self
                    .edge
                    .search_placement
                    .replace(Some((x, y, width, height)))
                    != Some((x, y, width, height))
            {
                self.edge
                    .search_popover
                    .set_pointing_to(Some(&gtk::gdk::Rectangle::new(x + width / 2, y - 1, 1, 1)));
                if self.edge.search_popover.is_visible() && !self.edge.output_suspended() {
                    self.edge.search_popover.present();
                }
            }
        }
        if self.edge.toast_note.get().is_some() {
            let (width, height) = measure(self.edge.delete_toast.upcast_ref());
            let (sx, sy, sw, sh) = self.edge.strip_bounds.get();
            let (x, y) = match config.edge {
                ScreenEdge::Left => (sx + sw + 8, sy + (sh - height) / 2),
                ScreenEdge::Right => (sx - width - 8, sy + (sh - height) / 2),
                ScreenEdge::Top => (sx + (sw - width) / 2, sy + sh + 8),
                ScreenEdge::Bottom => (sx + (sw - width) / 2, sy - height - 8),
            };
            let x = x.clamp(0, (screen_width - width).max(0));
            let y = y.clamp(0, (screen_height - height).max(0));
            position(self.edge.delete_toast.upcast_ref(), x, y);
            self.edge.toast_bounds.set((x, y, width, height));
        }
        self.edge.apply(effects);
        drop(docked);
        drop(config);
        self.edge.pointer.queue_draw();
        self.edge.root.queue_draw();
        self.set_input();
        if let Some(state) = self.edge.pending_motion.take() {
            self.animate_transition(state);
        }
    }
    pub(super) fn show_delete_toast(self: &Rc<Self>, id: NoteId) {
        if let Some(timer) = self.edge.toast_timeout.borrow_mut().take() {
            timer.remove();
        }
        let generation = self.edge.toast_generation.get().wrapping_add(1);
        self.edge.toast_generation.set(generation);
        self.edge.toast_note.set(Some(id));
        self.edge.delete_toast.set_visible(true);
        self.edge
            .apply(self.edge.effects(&self.config.borrow(), self.dnd.get()));
        self.transition_edge(EdgeEvent::Visibility);
        self.place_edge();
        let weak = Rc::downgrade(self);
        self.edge.toast_motion.animate(
            &self.edge.strip_window,
            1.,
            motion::PANEL_MS,
            self.motion_enabled(),
            move |value| {
                if let Some(ui) = weak.upgrade() {
                    ui.edge.delete_toast.set_opacity(value);
                }
            },
            || {},
        );
        let weak = Rc::downgrade(self);
        self.edge
            .toast_timeout
            .replace(Some(glib::timeout_add_local_once(
                Duration::from_secs(6),
                move || {
                    if let Some(ui) = weak.upgrade()
                        && ui.edge.toast_generation.get() == generation
                    {
                        ui.edge.toast_timeout.borrow_mut().take();
                        ui.hide_delete_toast();
                    }
                },
            )));
    }
    fn hide_delete_toast(self: &Rc<Self>) {
        if let Some(timer) = self.edge.toast_timeout.borrow_mut().take() {
            timer.remove();
        }
        let generation = self.edge.toast_generation.get();
        let weak = Rc::downgrade(self);
        let done = weak.clone();
        self.edge.toast_motion.animate(
            &self.edge.strip_window,
            0.,
            motion::PANEL_MS,
            self.motion_enabled(),
            move |value| {
                if let Some(ui) = weak.upgrade() {
                    ui.edge.delete_toast.set_opacity(value);
                }
            },
            move || {
                if let Some(ui) = done.upgrade()
                    && ui.edge.toast_generation.get() == generation
                {
                    ui.edge.toast_note.set(None);
                    ui.edge.delete_toast.set_visible(false);
                    ui.edge
                        .apply(ui.edge.effects(&ui.config.borrow(), ui.dnd.get()));
                    ui.transition_edge(EdgeEvent::Visibility);
                    ui.place_edge();
                }
            },
        );
    }
    pub(super) fn note_limits(&self) -> (i32, i32) {
        // Layer coordinates start at (0, 0) on the selected output. GDK's
        // desktop-global x/y must not be added, nor logical sizes scaled again.
        self.edge
            .monitor
            .borrow()
            .as_ref()
            .map(|monitor| {
                let geometry = monitor.geometry();
                (geometry.width().max(1), geometry.height().max(1))
            })
            .unwrap_or((720, 500))
    }

    #[cfg(target_os = "linux")]
    pub(super) fn docked_pointer(&self, gesture: &gtk::GestureDrag) -> Option<(f64, f64)> {
        // The panel surface has a fixed screen origin, including during resize.
        gesture.current_event()?.position()
    }
    fn strip_input_bounds(&self) -> (i32, i32, i32, i32) {
        if self.edge.toast_note.get().is_some() {
            return self.edge.toast_bounds.get();
        }

        let (x, y, width, height) = self.edge.strip_bounds.get();
        let expanded = (self.edge.state().is_revealed()
            || self.edge.strip_phase.get() == StripPhase::Collapsing)
            && !self.edge.state().is_repositioning();
        let pad = if expanded { 20 } else { 7 };
        let (screen_width, screen_height) = self.note_limits();
        let left = (x - pad).max(0);
        let top = (y - pad).max(0);
        (
            left,
            top,
            (x + width + pad).min(screen_width) - left,
            (y + height + pad).min(screen_height) - top,
        )
    }
    pub(super) fn set_input(&self) {
        let effects = self.edge.effects(&self.config.borrow(), self.dnd.get());
        let strip_extra = self
            .edge
            .toast_note
            .get()
            .is_some()
            .then(|| self.edge.strip_bounds.get());
        let panel_passthrough = (!effects.panel_input
            && self.edge.outgoing_panel.borrow().is_some())
        .then(|| self.edge.panel_bounds.get());

        crate::edge_surface::apply_input_region(
            &self.edge.strip_window,
            Some(self.strip_input_bounds()),
            strip_extra,
            12,
            None,
        );
        crate::edge_surface::apply_input_region(
            &self.edge.window,
            effects.panel_input.then(|| self.edge.panel_bounds.get()),
            None,
            18,
            panel_passthrough,
        );
    }

    pub(super) fn draw_pointer(&self, cr: &gtk::cairo::Context) {
        let (x, y, w, h) = self.edge.panel_bounds.get();
        let bounds = gtk::graphene::Rect::new(x as f32, y as f32, w as f32, h as f32);
        let session = self
            .edge
            .docked
            .borrow()
            .as_ref()
            .and_then(|editor| {
                self.sessions
                    .borrow()
                    .get(&editor.id())
                    .and_then(Weak::upgrade)
            })
            .or_else(|| self.edge.outgoing_session.borrow().clone());
        let config = session.as_ref().map_or_else(
            || self.config.borrow().clone(),
            |session| session.style().resolve(&self.config.borrow()),
        );
        let edge = config.edge;
        let horizontal = matches!(edge, ScreenEdge::Top | ScreenEdge::Bottom);
        let start = if horizontal {
            bounds.x() as f64
        } else {
            bounds.y() as f64
        };
        let length = if horizontal {
            bounds.width() as f64
        } else {
            bounds.height() as f64
        };
        let offset = if horizontal {
            self.edge.window.margin(Edge::Left)
        } else {
            self.edge.window.margin(Edge::Top)
        };
        let origin = if self.edge.outgoing_panel.borrow().is_some() && !self.note_switching() {
            self.edge.outgoing_origin.get()
        } else {
            self.edge.pointer_origin.get()
        };
        let center = origin
            .map_or(start + length / 2., |origin| f64::from(origin - offset))
            .clamp(start + 30., start + length - 30.);
        let x = bounds.x() as f64;
        let y = bounds.y() as f64;
        let w = bounds.width() as f64;
        let h = bounds.height() as f64;
        // Overlap the base far enough to cover even the inward Playful waves.
        // Only the portion outside the actual sheet outline gets an arrow stroke.
        let inset = if session.is_some() { 20. } else { 1. };
        let note_path = session.as_ref().and_then(|session| {
            let shape = if config.theme == config::Theme::Minimal {
                config::Shape::Square
            } else {
                config.shape
            };
            let bottom_radius =
                if config.theme == config::Theme::Paper && shape != config::Shape::Square {
                    0.
                } else {
                    18.
                };
            let padding = if config.theme == config::Theme::Playful {
                20.
            } else {
                16.
            };
            let _ = cr.save();
            cr.translate(x + 8., y + 8.);
            appearance::sheet_path(
                cr,
                (w - padding).max(1.),
                (h - padding).max(1.),
                shape,
                (18., bottom_radius),
                config.theme == config::Theme::Playful,
                session.id().value(),
            );
            let _ = cr.restore();
            cr.copy_path().ok()
        });
        let (tip, a, b) = match edge {
            ScreenEdge::Left => (
                (x - 4., center),
                (x + inset, center - 6.),
                (x + inset, center + 6.),
            ),
            ScreenEdge::Right => (
                (x + w + 4., center),
                (x + w - inset, center + 6.),
                (x + w - inset, center - 6.),
            ),
            ScreenEdge::Top => (
                (center, y - 4.),
                (center + 6., y + inset),
                (center - 6., y + inset),
            ),
            ScreenEdge::Bottom => (
                (center, y + h + 4.),
                (center - 6., y + h - inset),
                (center + 6., y + h - inset),
            ),
        };
        let (r, g, bg) = if let Some(session) = session.as_ref() {
            match config.theme {
                config::Theme::Playful => session.color().fill(),
                config::Theme::Minimal if appearance::dark(&config) => {
                    (39. / 255., 44. / 255., 50. / 255.)
                }
                config::Theme::Minimal => (1., 1., 1.),
                config::Theme::Paper => (1., 0.966, 0.864),
            }
        } else if config.theme == config::Theme::Minimal && appearance::dark(&config) {
            (34. / 255., 37. / 255., 44. / 255.)
        } else {
            (248. / 255., 247. / 255., 243. / 255.)
        };
        cr.new_path();
        cr.move_to(a.0, a.1);
        cr.line_to(tip.0, tip.1);
        cr.line_to(b.0, b.1);
        cr.set_source_rgb(r, g, bg);
        let _ = cr.fill_preserve();
        let arrow_path = cr.copy_path().ok();
        let _ = cr.save();
        if let (Some(note_path), Some(arrow_path)) = (note_path, arrow_path) {
            cr.new_path();
            cr.rectangle(x - 32., y - 32., w + 64., h + 64.);
            cr.append_path(&note_path);
            cr.set_fill_rule(gtk::cairo::FillRule::EvenOdd);
            cr.clip();
            cr.append_path(&arrow_path);
        }
        cr.set_line_join(gtk::cairo::LineJoin::Round);
        if session.is_some() && config.theme == config::Theme::Playful {
            cr.set_source_rgb(0.10, 0.12, 0.11);
            cr.set_line_width(3.2);
        } else if config.theme == config::Theme::Minimal && appearance::dark(&config) {
            cr.set_source_rgba(0.75, 0.80, 0.86, 0.12);
            cr.set_line_width(0.7);
        } else {
            cr.set_source_rgba(
                0.22,
                0.24,
                0.27,
                if config.theme == config::Theme::Minimal {
                    0.12
                } else {
                    0.16
                },
            );
            cr.set_line_width(0.7);
        }
        let _ = cr.stroke();
        let _ = cr.restore();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> Policy {
        Policy {
            transient_visible: true,
            hide_in_fullscreen: true,
            window_active: false,
            strip_active: false,
        }
    }

    #[test]
    fn dormant_tabs_docked_and_back_to_dormant() {
        let id = NoteId::new(1);
        let (state, effects) = transition(EdgeState::Dormant, EdgeEvent::Reveal, policy());
        assert_eq!(state, EdgeState::Tabs);
        assert!(!effects.marker_visible && effects.tabs_visible);
        assert_eq!(effects.strip_input, StripInput::Tabs);
        assert!(!effects.panel_input);
        assert_eq!(effects.strip_keyboard, KeyboardMode::OnDemand);
        let (state, effects) = transition(state, EdgeEvent::Dock(id), policy());
        assert_eq!(state, EdgeState::Docked(id));
        assert_eq!(effects.panel, Some("editor"));
        assert!(effects.tabs_visible && effects.pointer_visible && effects.panel_input);
        assert_eq!(effects.window_keyboard, KeyboardMode::OnDemand);
        assert_eq!(effects.strip_keyboard, KeyboardMode::None);
        assert_eq!(effects.window_layer, Layer::Overlay);
        // A saved note can switch directly without a hidden Tabs-only frame.
        let next = NoteId::new(2);
        let (state, effects) = transition(state, EdgeEvent::Dock(next), policy());
        assert_eq!(state, EdgeState::Docked(next));
        assert_eq!(effects.panel, Some("editor"));
        assert!(effects.panel_input && effects.pointer_visible);
        // Closing keeps the tabs open until the leave grace expires.
        let (state, effects) = transition(state, EdgeEvent::CloseDocked, policy());
        assert_eq!(state, EdgeState::Tabs);
        assert_eq!(effects.panel, None);
        let (state, effects) = transition(state, EdgeEvent::Collapse, policy());
        assert_eq!(state, EdgeState::Dormant);
        assert!(effects.marker_visible && !effects.tabs_visible);
        assert_eq!(effects.strip_input, StripInput::Marker);
        assert!(!effects.panel_input && !effects.pointer_visible);
        assert_eq!(effects.strip_keyboard, KeyboardMode::None);
        assert_eq!(effects.window_keyboard, KeyboardMode::None);
    }

    #[test]
    fn search_entry_exit_and_leave_grace() {
        for initial in [EdgeState::Dormant, EdgeState::Tabs] {
            let (state, effects) = transition(initial, EdgeEvent::Search, policy());
            assert_eq!(state, EdgeState::Search);
            assert_eq!(effects.panel, Some("search"));
            assert!(effects.tabs_visible && effects.pointer_visible && effects.panel_input);
            assert_eq!(effects.window_keyboard, KeyboardMode::OnDemand);
            assert_eq!(effects.strip_keyboard, KeyboardMode::None);
            assert_eq!(effects.window_layer, Layer::Top);
            assert_eq!(
                transition(state, EdgeEvent::Collapse, policy()),
                (state, effects)
            );
            let (state, effects) = transition(state, EdgeEvent::CloseSearch, policy());
            assert_eq!(state, EdgeState::Tabs);
            assert_eq!(effects.panel, None);
            assert!(!effects.pointer_visible && !effects.panel_input);
        }
    }

    #[test]
    fn reposition_entry_exit_and_leave_grace() {
        for initial in [EdgeState::Dormant, EdgeState::Tabs] {
            let (state, effects) = transition(initial, EdgeEvent::Reposition, policy());
            assert_eq!(state, EdgeState::Reposition);
            assert_eq!(effects.panel, Some("reposition"));
            assert!(effects.tabs_visible && effects.panel_input);
            assert!(!effects.pointer_visible);
            assert_eq!(effects.window_keyboard, KeyboardMode::OnDemand);
            assert_eq!(
                transition(state, EdgeEvent::Collapse, policy()),
                (state, effects)
            );
            let (state, effects) = transition(state, EdgeEvent::FinishReposition, policy());
            assert_eq!(state, EdgeState::Dormant);
            assert!(effects.marker_visible);
            assert!(!effects.tabs_visible && !effects.panel_input);
        }
    }

    #[test]
    fn hover_leave_and_reveal_preserve_a_docked_note() {
        let state = EdgeState::Docked(NoteId::new(1));
        let expected = transition(state, EdgeEvent::Refresh, policy());
        assert_eq!(transition(state, EdgeEvent::Collapse, policy()), expected);
        assert_eq!(transition(state, EdgeEvent::Reveal, policy()), expected);
        // Search/Reposition require a successful close. Direct note switches
        // pass the coordinator's save/release guards before this transition.
        assert_eq!(transition(state, EdgeEvent::Search, policy()), expected);
        assert_eq!(transition(state, EdgeEvent::Reposition, policy()), expected);
        assert_eq!(
            transition(state, EdgeEvent::Dock(NoteId::new(2)), policy()),
            (EdgeState::Docked(NoteId::new(2)), expected.1)
        );
    }

    #[test]
    fn dnd_and_fullscreen_keep_committed_notes_protected() {
        let hidden = Policy {
            transient_visible: false,
            ..policy()
        };
        for state in [
            EdgeState::Dormant,
            EdgeState::Tabs,
            EdgeState::Search,
            EdgeState::Reposition,
            EdgeState::Docked(NoteId::new(1)),
        ] {
            let (_, effects) = transition(state, EdgeEvent::Visibility, hidden);
            assert!(!effects.strip_visible);
            assert_eq!(effects.window_visible, state.docked_id().is_some());
            assert_eq!(effects.strip_layer, Layer::Top);
            assert_eq!(
                effects.window_layer,
                if state.docked_id().is_some() {
                    Layer::Overlay
                } else {
                    Layer::Top
                }
            );
            let (_, effects) = transition(
                state,
                EdgeEvent::Refresh,
                Policy {
                    hide_in_fullscreen: false,
                    ..policy()
                },
            );
            assert_eq!(effects.strip_layer, Layer::Overlay);
            assert_eq!(effects.window_layer, Layer::Overlay);
        }
    }

    #[test]
    fn explicit_focus_requests_return_to_on_demand() {
        let (_, effects) = transition(EdgeState::Search, EdgeEvent::FocusWindow, policy());
        assert_eq!(effects.window_keyboard, KeyboardMode::Exclusive);
        let (_, effects) = transition(EdgeState::Search, EdgeEvent::WindowFocused, policy());
        assert_eq!(effects.window_keyboard, KeyboardMode::OnDemand);
        let (_, effects) = transition(EdgeState::Tabs, EdgeEvent::FocusStrip, policy());
        assert_eq!(effects.strip_keyboard, KeyboardMode::Exclusive);
        let (_, effects) = transition(EdgeState::Tabs, EdgeEvent::StripFocused, policy());
        assert_eq!(effects.strip_keyboard, KeyboardMode::OnDemand);
        let (_, effects) = transition(
            EdgeState::Search,
            EdgeEvent::FocusWindow,
            Policy {
                window_active: true,
                ..policy()
            },
        );
        assert_eq!(effects.window_keyboard, KeyboardMode::OnDemand);
    }
    #[test]
    #[ignore = "requires Wayland and disposable XDG_DATA_HOME/XDG_CONFIG_HOME directories"]
    fn motion_keeps_input_bounds_and_reverses_from_painted_progress() {
        for variable in ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
            assert!(
                std::path::PathBuf::from(std::env::var(variable).unwrap())
                    .starts_with(std::env::temp_dir())
            );
        }
        ui_assets::register().unwrap();
        adw::init().unwrap();
        let settings = gtk::Settings::default().unwrap();
        settings.set_gtk_enable_animations(true);
        Config {
            launch_at_login: false,
            ..Config::default()
        }
        .save(&config::config_path())
        .unwrap();
        let app = gtk::Application::builder()
            .application_id("dev.stickies.MotionRegression")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let storage = crate::storage::Storage::open(&crate::storage::database_path()).unwrap();
        for title in ["One", "Two", "Three", "Four"] {
            storage
                .create_note(title, "Body", NoteColor::Green)
                .unwrap();
        }
        let ui = crate::build(&app, &Rc::new(Cell::new(false))).unwrap();
        let settle = |ms| {
            let end = std::time::Instant::now() + Duration::from_millis(ms);
            while std::time::Instant::now() < end {
                while glib::MainContext::default().pending() {
                    glib::MainContext::default().iteration(false);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        };
        let intermediate = |phase, value: &dyn Fn() -> f64| {
            let deadline = std::time::Instant::now() + Duration::from_millis(500);
            loop {
                let progress = value();
                if progress > 0. && progress < 1. {
                    return progress;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "{phase}: animation never painted an intermediate frame: {progress}"
                );
                settle(1);
            }
        };
        settle(250);
        assert!(ui.motion_enabled());
        // The dormant marker must keep the same placement on its first reveal.
        let marker = ui.edge.marker_bounds.get();
        ui.reveal();
        assert_eq!(ui.edge.marker_bounds.get(), marker);
        assert_eq!(ui.edge.tabs.last_child().unwrap().opacity(), 0.);
        // Sample the first reveal, including GTK's first tab allocations.
        let extent = (ui.edge.root.width(), ui.edge.root.height());
        let expanded = ui.edge.strip_bounds.get();
        let progress = intermediate("reveal", &|| ui.edge.strip_motion.value());
        let fixed = ui.edge.tabs.first_child().unwrap();
        assert_eq!(
            (fixed.width(), fixed.height()),
            (fixed.width_request(), fixed.height_request())
        );

        assert!(
            progress > 0. && progress < 1.,
            "reveal must have intermediate frames: {progress}"
        );
        let session = ui.note_session(ui.deck.borrow().notes()[0].id).unwrap();
        session.body.set_text("Saved during reveal");
        assert!(session.flush());
        assert_eq!(ui.edge.strip_motion.value(), progress);
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Revealing);
        assert!(ui.edge.refresh_pending.get());
        assert_eq!(ui.edge.strip_bounds.get(), expanded);
        ui.schedule_hide();
        assert!(ui.edge.deferred_hide.get().is_some());
        settle(240);
        assert_eq!(ui.edge.state(), EdgeState::Tabs);

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while ui.edge.strip_phase.get() != StripPhase::Settled {
            assert!(
                std::time::Instant::now() < deadline,
                "first reveal did not finish"
            );
            settle(5);
        }
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Settled);
        assert!(!ui.edge.refresh_pending.get());
        // The 350ms grace starts after reveal finishes, not at the earlier leave.
        settle(150);
        assert_eq!(ui.edge.state(), EdgeState::Tabs);
        ui.schedule_reveal();
        ui.dormant();
        assert_eq!(ui.edge.strip_bounds.get(), expanded);
        let before = intermediate("collapse", &|| ui.edge.strip_motion.value());
        assert!(
            before > 0. && before < 1.,
            "collapse must have intermediate frames: {before}"
        );
        session.body.set_text("Saved during collapse");
        assert!(session.flush());
        assert_eq!(ui.edge.strip_motion.value(), before);
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Collapsing);
        assert_eq!(ui.edge.strip_bounds.get(), expanded);
        assert!(ui.edge.refresh_pending.get());
        ui.schedule_reveal();
        assert_eq!(ui.edge.state(), EdgeState::Tabs);
        assert_eq!(
            ui.edge.strip_motion.value(),
            before,
            "re-entry must not restart progress"
        );
        assert_eq!(ui.edge.strip_bounds.get(), expanded);
        settle(250);
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Settled);
        ui.dormant();
        settle(200);
        assert_eq!(ui.edge.strip_bounds.get(), ui.edge.marker_bounds.get());
        assert!(!ui.edge.tabs.is_visible());
        ui.schedule_reveal();
        settle(185);
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Revealing);
        let before = ui.edge.strip_motion.value();
        ui.schedule_hide();
        settle(15);
        ui.schedule_reveal();
        assert!(ui.edge.strip_motion.value() >= before);
        assert!(ui.edge.deferred_hide.get().is_none());
        settle(500);
        assert_eq!(ui.edge.state(), EdgeState::Tabs);
        ui.show_edge_search();
        let before = intermediate("search open", &|| ui.edge.panel_motion.value());
        assert!(before > 0. && before < 1., "panel must fade: {before}");
        ui.edge.edge_search.set_text("Four");
        ui.refresh_edge_search();
        assert_eq!(
            ui.edge.panel_motion.value(),
            1.,
            "typing must finish the opening fade immediately"
        );
        assert!(!ui.edge.panel_motion.is_running());
        settle(150);
        ui.close_edge_search();
        settle(150);
        assert!(!ui.edge.stack.is_visible());
        for index in 0..20 {
            storage
                .create_note(&format!("Result {index:02}"), "Body", NoteColor::Blue)
                .unwrap();
        }
        ui.show_edge_search();
        settle(160);
        let search_focus = gtk::prelude::RootExt::focus(&ui.edge.window).unwrap();
        assert!(
            search_focus == ui.edge.edge_search.clone().upcast::<gtk::Widget>()
                || search_focus.is_ancestor(&ui.edge.edge_search)
        );
        ui.edge
            .search_results
            .select_row(ui.edge.search_results.row_at_index(15).as_ref());
        assert!(ui.keep_search_selection_visible());
        let check_selection = |phase| {
            let row = ui.edge.search_results.selected_row().unwrap();
            let bounds = row.compute_bounds(&ui.edge.search_results).unwrap();
            let adjustment = ui.edge.search_scroll.vadjustment();
            assert!(f64::from(bounds.y()) >= adjustment.value() - 1.);
            assert!(
                f64::from(bounds.y() + bounds.height())
                    <= adjustment.value() + adjustment.page_size() + 1.,
                "{phase}: row={bounds:?}, value={}, page={}, upper={}",
                adjustment.value(),
                adjustment.page_size(),
                adjustment.upper()
            );
            assert_eq!(
                gtk::prelude::RootExt::focus(&ui.edge.window),
                Some(search_focus.clone())
            );
        };
        check_selection("selection");
        ui.refresh_edge_search();
        settle(100);
        check_selection("refresh");
        for _ in 0..3 {
            ui.close_edge_search();
            let before = intermediate("search close", &|| ui.edge.panel_motion.value());
            assert!(before > 0. && before < 1.);
            ui.show_edge_search();
            assert_eq!(ui.edge.panel_motion.value(), before);
            // GTK stores widget opacity at 8-bit precision.
            assert!((ui.edge.stack.opacity() - before).abs() <= 1. / 255.);
            assert!((ui.edge.pointer.opacity() - before).abs() <= 1. / 255.);
            settle(160);
            assert_eq!(ui.edge.panel_motion.value(), 1.);
        }
        ui.close_edge_search();
        settle(160);
        // The outgoing pointer must retain its geometry even when the panel is
        // clamped against a screen end and its origin differs from its center.
        let pointer_pixels = || {
            let (width, height) = ui.note_limits();
            let mut surface =
                gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, width, height)
                    .unwrap();
            {
                let cr = gtk::cairo::Context::new(&surface).unwrap();
                ui.draw_pointer(&cr);
            }
            surface.data().unwrap().to_vec()
        };
        for edge in [
            ScreenEdge::Left,
            ScreenEdge::Right,
            ScreenEdge::Top,
            ScreenEdge::Bottom,
        ] {
            for offset in [0., 1.] {
                let mut config = ui.config.borrow().clone();
                config.edge = edge;
                config.marker_offset = offset;
                ui.change_settings(config).unwrap();
                let id = ui.deck.borrow().notes()[0].id;
                ui.open_docked(id);
                settle(160);
                let before = pointer_pixels();
                assert!(ui.close_docked());
                assert_eq!(
                    pointer_pixels(),
                    before,
                    "note pointer moved on close: {edge:?}/{offset}"
                );
                settle(160);
                assert!(ui.edge.outgoing_origin.get().is_none());
                ui.show_edge_search();
                settle(160);
                let before = pointer_pixels();
                ui.close_edge_search();
                assert_eq!(
                    pointer_pixels(),
                    before,
                    "search pointer moved on close: {edge:?}/{offset}"
                );
                settle(160);
            }
        }
        ui.begin_reposition().unwrap();
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Settled);
        assert_eq!(ui.edge.stack.opacity(), 1.);
        ui.finish_reposition(false).unwrap();
        ui.config.borrow_mut().reduced_motion = true;
        ui.reveal();
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Settled);
        ui.dormant();
        assert_eq!(ui.edge.strip_bounds.get(), ui.edge.marker_bounds.get());
        ui.config.borrow_mut().reduced_motion = false;
        settings.set_gtk_enable_animations(false);
        ui.reveal();
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Settled);
        assert_eq!(ui.edge.strip_motion.value(), 1.);
        assert_eq!((ui.edge.root.width(), ui.edge.root.height()), extent);
        // Cancel a queued start before its first tick, then unmap during another.
        settings.set_gtk_enable_animations(true);
        ui.dormant();
        settings.set_gtk_enable_animations(false);
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Settled);
        settings.set_gtk_enable_animations(true);
        ui.reveal();
        ui.edge.strip_window.set_visible(false);
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Settled);
        assert!(!ui.edge.strip_motion.is_running());
        ui.edge.window.close();
        ui.edge.strip_window.close();
    }
    #[test]
    #[ignore = "requires Wayland and disposable XDG directories"]
    fn hover_keeps_titles_attached_and_preserves_clicks() {
        for variable in ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
            assert!(
                std::path::PathBuf::from(std::env::var(variable).unwrap())
                    .starts_with(std::env::temp_dir())
            );
        }
        ui_assets::register().unwrap();
        adw::init().unwrap();
        gtk::Settings::default()
            .unwrap()
            .set_gtk_enable_animations(true);
        Config {
            launch_at_login: false,
            ..Config::default()
        }
        .save(&config::config_path())
        .unwrap();
        let app = gtk::Application::builder()
            .application_id("dev.stickies.HoverRegression")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let storage = crate::storage::Storage::open(&crate::storage::database_path()).unwrap();
        storage
            .create_note("Groceries", "Body", NoteColor::Green)
            .unwrap();
        storage
            .create_note("Reading", "Body", NoteColor::Pink)
            .unwrap();
        let ui = crate::build(&app, &Rc::new(Cell::new(false))).unwrap();
        let settle = |ms| {
            let end = std::time::Instant::now() + Duration::from_millis(ms);
            while std::time::Instant::now() < end {
                while glib::MainContext::default().pending() {
                    glib::MainContext::default().iteration(false);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
        };
        let capture = |name: &str| {
            if let Ok(directory) = std::env::var("STICKIES_HOVER_CAPTURES") {
                let snapshot = gtk::Snapshot::new();
                gtk::WidgetPaintable::new(Some(&ui.edge.strip_root)).snapshot(
                    &snapshot,
                    ui.edge.strip_root.width() as f64,
                    ui.edge.strip_root.height() as f64,
                );
                let (x, y, w, h) = ui.edge.strip_bounds.get();
                let (sw, sh) = ui.note_limits();
                let left = (x - 8).max(0);
                let top = (y - 8).max(0);
                let bounds = gtk::graphene::Rect::new(
                    left as f32,
                    top as f32,
                    ((x + w + 8).min(sw) - left) as f32,
                    ((y + h + 8).min(sh) - top) as f32,
                );
                ui.edge
                    .strip_window
                    .renderer()
                    .unwrap()
                    .render_texture(snapshot.to_node().unwrap(), Some(&bounds))
                    .save_to_png(std::path::Path::new(&directory).join(format!("{name}.png")))
                    .unwrap();
            }
        };
        settle(250);
        ui.reveal();
        settle(400);
        let hover_at = |index: Option<usize>, offset: f64| {
            let fixed = ui.edge.tabs.first_child().unwrap();
            let tab = fixed.first_child().unwrap();
            let horizontal = matches!(
                ui.config.borrow().edge,
                ScreenEdge::Top | ScreenEdge::Bottom
            );
            let size = if horizontal {
                tab.width_request()
            } else {
                tab.height_request()
            };
            let along = f64::from(TAB_INSET)
                + index.unwrap_or(0) as f64 * f64::from(size + TAB_GAP)
                + f64::from(size) / 2.
                + offset;
            let (x, y) = if horizontal {
                (along, 12.)
            } else {
                (12., along)
            };
            for controller in fixed.observe_controllers().iter::<glib::Object>().flatten() {
                if let Ok(motion) = controller.downcast::<gtk::EventControllerMotion>() {
                    if index.is_some() {
                        motion.emit_by_name::<()>("motion", &[&x, &y]);
                    } else {
                        motion.emit_by_name::<()>("leave", &[]);
                    }
                }
            }
        };
        let hover = |index| hover_at(index, 0.);
        hover(Some(0));
        settle(45);
        let progress = ui.edge.tab_hover.borrow()[0].value();
        assert!(progress > 0. && progress < 1.);
        hover_at(Some(0), 8.);
        assert_eq!(ui.edge.tab_hover.borrow()[0].value(), progress);
        assert!(
            ui.edge
                .tabs
                .first_child()
                .unwrap()
                .downcast_ref::<TabLayout>()
                .unwrap()
                .imp()
                .pointer
                .get()
                > 0.,
            "the hover destination must follow movement within a tab"
        );
        settle(240);
        assert!(ui.edge.tab_hover.borrow()[0].value() > 0.9);
        assert!(ui.edge.tab_hover.borrow()[0].value() < 1.);
        assert!(ui.edge.tab_hover.borrow()[1].value() > 0.25);
        hover(Some(0));
        settle(240);
        assert_eq!(ui.edge.tab_hover.borrow()[0].value(), 1.);
        assert!((ui.edge.tab_hover.borrow()[1].value() - 0.25).abs() < 0.001);
        let progress = ui.edge.tab_hover.borrow()[0].value();
        let fixed = ui
            .edge
            .tabs
            .first_child()
            .unwrap()
            .downcast::<gtk::Fixed>()
            .unwrap();
        let tab = fixed.first_child().unwrap();
        let origin = gtk::graphene::Point::new(0., 0.);
        let before = fixed
            .child_transform(&tab)
            .unwrap()
            .transform_point(&origin);
        hover(Some(1));
        assert_eq!(ui.edge.tab_hover.borrow()[0].value(), progress);
        let after = fixed
            .child_transform(&tab)
            .unwrap()
            .transform_point(&origin);
        assert!(
            (after.x() - before.x()).abs() < 0.01 && (after.y() - before.y()).abs() < 0.01,
            "switching tabs must preserve the painted hover transform"
        );
        settle(45);
        let progress = ui.edge.tab_hover.borrow()[1].value();
        hover(None);
        assert_eq!(ui.edge.tab_hover.borrow()[1].value(), progress);
        settle(240);
        assert!(
            ui.edge
                .tab_hover
                .borrow()
                .iter()
                .all(|hover| hover.value() == 0.)
        );
        let extent = (ui.edge.strip_root.width(), ui.edge.strip_root.height());
        for (theme, appearance, name) in [
            (config::Theme::Paper, config::Appearance::Light, "paper"),
            (config::Theme::Playful, config::Appearance::Light, "playful"),
            (
                config::Theme::Minimal,
                config::Appearance::Light,
                "minimal-light",
            ),
            (
                config::Theme::Minimal,
                config::Appearance::Dark,
                "minimal-dark",
            ),
        ] {
            for edge in [
                ScreenEdge::Left,
                ScreenEdge::Right,
                ScreenEdge::Top,
                ScreenEdge::Bottom,
            ] {
                let mut config = ui.config.borrow().clone();
                config.theme = theme;
                config.appearance = appearance;
                config.edge = edge;
                config.reduced_motion = true;
                ui.change_settings(config).unwrap();
                ui.reveal();
                settle(80);
                let prefix = format!("{name}-{edge:?}").to_lowercase();
                let fixed = ui.edge.tabs.first_child().unwrap();
                let tab = fixed.first_child().unwrap();
                let base = tab.compute_bounds(&ui.edge.strip_root).unwrap();
                let input = ui.strip_input_bounds();
                capture(&format!("{prefix}-normal"));
                hover(Some(0));
                settle(40);
                let hovered = tab.compute_bounds(&ui.edge.strip_root).unwrap();
                assert!((hovered.width() / base.width() - 1.24).abs() < 0.001);
                assert!((hovered.height() / base.height() - 1.24).abs() < 0.001);
                assert_eq!(ui.edge.tab_hover.borrow()[0].value(), 1.);
                assert!(!ui.edge.tab_hover.borrow()[0].is_running());
                let (screen_width, screen_height) = ui.note_limits();
                let gap = match edge {
                    ScreenEdge::Left => hovered.x(),
                    ScreenEdge::Right => screen_width as f32 - hovered.x() - hovered.width(),
                    ScreenEdge::Top => hovered.y(),
                    ScreenEdge::Bottom => screen_height as f32 - hovered.y() - hovered.height(),
                };
                assert!(gap.abs() < 0.01, "hover left an edge gap: {edge:?}/{gap}");
                assert_eq!(
                    (fixed.width(), fixed.height()),
                    (fixed.width_request(), fixed.height_request())
                );
                assert_eq!(
                    ui.strip_input_bounds(),
                    input,
                    "hover must not resize the input region per frame"
                );
                assert!(hovered.x() >= input.0 as f32 && hovered.y() >= input.1 as f32);
                assert!(hovered.x() + hovered.width() <= (input.0 + input.2) as f32 + 0.01);
                assert!(hovered.y() + hovered.height() <= (input.1 + input.3) as f32 + 0.01);
                let (px, py) = match edge {
                    ScreenEdge::Left => {
                        (base.x() + base.width() + 0.5, base.y() + base.height() / 2.)
                    }
                    ScreenEdge::Right => (base.x() - 0.5, base.y() + base.height() / 2.),
                    ScreenEdge::Top => {
                        (base.x() + base.width() / 2., base.y() + base.height() + 0.5)
                    }
                    ScreenEdge::Bottom => (base.x() + base.width() / 2., base.y() - 0.5),
                };
                let picked = ui
                    .edge
                    .strip_root
                    .pick(px as f64, py as f64, gtk::PickFlags::DEFAULT)
                    .unwrap();
                assert!(
                    picked == tab || picked.is_ancestor(&tab),
                    "edge title must receive clicks"
                );
                capture(&format!("{prefix}-hover"));
                tab.clone()
                    .downcast::<gtk::Button>()
                    .unwrap()
                    .emit_clicked();
                settle(60);
                assert!(
                    ui.edge.state().docked_id().is_some(),
                    "hovered tab must open its note"
                );
                capture(&format!("{prefix}-selected-hover"));
                assert!(ui.close_docked());
                settle(40);
                assert_eq!(
                    (ui.edge.strip_root.width(), ui.edge.strip_root.height()),
                    extent
                );
            }
        }
        let mut config = ui.config.borrow().clone();
        config.reduced_motion = false;
        ui.change_settings(config).unwrap();
        gtk::Settings::default()
            .unwrap()
            .set_gtk_enable_animations(true);
        hover(Some(0));
        settle(35);
        gtk::Settings::default()
            .unwrap()
            .set_gtk_enable_animations(false);
        assert_eq!(ui.edge.tab_hover.borrow()[0].value(), 1.);
        assert!(!ui.edge.tab_hover.borrow()[0].is_running());
        ui.begin_reposition().unwrap();
        assert!(
            ui.edge
                .tab_hover
                .borrow()
                .iter()
                .all(|hover| hover.value() == 0. && !hover.is_running())
        );
        ui.finish_reposition(false).unwrap();
        ui.edge.window.close();
        ui.edge.strip_window.close();
    }
}

#[cfg(test)]
mod feedback_tests {
    use super::*;
    #[test]
    #[ignore = "requires isolated Wayland, private bus and disposable XDG directories"]
    fn quick_search_native_popup_uses_the_requested_panel_position() {
        for key in ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
            assert!(std::path::Path::new(&std::env::var(key).unwrap()).starts_with("/tmp"));
        }
        ui_assets::register().unwrap();
        adw::init().unwrap();
        let app = gtk::Application::builder()
            .application_id("dev.stickies.SearchPlacementRegression")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let storage = crate::storage::Storage::open(&crate::storage::database_path()).unwrap();
        for index in 0..12 {
            storage
                .create_note(&format!("Note {index}"), "café 東京", NoteColor::Blue)
                .unwrap();
        }
        let settle = || {
            let end = std::time::Instant::now() + Duration::from_millis(250);
            while std::time::Instant::now() < end {
                while glib::MainContext::default().iteration(false) {}
                std::thread::sleep(Duration::from_millis(2));
            }
        };
        for edge in [
            ScreenEdge::Right,
            ScreenEdge::Left,
            ScreenEdge::Top,
            ScreenEdge::Bottom,
        ] {
            Config {
                edge,
                ..Config::default()
            }
            .save(&crate::config::config_path())
            .unwrap();
            // Each edge starts with a new, not-yet-allocated parent surface.
            let ui = crate::build(&app, &Rc::new(Cell::new(false))).unwrap();
            let layouts = Rc::new(RefCell::new(Vec::new()));
            let positions = layouts.clone();
            let weak = Rc::downgrade(&ui);
            ui.edge.search_popover.connect_realize(move |popover| {
                let positions = positions.clone();
                let weak = weak.clone();
                // Map precedes the async configure. Check the first configured
                // layout and every later one, before relying on a settled frame.
                popover
                    .surface()
                    .unwrap()
                    .connect_layout(move |surface, _, _| {
                        let ui = weak.upgrade().unwrap();
                        if ui.edge.state().is_searching() {
                            let popup = surface.downcast_ref::<gtk::gdk::Popup>().unwrap();
                            let (x, y, _, _) = ui.edge.panel_bounds.get();
                            positions.borrow_mut().push((
                                popup.position_x(),
                                popup.position_y(),
                                x,
                                y,
                            ));
                        }
                    });
            });
            ui.show_edge_search();
            settle();
            assert!(
                ui.edge.search_popover.is_mapped(),
                "{edge:?}: initial opening"
            );
            assert!(
                ui.edge.tabs.is_mapped(),
                "{edge:?}: title strip must remain visible"
            );
            assert_eq!(ui.edge.strip_motion.value(), 1.);
            let first_layouts = layouts.borrow().len();
            assert!(first_layouts > 0);
            ui.close_edge_search();
            settle();
            assert!(!ui.edge.search_popover.is_mapped());
            // Use the same button callback as a strip click for the second opening.
            ui.edge
                .tabs
                .last_child()
                .unwrap()
                .first_child()
                .unwrap()
                .next_sibling()
                .unwrap()
                .downcast::<gtk::Button>()
                .unwrap()
                .emit_clicked();
            settle();
            assert!(
                ui.edge.search_popover.is_mapped(),
                "{edge:?}: strip opening"
            );
            assert!(layouts.borrow().len() > first_layouts);
            for &(actual_x, actual_y, x, y) in layouts.borrow().iter() {
                assert!(
                    (actual_x - x).abs() <= 10 && (actual_y - y).abs() <= 10,
                    "{edge:?}: native popup at {actual_x}, {actual_y}; requested panel at {x}, {y}"
                );
            }
            ui.close_edge_search();
            ui.edge.window.close();
            ui.edge.strip_window.close();
            settle();
        }
        // Closing before the first allocation must cancel the queued popup.
        let ui = crate::build(&app, &Rc::new(Cell::new(false))).unwrap();
        ui.show_edge_search();
        ui.close_edge_search();
        settle();
        assert!(!ui.edge.search_popover.is_mapped());
        assert!(!ui.edge.state().is_searching());
        ui.edge.window.close();
        ui.edge.strip_window.close();
    }

    #[test]
    #[ignore = "requires Wayland and disposable XDG directories"]
    fn stacked_tabs_and_bar_keep_clicks_and_bounds_on_every_edge() {
        for key in ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
            assert!(std::path::Path::new(&std::env::var(key).unwrap()).starts_with("/tmp"));
        }
        ui_assets::register().unwrap();
        adw::init().unwrap();
        let app = gtk::Application::builder()
            .application_id("dev.stickies.StackedCheck")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let storage = crate::storage::Storage::open(&crate::storage::database_path()).unwrap();
        for title in ["Shopping", "Ideas", "Weekend", "Reading"] {
            storage
                .create_note(title, "Body", NoteColor::Yellow)
                .unwrap();
        }
        let ui = crate::build(&app, &Rc::new(Cell::new(false))).unwrap();
        let settle = || {
            let end = std::time::Instant::now() + Duration::from_millis(200);
            while std::time::Instant::now() < end {
                while glib::MainContext::default().iteration(false) {}
                std::thread::sleep(Duration::from_millis(2));
            }
        };
        for edge in [
            ScreenEdge::Left,
            ScreenEdge::Right,
            ScreenEdge::Top,
            ScreenEdge::Bottom,
        ] {
            for reduced in [false, true] {
                let mut config = ui.config.borrow().clone();
                config.edge = edge;
                config.reduced_motion = reduced;
                ui.change_settings(config).unwrap();
                ui.dormant();
                settle();
                assert!(
                    ui.edge
                        .marker
                        .first_child()
                        .unwrap()
                        .has_css_class("marker-line")
                );
                assert!(
                    ui.edge
                        .marker
                        .first_child()
                        .unwrap()
                        .next_sibling()
                        .is_none()
                );
                ui.reveal();
                settle();
                let fixed = ui
                    .edge
                    .tabs
                    .first_child()
                    .unwrap()
                    .downcast::<gtk::Fixed>()
                    .unwrap();
                let tabs: Vec<_> = {
                    let mut tabs = Vec::new();
                    let mut child = fixed.first_child();
                    while let Some(tab) = child {
                        child = tab.next_sibling();
                        tabs.push(tab);
                    }
                    tabs
                };
                assert!(tabs.iter().all(|tab| tab.tooltip_text().is_none()));
                let horizontal = matches!(edge, ScreenEdge::Top | ScreenEdge::Bottom);
                let size = if horizontal {
                    tabs[0].width_request()
                } else {
                    tabs[0].height_request()
                };
                let along = f64::from(TAB_INSET) + f64::from(size + TAB_GAP) + f64::from(size) / 2.;
                let (x, y) = if horizontal {
                    (along, 12.)
                } else {
                    (12., along)
                };
                let input = ui.strip_input_bounds();
                for controller in fixed.observe_controllers().iter::<glib::Object>().flatten() {
                    if let Ok(motion) = controller.downcast::<gtk::EventControllerMotion>() {
                        motion.emit_by_name::<()>("motion", &[&x, &y]);
                    }
                }
                settle();
                assert!((ui.edge.tab_hover.borrow()[1].value() - 1.).abs() < 0.001);
                assert!((ui.edge.tab_hover.borrow()[0].value() - 0.25).abs() < 0.001);
                assert_eq!(
                    fixed
                        .downcast_ref::<TabLayout>()
                        .unwrap()
                        .imp()
                        .pointer
                        .get(),
                    1.
                );
                assert_eq!(ui.strip_input_bounds(), input);
                for tab in &tabs {
                    let bounds = tab.compute_bounds(&ui.edge.strip_root).unwrap();
                    assert!(
                        bounds.x() >= input.0 as f32 - 0.01 && bounds.y() >= input.1 as f32 - 0.01
                    );
                    assert!(
                        bounds.x() + bounds.width() <= (input.0 + input.2) as f32 + 0.01
                            && bounds.y() + bounds.height() <= (input.1 + input.3) as f32 + 0.01
                    );
                }
                if let Ok(path) = std::env::var("STICKIES_EDGE_CAPTURES") {
                    std::fs::create_dir_all(&path).unwrap();
                    let snapshot = gtk::Snapshot::new();
                    gtk::WidgetPaintable::new(Some(&ui.edge.strip_root)).snapshot(
                        &snapshot,
                        ui.edge.strip_root.width() as f64,
                        ui.edge.strip_root.height() as f64,
                    );
                    let bounds = gtk::graphene::Rect::new(
                        input.0 as f32,
                        input.1 as f32,
                        input.2 as f32,
                        input.3 as f32,
                    );
                    ui.edge
                        .strip_window
                        .renderer()
                        .unwrap()
                        .render_texture(snapshot.to_node().unwrap(), Some(&bounds))
                        .save_to_png(
                            std::path::Path::new(&path)
                                .join(format!("stacked-{edge:?}-{reduced}.png")),
                        )
                        .unwrap();
                }
                let id = ui.deck.borrow().notes()[1].id;
                for controller in fixed.observe_controllers().iter::<glib::Object>().flatten() {
                    if let Ok(click) = controller.downcast::<gtk::GestureClick>() {
                        click.emit_by_name::<()>("released", &[&1i32, &x, &y]);
                    }
                }
                assert_eq!(ui.edge.state(), EdgeState::Docked(id));
                assert!(ui.close_docked());
            }
        }
        ui.edge.window.close();
        ui.edge.strip_window.close();
    }
    #[test]
    #[ignore = "requires Wayland and disposable XDG directories"]
    fn individual_collection_guards_and_edge_undo_lifetime() {
        for key in ["XDG_DATA_HOME", "XDG_CONFIG_HOME", "XDG_CACHE_HOME"] {
            assert!(std::path::Path::new(&std::env::var(key).unwrap()).starts_with("/tmp"));
        }
        ui_assets::register().unwrap();
        adw::init().unwrap();
        Config {
            reduced_motion: true,
            ..Config::default()
        }
        .save(&config::config_path())
        .unwrap();
        let app = gtk::Application::builder()
            .application_id("dev.stickies.EdgeUndoCheck")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk::gio::Cancellable>).unwrap();
        let storage = crate::storage::Storage::open(&crate::storage::database_path()).unwrap();
        let id = storage
            .create_note("Keep me", "Original", NoteColor::Green)
            .unwrap()
            .id;
        storage.set_note_pinned(id, true).unwrap();
        let ui = crate::build(&app, &Rc::new(Cell::new(false))).unwrap();
        let settle = |ms| {
            let end = std::time::Instant::now() + Duration::from_millis(ms);
            while std::time::Instant::now() < end {
                while glib::MainContext::default().iteration(false) {}
                std::thread::sleep(Duration::from_millis(2));
            }
        };
        ui.open_docked(id);
        settle(200);
        let editor = ui.edge.docked.borrow().clone().unwrap();
        let db = rusqlite::Connection::open(crate::storage::database_path()).unwrap();
        db.execute_batch("CREATE TRIGGER reject_note_save BEFORE UPDATE ON notes BEGIN SELECT RAISE(FAIL, 'save rejected'); END;").unwrap();
        editor.body.buffer().set_text("Saved before delete");
        assert!(editor.controls.archive.is_none());
        editor.controls.delete.emit_clicked();
        assert_eq!(ui.edge.state(), EdgeState::Docked(id));
        assert!(storage.load_note(id).is_ok());
        assert!(ui.edge.toast_note.get().is_none());
        db.execute_batch("DROP TRIGGER reject_note_save;").unwrap();
        assert!(ui.close_docked());
        assert_eq!(storage.load_note(id).unwrap().body, "Saved before delete");
        ui.open_separate(id);
        settle(200);
        let separate = ui.separate.borrow().get(&id).unwrap().editor.clone();
        separate.controls.delete.emit_clicked();
        settle(200);
        assert!(!ui.separate.borrow().contains_key(&id));
        assert!(ui.manager.borrow().is_none());
        assert_eq!(ui.edge.toast_note.get(), Some(id));
        assert_eq!(ui.edge.delete_toast.opacity(), 1.);
        assert!(!ui.edge.toast_motion.is_running());
        assert_eq!(ui.edge.strip_window.layer(), Layer::Overlay);
        ui.dnd.set(true);
        ui.update_visibility();
        assert!(ui.edge.strip_window.is_visible());
        ui.edge.undo_delete.emit_clicked();
        settle(100);
        assert!(storage.load_note(id).unwrap().pinned);
        assert!(!storage.is_archived(id).unwrap());
        assert!(ui.edge.toast_note.get().is_none());
        // The same storage path also preserves an archived note's previous collection.
        storage.archive_note(id).unwrap();
        crate::manager::change_collection(&storage, id, crate::manager::ManagerChange::Delete)
            .unwrap();
        ui.show_delete_toast(id);
        ui.edge.undo_delete.emit_clicked();
        assert!(storage.is_archived(id).unwrap());
        assert!(
            storage
                .search_notes(
                    crate::storage::NoteCollection::Archived,
                    "",
                    crate::storage::NoteSort::Recent
                )
                .unwrap()
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .pinned
        );
        let first = storage
            .create_note("First", "Body", NoteColor::Blue)
            .unwrap()
            .id;
        let second = storage
            .create_note("Second", "Body", NoteColor::Pink)
            .unwrap()
            .id;
        crate::manager::change_collection(&storage, first, crate::manager::ManagerChange::Delete)
            .unwrap();
        ui.show_delete_toast(first);
        let old = glib::MainContext::default()
            .find_source_by_id(ui.edge.toast_timeout.borrow().as_ref().unwrap())
            .unwrap();
        crate::manager::change_collection(&storage, second, crate::manager::ManagerChange::Delete)
            .unwrap();
        ui.show_delete_toast(second);
        assert!(old.is_destroyed());
        assert_eq!(ui.edge.toast_note.get(), Some(second));
        assert_eq!(
            app.windows().len(),
            2,
            "the toast must reuse the existing strip surface"
        );
        for edge in [
            ScreenEdge::Left,
            ScreenEdge::Right,
            ScreenEdge::Top,
            ScreenEdge::Bottom,
        ] {
            for offset in [0., 1.] {
                ui.config.borrow_mut().edge = edge;
                ui.config.borrow_mut().marker_offset = offset;
                ui.edge.strip_offset.set(None);
                ui.place_edge();
                settle(50);
                let (x, y, w, h) = ui.edge.toast_bounds.get();
                let (sw, sh) = ui.note_limits();
                assert!(w > 0 && h > 0 && x >= 0 && y >= 0 && x + w <= sw && y + h <= sh);
                assert_eq!(
                    ui.strip_input_bounds(),
                    ui.edge.toast_bounds.get(),
                    "only the toast accepts strip-surface input"
                );
            }
        }
        for (theme, appearance, name) in [
            (config::Theme::Paper, config::Appearance::Light, "paper"),
            (config::Theme::Playful, config::Appearance::Light, "playful"),
            (
                config::Theme::Minimal,
                config::Appearance::Light,
                "minimal-light",
            ),
            (
                config::Theme::Minimal,
                config::Appearance::Dark,
                "minimal-dark",
            ),
        ] {
            ui.config.borrow_mut().theme = theme;
            ui.config.borrow_mut().appearance = appearance;
            ui.place_edge();
            settle(100);
            crate::text_editing::capture_native(&format!("{name}-edge-undo"));
        }
        ui.show_delete_toast(second);
        settle(5800);
        assert_eq!(
            ui.edge.toast_note.get(),
            Some(second),
            "toast must last six seconds"
        );
        settle(450);
        assert!(ui.edge.toast_note.get().is_none());
        assert!(!ui.edge.delete_toast.is_visible());
        assert!(ui.edge.toast_timeout.borrow().is_none());
        ui.dnd.set(false);
        ui.update_visibility();
        // An animated toast uses the existing 120ms helper; reduced motion above skipped it.
        ui.config.borrow_mut().reduced_motion = false;
        gtk::Settings::default()
            .unwrap()
            .set_gtk_enable_animations(true);
        settle(100);
        ui.show_delete_toast(second);
        settle(45);
        assert!(ui.edge.toast_motion.value() > 0. && ui.edge.toast_motion.value() < 1.);
        settle(200);
        assert_eq!(ui.edge.toast_motion.value(), 1.);
        ui.hide_delete_toast();
        settle(250);
        assert!(ui.edge.toast_note.get().is_none());
        ui.edge.window.close();
        ui.edge.strip_window.close();
    }
}

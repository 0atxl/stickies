use crate::{
    Ui,
    app::{Action, NoteColor, NoteId},
    appearance,
    config::{self, Config, ScreenEdge},
    editor, motion,
    storage::NoteCollection,
    ui_assets,
};
use gtk::{glib, prelude::*};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    time::Duration,
};

// Keep transformed children out of the tab group's measurement. GtkFixed uses
// its layout manager directly, so overriding Widget::measure alone has no effect.
mod tab_layout {
    use gtk::{glib, prelude::*, subclass::prelude::*};
    #[derive(Default)]
    pub struct TabLayout;
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
    impl WidgetImpl for TabLayout {}
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
    pub struct TabLayout(ObjectSubclass<tab_layout::TabLayout>)
        @extends gtk::Fixed, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
glib::wrapper! {
    pub struct TabMeasure(ObjectSubclass<tab_layout::TabMeasure>) @extends gtk::LayoutManager;
}

const WIDTH: i32 = 420;
const HEIGHT: i32 = 500;

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
        (State::Dormant | State::Tabs | State::Search, Event::Dock(id)) => State::Docked(id),
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
    pub(super) search_results: gtk::ListBox,
    search_scroll: gtk::ScrolledWindow,
    pub(super) search_ids: RefCell<Vec<NoteId>>,
    pub(super) search_create: gtk::Button,
    pub(super) pointer: gtk::DrawingArea,
    pub(super) pointer_origin: Cell<Option<i32>>,
    pub(super) docked: RefCell<Option<Rc<editor::Editor>>>,
    pub(super) docked_offset: Cell<Option<(ScreenEdge, i32)>>,
    pub(super) cancel_reposition: gtk::Button,
    pub(super) done_reposition: gtk::Button,
    state: Cell<EdgeState>,
    generation: Cell<u64>,
    preview_offset: Cell<f64>,
    strip_phase: Cell<StripPhase>,
    pending_motion: Cell<Option<EdgeState>>,
    deferred_hide: Cell<Option<u64>>,
    refresh_pending: Cell<bool>,
    marker_bounds: Cell<(i32, i32, i32, i32)>,
    tab_motion: RefCell<Vec<motion::Motion>>,
    tab_hover: RefCell<Vec<Rc<motion::Motion>>>,
    hovered_tab: Cell<Option<usize>>,
    marker_motion: motion::Motion,
    utilities_motion: motion::Motion,
    panel_motion: motion::Motion,
    outgoing_panel: RefCell<Option<gtk::Widget>>,
    outgoing_origin: Cell<Option<i32>>,
    outgoing_session: RefCell<Option<Rc<crate::note_session::NoteSession>>>,
}

impl Strip {
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
        let tabs = gtk::Box::new(gtk::Orientation::Vertical, 10);
        tabs.set_visible(initial.tabs_visible);
        root.add_overlay(&stack);
        let search_panel = gtk::Box::new(gtk::Orientation::Vertical, 6);
        search_panel.add_css_class("edge-search");
        search_panel.set_size_request(288, 300);
        let edge_search = gtk::SearchEntry::new();
        edge_search.set_placeholder_text(Some("Search active notes"));
        edge_search.set_hexpand(true);
        search_panel.append(&edge_search);
        let search_results = gtk::ListBox::new();
        search_results.set_activate_on_single_click(true);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&search_results)
            .build();
        search_panel.append(&scroll);
        let search_create = gtk::Button::with_label("No active notes");
        search_create.add_css_class("edge-create-result");
        search_panel.append(&search_create);
        stack.add_named(&search_panel, Some("search"));
        let reposition_panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
        reposition_panel.add_css_class("edge-search");
        reposition_panel.set_width_request(240);
        let instruction = gtk::Label::new(Some("Drag the strip along the edge."));
        instruction.set_wrap(true);
        reposition_panel.append(&instruction);
        let reposition_error = gtk::Label::new(None);
        reposition_error.set_wrap(true);
        reposition_error.set_max_width_chars(28);
        reposition_error.add_css_class("save-error");
        reposition_error.set_visible(false);
        reposition_panel.append(&reposition_error);
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let cancel_reposition = gtk::Button::with_label("Cancel");
        let done_reposition = gtk::Button::with_label("Done");
        for button in [&cancel_reposition, &done_reposition] {
            button.set_hexpand(true);
            actions.append(button);
        }
        reposition_panel.append(&actions);
        stack.add_named(&reposition_panel, Some("reposition"));
        let marker = gtk::Box::new(gtk::Orientation::Vertical, 6);
        marker.add_css_class("edge-marker");

        marker.set_can_target(false);
        strip_root.add_overlay(&marker);
        strip_root.add_overlay(&tabs);
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
            search_results,
            search_scroll: scroll,
            search_ids: RefCell::new(Vec::new()),
            search_create,
            pointer,
            pointer_origin: Cell::new(None),
            docked: RefCell::new(None),
            docked_offset: Cell::new(None),
            cancel_reposition,
            done_reposition,
            state: Cell::new(EdgeState::Dormant),
            generation: Cell::new(0),
            preview_offset: Cell::new(0.),
            strip_phase: Cell::new(StripPhase::Settled),
            pending_motion: Cell::new(None),
            deferred_hide: Cell::new(None),
            refresh_pending: Cell::new(false),
            marker_bounds: Cell::new((0, 0, 0, 0)),
            tab_motion: RefCell::new(Vec::new()),
            tab_hover: RefCell::new(Vec::new()),
            hovered_tab: Cell::new(None),
            marker_motion: motion::Motion::new(1.),
            utilities_motion: motion::Motion::new(0.),
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
        self.stack.set_visible(effects.panel.is_some() || outgoing);
        self.pointer
            .set_visible(effects.pointer_visible || outgoing);
        self.tabs.set_sensitive(effects.panel != Some("reposition"));
        self.strip_root
            .set_cursor_from_name((effects.panel == Some("reposition")).then_some("grab"));
        self.strip_window.set_layer(effects.strip_layer);
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
        if matches!(previous, EdgeState::Docked(_) | EdgeState::Search) && state == EdgeState::Tabs
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
                (motion::PANEL_MS, 0),
                self.motion_enabled(),
                move |value| {
                    if let Some(ui) = update.upgrade() {
                        ui.edge.stack.set_opacity(value);
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
        edge: ScreenEdge,
        index: usize,
        progress: f64,
        hover: f64,
    ) {
        let distance = (24. * (1. - progress)) as f32;
        let (x, y) = match edge {
            ScreenEdge::Left => (-distance, index as f32 * (tab.height_request() + 4) as f32),
            ScreenEdge::Right => (distance, index as f32 * (tab.height_request() + 4) as f32),
            ScreenEdge::Top => (index as f32 * (tab.width_request() + 4) as f32, -distance),
            ScreenEdge::Bottom => (index as f32 * (tab.width_request() + 4) as f32, distance),
        };
        let (width, height) = (tab.width_request() as f32, tab.height_request() as f32);
        let (cx, cy) = match edge {
            ScreenEdge::Left => (0., height / 2.),
            ScreenEdge::Right => (width, height / 2.),
            ScreenEdge::Top => (width / 2., 0.),
            ScreenEdge::Bottom => (width / 2., height),
        };
        let scale = (1. + (motion::HOVER_SCALE - 1.) * hover) as f32;
        let transform = gtk::gsk::Transform::new()
            .translate(&gtk::graphene::Point::new(x, y))
            .translate(&gtk::graphene::Point::new(cx, cy))
            .scale(scale, scale)
            .translate(&gtk::graphene::Point::new(-cx, -cy));
        fixed.set_child_transform(tab, Some(&transform));
        tab.set_opacity(progress);
    }
    fn hover_tab(self: &Rc<Self>, index: usize, hovered: bool) {
        let hovered =
            hovered && self.edge.state().is_revealed() && !self.edge.state().is_repositioning();
        let previous = self.edge.hovered_tab.get();
        if hovered {
            if previous == Some(index) {
                return;
            }
            self.edge.hovered_tab.set(Some(index));
            // Transfer both titles on the same frame clock. A late leave from
            // the old title must not restart its return animation.
            if let Some(previous) = previous {
                self.animate_tab_hover(previous, false);
            }
            self.animate_tab_hover(index, true);
        } else if previous == Some(index) {
            self.edge.hovered_tab.set(None);
            self.animate_tab_hover(index, false);
        }
    }
    fn animate_tab_hover(self: &Rc<Self>, index: usize, hovered: bool) {
        let Some(fixed) = self.edge.tabs.first_child().and_downcast::<gtk::Fixed>() else {
            return;
        };
        let mut child = fixed.first_child();
        for _ in 0..index {
            child = child.and_then(|tab| tab.next_sibling());
        }
        let Some(tab) = child else {
            return;
        };
        let Some(hover) = self.edge.tab_hover.borrow().get(index).cloned() else {
            return;
        };
        let enabled = self.motion_enabled() && !self.edge.state().is_repositioning();
        let weak = Rc::downgrade(self);
        let fixed = fixed.downgrade();
        let tab = tab.downgrade();
        hover.animate(
            &self.edge.strip_window,
            if hovered { 1. } else { 0. },
            (motion::HOVER_MS, 0),
            enabled,
            move |value| {
                if let Some(ui) = weak.upgrade()
                    && let Some(fixed) = fixed.upgrade()
                    && let Some(tab) = tab.upgrade()
                    && let Some(reveal) = ui.edge.tab_motion.borrow().get(index)
                {
                    Self::paint_tab(
                        &fixed,
                        &tab,
                        ui.config.borrow().edge,
                        index,
                        reveal.value(),
                        value,
                    );
                }
            },
            || {},
        );
    }
    fn finish_hover_motion(self: &Rc<Self>) {
        self.edge.hovered_tab.set(None);
        if let Some(fixed) = self.edge.tabs.first_child().and_downcast::<gtk::Fixed>() {
            let mut child = fixed.first_child();
            for (index, hover) in self.edge.tab_hover.borrow().iter().enumerate() {
                let Some(tab) = child else {
                    break;
                };
                child = tab.next_sibling();
                let value = if self.edge.state().is_revealed()
                    && !self.edge.state().is_repositioning()
                    && tab.state_flags().contains(gtk::StateFlags::PRELIGHT)
                {
                    1.
                } else {
                    0.
                };
                if value == 1. {
                    self.edge.hovered_tab.set(Some(index));
                }
                hover.jump(value);
                if let Some(reveal) = self.edge.tab_motion.borrow().get(index) {
                    Self::paint_tab(
                        &fixed,
                        &tab,
                        self.config.borrow().edge,
                        index,
                        reveal.value(),
                        value,
                    );
                }
            }
        }
    }
    fn animate_strip(self: &Rc<Self>) {
        let revealing = self.edge.strip_phase.get() == StripPhase::Revealing;
        let to = if revealing { 1. } else { 0. };
        let duration = if revealing {
            motion::REVEAL_MS
        } else {
            motion::COLLAPSE_MS
        };
        let motions = self.edge.tab_motion.borrow();
        // A reversed collapse resumes every tab at its painted value immediately.
        let reversing = revealing && motions.iter().any(|motion| motion.value() > 0.);
        let remaining = Rc::new(Cell::new(motions.len() + 2));
        let complete = |ui: &Rc<Self>| {
            let weak = Rc::downgrade(ui);
            let remaining = remaining.clone();
            move || {
                remaining.set(remaining.get() - 1);
                if remaining.get() == 0
                    && let Some(ui) = weak.upgrade()
                {
                    ui.finish_strip_motion();
                }
            }
        };
        let edge = self.config.borrow().edge;
        if let Some(fixed) = self.edge.tabs.first_child().and_downcast::<gtk::Fixed>() {
            let mut child = fixed.first_child();
            for (index, motion) in motions.iter().enumerate() {
                let Some(tab) = child else {
                    break;
                };
                child = tab.next_sibling();
                let weak_tab = tab.downgrade();
                let weak_fixed = fixed.downgrade();
                // Apply the current progress before the first frame or delayed start.
                let hover = self.edge.tab_hover.borrow().get(index).cloned();
                Self::paint_tab(
                    &fixed,
                    &tab,
                    edge,
                    index,
                    motion.value(),
                    hover.as_ref().map_or(0., |hover| hover.value()),
                );
                let order = if revealing {
                    index
                } else {
                    motions.len() - 1 - index
                };
                motion.animate(
                    &self.edge.strip_window,
                    to,
                    (
                        duration,
                        if reversing {
                            0
                        } else {
                            order as u32 * motion::STAGGER_MS
                        },
                    ),
                    self.motion_enabled(),
                    move |value| {
                        if let Some(tab) = weak_tab.upgrade()
                            && let Some(fixed) = weak_fixed.upgrade()
                        {
                            Self::paint_tab(
                                &fixed,
                                &tab,
                                edge,
                                index,
                                value,
                                hover.as_ref().map_or(0., |hover| hover.value()),
                            );
                        }
                    },
                    complete(self),
                );
            }
        }
        let marker = self.edge.marker.downgrade();
        self.edge.marker_motion.animate(
            &self.edge.strip_window,
            1. - to,
            (duration, 0),
            self.motion_enabled(),
            move |value| {
                if let Some(marker) = marker.upgrade() {
                    marker.set_opacity(value);
                }
            },
            complete(self),
        );
        let utilities = self.edge.tabs.last_child().map(|widget| widget.downgrade());
        self.edge.utilities_motion.animate(
            &self.edge.strip_window,
            to,
            (duration, 0),
            self.motion_enabled(),
            move |value| {
                if let Some(utilities) = utilities.as_ref().and_then(|weak| weak.upgrade()) {
                    utilities.set_opacity(value);
                }
            },
            complete(self),
        );
    }
    fn finish_strip_motion(self: &Rc<Self>) {
        let revealed = self.edge.state().is_revealed();
        let progress = if revealed { 1. } else { 0. };
        self.edge.strip_phase.set(StripPhase::Settled);
        if !revealed || self.edge.state().is_repositioning() {
            self.edge.hovered_tab.set(None);
            for hover in self.edge.tab_hover.borrow().iter() {
                hover.jump(0.);
            }
        }
        let motions = self.edge.tab_motion.borrow();
        if let Some(fixed) = self.edge.tabs.first_child().and_downcast::<gtk::Fixed>() {
            let mut child = fixed.first_child();
            for (index, motion) in motions.iter().enumerate() {
                motion.jump(progress);
                if let Some(tab) = child {
                    child = tab.next_sibling();
                    let hover = self
                        .edge
                        .tab_hover
                        .borrow()
                        .get(index)
                        .map_or(0., |hover| hover.value());
                    Self::paint_tab(
                        &fixed,
                        &tab,
                        self.config.borrow().edge,
                        index,
                        progress,
                        hover,
                    );
                }
            }
        }
        self.edge.marker_motion.jump(1. - progress);
        self.edge.utilities_motion.jump(progress);
        self.edge.marker.set_opacity(1. - progress);
        if let Some(utilities) = self.edge.tabs.last_child() {
            utilities.set_opacity(progress);
        }
        self.edge.marker.set_visible(!revealed);
        self.edge.tabs.set_visible(revealed);
        if !revealed {
            self.edge.strip_bounds.set(self.edge.marker_bounds.get());
            self.edge.strip_offset.set(None);
        }
        self.set_input();
        drop(motions);
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
        if let Some(outgoing) = self.edge.outgoing_panel.borrow_mut().take()
            && self.edge.stack.child_by_name("editor").as_ref() == Some(&outgoing)
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
        self.edge.pointer.set_visible(effects.pointer_visible);
        self.edge.root.queue_draw();
    }
    pub(super) fn finish_edge_motion(self: &Rc<Self>) {
        self.edge.pending_motion.set(None);
        self.finish_strip_motion();
        self.finish_hover_motion();
        self.finish_panel_motion();
    }
    pub(super) fn configure_docked_editor(self: &Rc<Self>, editor: &Rc<editor::Editor>) {
        editor.set_docked_size(editor::WIDTH, editor::HEIGHT);
        let drag = gtk::GestureDrag::new();
        drag.set_button(1);
        drag.set_propagation_phase(gtk::PropagationPhase::Capture);
        let start = Rc::new(Cell::new((editor::WIDTH, editor::HEIGHT)));
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
                    (width + dx as i32).clamp(editor::MIN_WIDTH, max_width),
                    (height + dy as i32).clamp(editor::MIN_HEIGHT, max_height),
                );
                ui.place_edge();
            }
        });
        editor.resize.add_controller(drag);
        let weak = Rc::downgrade(self);
        editor.connect_size_changed(move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_input();
            }
        });
    }
    pub(super) fn connect_edge(self: &Rc<Self>) {
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
                // Search stays open across focus changes, including focus-following
                // pointer movement and the handoff from the strip to its panel.
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
        for area in [&root, &strip_root] {
            let motion = gtk::EventControllerMotion::new();
            {
                let weak = Rc::downgrade(ui);
                motion.connect_enter(move |_, _, _| {
                    if let Some(ui) = weak.upgrade()
                        && ui.config.borrow().edge_hover_enabled
                    {
                        ui.schedule_reveal();
                    }
                });
            }
            {
                let weak = Rc::downgrade(ui);
                motion.connect_leave(move |_| {
                    if let Some(ui) = weak.upgrade() {
                        ui.schedule_hide();
                    }
                });
            }
            area.add_controller(motion);
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
        for target in [&window, &strip_window] {
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
        {
            let weak = Rc::downgrade(ui);
            window.connect_map(move |_| {
                if let Some(ui) = weak.upgrade() {
                    ui.set_input();
                }
            });
        }
        {
            let weak = Rc::downgrade(ui);
            window.connect_realize(move |window| {
                if let Some(surface) = window.surface() {
                    let weak = weak.clone();
                    surface.connect_layout(move |_, _, _| {
                        if let Some(ui) = weak.upgrade() {
                            ui.set_input();
                        }
                    });
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
        match event {
            EdgeEvent::FocusWindow | EdgeEvent::WindowFocused => {
                self.edge.window.set_keyboard_mode(effects.window_keyboard)
            }
            EdgeEvent::FocusStrip | EdgeEvent::StripFocused => self
                .edge
                .strip_window
                .set_keyboard_mode(effects.strip_keyboard),
            EdgeEvent::Visibility => {
                self.edge.strip_window.set_visible(effects.strip_visible);
                self.edge.window.set_visible(effects.window_visible);
            }
            _ => self.edge.apply(effects),
        }
    }
    pub(super) fn refresh_tabs(self: &Rc<Self>) {
        // Saves can arrive during hover motion. Keep the current widgets and input
        // bounds until that motion settles, then apply the latest note data once.
        if self.edge.strip_phase.get() != StripPhase::Settled {
            self.edge.refresh_pending.set(true);
            return;
        }
        self.edge.refresh_pending.set(false);
        self.edge.hovered_tab.set(None);
        self.edge.tab_hover.borrow_mut().clear();
        while let Some(child) = self.edge.tabs.first_child() {
            self.edge.tabs.remove(&child);
        }
        while let Some(child) = self.edge.marker.first_child() {
            self.edge.marker.remove(&child);
        }
        let config = self.config.borrow().clone();
        let notes = match self.storage.load_deck_notes(4, self.edge_sort.get()) {
            Ok(notes) => notes,
            Err(error) => {
                self.error(&format!("Could not load edge notes: {error}"));
                return;
            }
        };
        self.deck.borrow_mut().replace_notes(notes);
        let notes: Vec<_> = self.deck.borrow().notes().iter().take(4).cloned().collect();
        let horizontal = matches!(config.edge, ScreenEdge::Top | ScreenEdge::Bottom);
        let orientation = if horizontal {
            gtk::Orientation::Horizontal
        } else {
            gtk::Orientation::Vertical
        };
        self.edge.tabs.set_orientation(orientation);
        self.edge.marker.set_orientation(orientation);
        // Measure the upright titles, but cap their contribution so a long title
        // cannot widen the whole strip. Both orientations use a slim 24px depth.
        let tab_width = if horizontal {
            let title_width = notes
                .iter()
                .map(|note| {
                    let layout = self
                        .edge
                        .tabs
                        .create_pango_layout(Some(&note.title.to_uppercase()));
                    layout.set_font_description(Some(&gtk::pango::FontDescription::from_string(
                        "Inter Semi-Bold 11px",
                    )));
                    layout.pixel_size().0
                })
                .max()
                .unwrap_or(0);
            let (screen_width, _) = self.note_limits();
            let available = ((screen_width - 110) / 4).max(24);
            let width =
                if self.edge.state().is_revealed() && self.edge.horizontal_tab_width.get() > 0 {
                    self.edge.horizontal_tab_width.get()
                } else {
                    (title_width + 16).clamp(80, 112)
                }
                .min(available);
            self.edge.horizontal_tab_width.set(width);
            width
        } else {
            self.edge.horizontal_tab_width.set(0);
            24
        };
        let tab_height = if horizontal { 24 } else { 80 };
        let fixed = glib::Object::new::<TabLayout>().upcast::<gtk::Fixed>();
        fixed.set_overflow(gtk::Overflow::Visible);
        fixed.set_size_request(
            if horizontal {
                (notes.len() as i32 * (tab_width + 4) - 4).max(0)
            } else {
                tab_width
            },
            if horizontal {
                tab_height
            } else {
                (notes.len() as i32 * (tab_height + 4) - 4).max(0)
            },
        );
        for (index, note) in notes.iter().enumerate() {
            let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
            dot.add_css_class("edge-pill");
            appearance::set_color(&dot, note.color);
            self.edge.marker.append(&dot);
            let tab = gtk::Button::new();
            tab.add_css_class("note-tab");
            tab.set_widget_name(&format!("note-{:?}", note.id));
            appearance::set_color(&tab, note.color);
            let title = if note.title.trim().is_empty() {
                "Untitled note"
            } else {
                &note.title
            };
            tab.set_tooltip_text(Some(title));
            let label = gtk::Label::new(Some(&title.to_uppercase()));
            label.add_css_class("tab-title");
            label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            label.set_width_chars(1);
            label.set_max_width_chars(10);
            label.set_xalign(0.5);
            if horizontal {
                label.set_max_width_chars(1);
                tab.set_child(Some(&label));
            } else {
                label.set_size_request(72, 16);
                let text = gtk::Fixed::new();
                text.set_size_request(22, 78);
                text.put(&label, 0., 0.);
                let transform = if config.edge == ScreenEdge::Right {
                    gtk::gsk::Transform::new()
                        .translate(&gtk::graphene::Point::new(19., 3.))
                        .rotate(90.)
                } else {
                    gtk::gsk::Transform::new()
                        .translate(&gtk::graphene::Point::new(3., 75.))
                        .rotate(-90.)
                };
                text.set_child_transform(&label, Some(&transform));
                tab.set_child(Some(&text));
            }
            tab.set_size_request(tab_width, tab_height);
            let (x, y) = if horizontal {
                (index as f64 * f64::from(tab_width + 4), 0.)
            } else {
                (0., index as f64 * f64::from(tab_height + 4))
            };
            fixed.put(&tab, x, y);
            self.edge
                .tab_hover
                .borrow_mut()
                .push(Rc::new(motion::Motion::new(0.)));
            let weak = Rc::downgrade(self);
            tab.connect_state_flags_changed(move |tab, previous| {
                let hovered = tab.state_flags().contains(gtk::StateFlags::PRELIGHT);
                if hovered != previous.contains(gtk::StateFlags::PRELIGHT)
                    && let Some(ui) = weak.upgrade()
                {
                    ui.hover_tab(index, hovered);
                }
            });
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
        if notes.is_empty() {
            let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
            dot.add_css_class("edge-pill");
            appearance::set_color(&dot, NoteColor::Yellow);
            self.edge.marker.append(&dot);
        }
        if !notes.is_empty() {
            self.edge.tabs.append(&fixed);
        }
        let utilities = gtk::Box::new(orientation, 4);
        utilities.add_css_class("edge-utilities");
        // GTK has no CSS overflow property. Clip utility feedback to the capsule.
        utilities.set_overflow(gtk::Overflow::Hidden);
        for (name, label) in [
            ("plus", "New note"),
            ("search", "Search notes"),
            ("grid", "All Notes"),
        ] {
            let button = ui_assets::edge_icon(name, label);
            button.add_css_class("strip-control");
            button.set_size_request(24, 24);
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
        ui_assets::refresh_icons(
            self.edge.tabs.upcast_ref(),
            config.theme == config::Theme::Playful,
            false,
        );
        self.edge.tab_motion.replace(
            notes
                .iter()
                .map(|_| {
                    motion::Motion::new(if self.edge.state().is_revealed() {
                        1.
                    } else {
                        0.
                    })
                })
                .collect(),
        );
        if self.edge.state().is_searching() {
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
        self.place_edge();
        self.transition_edge(EdgeEvent::FocusWindow);
        self.edge.edge_search.grab_focus();
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
            dot.add_css_class("edge-pill");
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
                "No matches. Create a note from this search"
            });
        // Rebuilt rows need their first allocation before scrolling to them.
        let weak = Rc::downgrade(self);
        self.edge.search_results.add_tick_callback(move |_, _| {
            if let Some(ui) = weak.upgrade()
                && !ui.keep_search_selection_visible()
            {
                return glib::ControlFlow::Continue;
            }
            // Tick callbacks run before layout; the scroller may reset its
            // adjustment during that layout. Apply the final position afterward.
            let weak = weak.clone();
            glib::idle_add_local_once(move || {
                if let Some(ui) = weak.upgrade() {
                    ui.keep_search_selection_visible();
                }
            });
            glib::ControlFlow::Break
        });
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
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(140), move || {
            if let Some(ui) = weak.upgrade()
                && ui.edge.generation.get() == generation
                && ui.config.borrow().edge_hover_enabled
                && ui.edge.state() == EdgeState::Dormant
            {
                ui.reveal();
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
        let weak = Rc::downgrade(self);
        glib::timeout_add_local_once(Duration::from_millis(350), move || {
            if let Some(ui) = weak.upgrade()
                && ui.edge.generation.get() == generation
            {
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
                window.remove_css_class(class);
            }
            window.add_css_class(edge);
            window.add_css_class(theme);
        }
    }

    pub(super) fn place_edge(self: &Rc<Self>) {
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
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Right, false);
            window.set_anchor(Edge::Bottom, false);
            root.set_size_request(screen_width, screen_height);
            window.set_default_size(screen_width, screen_height);
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
        let position = |widget: &gtk::Widget, x, y| {
            widget.set_halign(gtk::Align::Start);
            widget.set_valign(gtk::Align::Start);
            widget.set_margin_start(x);
            widget.set_margin_top(y);
        };
        // Measure without placement margins, which otherwise feed back into size.
        position(self.edge.tabs.upcast_ref(), tx, ty);
        let (mw, mh) = measure(self.edge.marker.upcast_ref());
        let (mx, my) = match config.edge {
            ScreenEdge::Left => (0, (offset + (th - mh) / 2).clamp(0, screen_height - mh)),
            ScreenEdge::Right => (
                screen_width - mw,
                (offset + (th - mh) / 2).clamp(0, screen_height - mh),
            ),
            ScreenEdge::Top => ((offset + (tw - mw) / 2).clamp(0, screen_width - mw), 0),
            ScreenEdge::Bottom => (
                (offset + (tw - mw) / 2).clamp(0, screen_width - mw),
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
                    self.edge
                        .pointer_origin
                        .set(Some(offset + index * (length + 4) + length / 2));
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
            let (max_width, max_height) = if horizontal {
                (screen_width, screen_height - th - 10)
            } else {
                (screen_width - tw - 10, screen_height)
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
                editor.docked_size()
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
                ScreenEdge::Left => (tw + 10, along),
                ScreenEdge::Right => (screen_width - tw - 10 - width, along),
                ScreenEdge::Top => (along, th + 10),
                ScreenEdge::Bottom => (along, screen_height - th - 10 - height),
            };
            position(self.edge.stack.upcast_ref(), x, y);
            self.edge.panel_bounds.set((x, y, width, height));
        }
        let active = self
            .edge
            .state()
            .docked_id()
            .map(|id| format!("note-{id:?}"));
        if let Some(group) = self.edge.tabs.first_child() {
            let mut child = group.first_child();
            while let Some(tab) = child {
                if active.as_deref() == Some(tab.widget_name().as_str()) {
                    tab.add_css_class("edge-active");
                } else {
                    tab.remove_css_class("edge-active");
                }
                child = tab.next_sibling();
            }
        }
        if let Some(search) = self
            .edge
            .tabs
            .last_child()
            .and_then(|group| group.first_child())
            .and_then(|first| first.next_sibling())
        {
            if self.edge.state().is_searching() {
                search.add_css_class("edge-active");
            } else {
                search.remove_css_class("edge-active");
            }
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
    pub(super) fn note_limits(&self) -> (i32, i32) {
        gtk::gdk::Display::default()
            .and_then(|display| {
                display
                    .monitors()
                    .item(0)
                    .and_downcast::<gtk::gdk::Monitor>()
            })
            .map(|monitor| {
                let geometry = monitor.geometry();
                (
                    geometry.width().max(editor::MIN_WIDTH),
                    geometry.height().max(editor::MIN_HEIGHT),
                )
            })
            .unwrap_or((720, 500))
    }
    pub(super) fn docked_pointer(&self, gesture: &gtk::GestureDrag) -> Option<(f64, f64)> {
        // The panel surface has a fixed screen origin, including during resize.
        gesture.current_event()?.position()
    }
    fn strip_input_bounds(&self) -> (i32, i32, i32, i32) {
        let (x, y, width, height) = self.edge.strip_bounds.get();
        let expanded = (self.edge.state().is_revealed()
            || self.edge.strip_phase.get() == StripPhase::Collapsing)
            && !self.edge.state().is_repositioning();
        let pad = if expanded { motion::HOVER_PAD } else { 0 };
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
        for (window, bounds) in [
            (&self.edge.strip_window, Some(self.strip_input_bounds())),
            (
                &self.edge.window,
                effects.panel_input.then(|| self.edge.panel_bounds.get()),
            ),
        ] {
            if let Some(surface) = window.surface() {
                let region = gtk::cairo::Region::create();
                if let Some((x, y, width, height)) = bounds {
                    let _ =
                        region.union_rectangle(&gtk::cairo::RectangleInt::new(x, y, width, height));
                }
                surface.set_input_region(Some(&region));
            }
        }
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
        let origin = if self.edge.outgoing_panel.borrow().is_some() {
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
                config::Theme::Paper if config.white_paper => (1., 1., 1.),
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
        // Replacement requires the coordinator's successful close first.
        assert_eq!(transition(state, EdgeEvent::Search, policy()), expected);
        assert_eq!(transition(state, EdgeEvent::Reposition, policy()), expected);
        assert_eq!(
            transition(state, EdgeEvent::Dock(NoteId::new(2)), policy()),
            expected
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
        let ui = crate::build(&app).unwrap();
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
        let progress = intermediate("reveal", &|| ui.edge.tab_motion.borrow()[0].value());
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
        assert_eq!(ui.edge.tab_motion.borrow()[0].value(), progress);
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
        let before = intermediate("collapse", &|| ui.edge.tab_motion.borrow()[3].value());
        assert!(
            before > 0. && before < 1.,
            "collapse must have intermediate frames: {before}"
        );
        session.body.set_text("Saved during collapse");
        assert!(session.flush());
        assert_eq!(ui.edge.tab_motion.borrow()[3].value(), before);
        assert_eq!(ui.edge.strip_phase.get(), StripPhase::Collapsing);
        assert_eq!(ui.edge.strip_bounds.get(), expanded);
        assert!(ui.edge.refresh_pending.get());
        ui.schedule_reveal();
        assert_eq!(ui.edge.state(), EdgeState::Tabs);
        assert_eq!(
            ui.edge.tab_motion.borrow()[3].value(),
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
        let before = ui.edge.tab_motion.borrow()[0].value();
        ui.schedule_hide();
        settle(15);
        ui.schedule_reveal();
        assert!(ui.edge.tab_motion.borrow()[0].value() >= before);
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
        assert!(
            ui.edge
                .tab_motion
                .borrow()
                .iter()
                .all(|motion| motion.value() == 1.)
        );
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
        assert!(
            ui.edge
                .tab_motion
                .borrow()
                .iter()
                .all(|motion| !motion.is_running())
        );
        ui.edge.window.close();
        ui.edge.strip_window.close();
    }
    #[test]
    #[ignore = "requires Wayland and disposable XDG directories"]
    fn hover_pop_preserves_clicks_and_surface_geometry() {
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
        let ui = crate::build(&app).unwrap();
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
        let tab = ui.edge.tabs.first_child().unwrap().first_child().unwrap();
        tab.set_state_flags(gtk::StateFlags::PRELIGHT, false);
        settle(45);
        let progress = ui.edge.tab_hover.borrow()[0].value();
        assert!(
            progress > 0. && progress < 1.,
            "hover must have intermediate progress: {progress}"
        );
        tab.unset_state_flags(gtk::StateFlags::PRELIGHT);
        assert_eq!(
            ui.edge.tab_hover.borrow()[0].value(),
            progress,
            "leave must reverse from current progress"
        );
        settle(180);
        assert_eq!(ui.edge.tab_hover.borrow()[0].value(), 0.);
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
                tab.set_state_flags(gtk::StateFlags::PRELIGHT, false);
                settle(40);
                assert_eq!(ui.edge.tab_hover.borrow()[0].value(), 1.);
                assert!(
                    !ui.edge.tab_hover.borrow()[0].is_running(),
                    "reduced motion must jump"
                );
                let grown = tab.compute_bounds(&ui.edge.strip_root).unwrap();
                assert!((grown.width() / base.width() - motion::HOVER_SCALE as f32).abs() < 0.001);
                assert!(
                    (grown.height() / base.height() - motion::HOVER_SCALE as f32).abs() < 0.001
                );
                let (anchor_before, anchor_after) = match edge {
                    ScreenEdge::Left => (base.x(), grown.x()),
                    ScreenEdge::Right => (base.x() + base.width(), grown.x() + grown.width()),
                    ScreenEdge::Top => (base.y(), grown.y()),
                    ScreenEdge::Bottom => (base.y() + base.height(), grown.y() + grown.height()),
                };
                assert!(
                    (anchor_before - anchor_after).abs() < 0.01,
                    "hover must stay attached to {edge:?}"
                );
                assert_eq!(
                    (fixed.width(), fixed.height()),
                    (fixed.width_request(), fixed.height_request())
                );
                assert_eq!(
                    ui.strip_input_bounds(),
                    input,
                    "hover must not resize the input region per frame"
                );
                assert!(grown.x() >= input.0 as f32 && grown.y() >= input.1 as f32);
                assert!(grown.x() + grown.width() <= (input.0 + input.2) as f32 + 0.01);
                assert!(grown.y() + grown.height() <= (input.1 + input.3) as f32 + 0.01);
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
                    "enlarged title must receive clicks"
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
        let settings = gtk::Settings::default().unwrap();
        settings.set_gtk_enable_animations(true);
        let tab = ui.edge.tabs.first_child().unwrap().first_child().unwrap();
        tab.set_state_flags(gtk::StateFlags::PRELIGHT, false);
        settle(35);
        settings.set_gtk_enable_animations(false);
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

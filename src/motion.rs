use adw::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub const REVEAL_MS: u32 = 160;
pub const COLLAPSE_MS: u32 = 110;
pub const PANEL_MS: u32 = 120;
pub const HOVER_MS: u32 = 120;
pub const HOVER_PAD: i32 = 4;

struct Running {
    value: Cell<f64>,
    follow_target: Cell<Option<f64>>,
    generation: Cell<u64>,
    animation: RefCell<Option<adw::TimedAnimation>>,
    start: RefCell<
        Option<(
            gtk::TickCallbackId,
            glib::WeakRef<gtk::Widget>,
            glib::SignalHandlerId,
        )>,
    >,
}
pub struct Motion(Rc<Running>);

impl Running {
    fn cancel_start(&self) {
        if let Some((tick, widget, unmap)) = self.start.borrow_mut().take() {
            tick.remove();
            if let Some(widget) = widget.upgrade() {
                widget.disconnect(unmap);
            }
        }
    }
}

impl Motion {
    pub fn new(value: f64) -> Self {
        Self(Rc::new(Running {
            value: Cell::new(value),
            follow_target: Cell::new(None),
            generation: Cell::new(0),
            animation: RefCell::new(None),
            start: RefCell::new(None),
        }))
    }
    pub fn is_running(&self) -> bool {
        self.0.animation.borrow().is_some() || self.0.follow_target.get().is_some()
    }
    pub fn value(&self) -> f64 {
        self.0.value.get()
    }
    pub fn cancel(&self) {
        self.0.follow_target.set(None);
        self.0
            .generation
            .set(self.0.generation.get().wrapping_add(1));
        self.0.cancel_start();
        if let Some(animation) = self.0.animation.borrow_mut().take()
            && animation.state() == adw::AnimationState::Playing
        {
            animation.pause();
        }
    }
    pub fn jump(&self, value: f64) {
        self.cancel();
        self.0.value.set(value);
    }
    // Pointer tracking keeps one frame callback alive. New positions change its
    // destination without cancelling a frame or restarting an easing timeline.
    pub fn follow(
        &self,
        widget: &impl IsA<gtk::Widget>,
        to: f64,
        enabled: bool,
        update: impl Fn(f64) + 'static,
    ) {
        if enabled && widget.is_mapped() && self.0.follow_target.get().is_some() {
            self.0.follow_target.set(Some(to));
            return;
        }
        if !self.is_running() && self.value() == to {
            return;
        }
        self.cancel();
        if !enabled || !widget.is_mapped() || self.value() == to {
            self.0.value.set(to);
            update(to);
            return;
        }
        self.0.follow_target.set(Some(to));
        let update = Rc::new(update);
        let on_unmap = update.clone();
        let weak = Rc::downgrade(&self.0);
        let unmap = widget.connect_unmap(move |_| {
            if let Some(running) = weak.upgrade()
                && let Some(target) = running.follow_target.take()
            {
                running.cancel_start();
                running.value.set(target);
                on_unmap(target);
            }
        });
        // An idle frame clock can still contain an old timestamp. Start at the
        // pointer event, otherwise the first frame consumes most of the hover.
        let previous = Cell::new(glib::monotonic_time());
        let weak = Rc::downgrade(&self.0);
        let tick = widget.add_tick_callback(move |widget, clock| {
            let Some(running) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            let Some(target) = running.follow_target.get() else {
                return glib::ControlFlow::Break;
            };
            let elapsed = (clock.frame_time() - previous.replace(clock.frame_time()))
                .clamp(0, 50_000) as f64
                / 1_000.;
            // Frame-rate independent response: 99% of the distance in HOVER_MS.
            let remaining = (-elapsed * std::f64::consts::LN_10 * 2. / f64::from(HOVER_MS)).exp();
            let value = target + (running.value.get() - target) * remaining;
            let settled = (value - target).abs() < 0.001;
            running.value.set(if settled { target } else { value });
            if settled {
                running.follow_target.set(None);
                if let Some((_, _, unmap)) = running.start.borrow_mut().take() {
                    widget.disconnect(unmap);
                }
            }
            update(running.value.get());
            if settled {
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
        self.0
            .start
            .replace(Some((tick, widget.as_ref().downgrade(), unmap)));
    }
    pub fn animate(
        &self,
        widget: &impl IsA<gtk::Widget>,
        to: f64,
        duration: u32,
        enabled: bool,
        update: impl Fn(f64) + 'static,
        done: impl Fn() + 'static,
    ) {
        self.cancel();
        let duration = if enabled { duration } else { 0 };
        let from = self.value();
        if duration == 0 || from == to || !widget.is_mapped() {
            self.0.value.set(to);
            update(to);
            done();
            return;
        }
        update(from);
        let generation = self.0.generation.get();
        let weak = Rc::downgrade(&self.0);
        let target = adw::CallbackAnimationTarget::new(move |eased| {
            if let Some(running) = weak.upgrade()
                && running.generation.get() == generation
            {
                let value = from + (to - from) * eased;
                running.value.set(value);
                update(value);
            }
        });
        let animation = adw::TimedAnimation::new(widget, 0., 1., duration, target);
        animation.set_easing(adw::Easing::EaseOutCubic);
        let weak = Rc::downgrade(&self.0);
        animation.connect_done(move |_| {
            if let Some(running) = weak.upgrade()
                && running.generation.get() == generation
            {
                running.cancel_start();
                running.animation.borrow_mut().take();
                done();
            }
        });
        self.0.animation.replace(Some(animation));
        // Start on a fresh frame-clock tick. Starting from an idle clock can
        // consume the reveal before GTK has allocated its first visible frame.
        let weak = Rc::downgrade(&self.0);
        let unmap = widget.connect_unmap(move |_| {
            if let Some(running) = weak.upgrade() {
                let animation = running.animation.borrow().clone();
                if let Some(animation) = animation {
                    animation.skip();
                }
            }
        });
        let weak = Rc::downgrade(&self.0);
        let tick = widget.add_tick_callback(move |widget, _| {
            if let Some(running) = weak.upgrade() {
                if let Some((_, _, unmap)) = running.start.borrow_mut().take() {
                    widget.disconnect(unmap);
                }
                let animation = running.animation.borrow().clone();
                if let Some(animation) = animation {
                    animation.play();
                }
            }
            glib::ControlFlow::Break
        });
        self.0
            .start
            .replace(Some((tick, widget.as_ref().downgrade(), unmap)));
    }
}
impl Drop for Motion {
    fn drop(&mut self) {
        self.cancel();
    }
}

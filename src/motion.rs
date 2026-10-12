use adw::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub const REVEAL_MS: u32 = 160;
pub const COLLAPSE_MS: u32 = 110;
pub const PANEL_MS: u32 = 120;
pub const STAGGER_MS: u32 = 20;
pub const HOVER_MS: u32 = 120;
pub const HOVER_PAD: i32 = 4;

struct Running {
    value: Cell<f64>,
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

// The stagger uses the same frame clock as the animation, rather than delayed
// GLib timers that can bunch up when the first frame is expensive to render.
fn staggered_progress(eased: f64, duration: u32, delay: u32) -> f64 {
    if delay == 0 {
        return eased;
    }
    let linear = 1. - (1. - eased).cbrt();
    let local = ((linear * f64::from(duration + delay) - f64::from(delay)) / f64::from(duration))
        .clamp(0., 1.);
    1. - (1. - local).powi(3)
}

impl Motion {
    pub fn new(value: f64) -> Self {
        Self(Rc::new(Running {
            value: Cell::new(value),
            generation: Cell::new(0),
            animation: RefCell::new(None),
            start: RefCell::new(None),
        }))
    }
    pub fn is_running(&self) -> bool {
        self.0.animation.borrow().is_some()
    }
    pub fn value(&self) -> f64 {
        self.0.value.get()
    }
    pub fn cancel(&self) {
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
    pub fn animate(
        &self,
        widget: &impl IsA<gtk::Widget>,
        to: f64,
        timing: (u32, u32),
        enabled: bool,
        update: impl Fn(f64) + 'static,
        done: impl Fn() + 'static,
    ) {
        self.cancel();
        let (duration, delay) = if enabled { timing } else { (0, 0) };
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
                let value = from + (to - from) * staggered_progress(eased, duration, delay);
                running.value.set(value);
                update(value);
            }
        });
        let animation = adw::TimedAnimation::new(widget, 0., 1., duration + delay, target);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stagger_preserves_the_delay_and_ease_out_duration() {
        for delay in [0, 20, 40, 60] {
            let at = |ms: u32| {
                let linear = f64::from(ms) / f64::from(REVEAL_MS + delay);
                staggered_progress(1. - (1. - linear).powi(3), REVEAL_MS, delay)
            };
            assert!(at(delay).abs() < 1e-12);
            assert!((at(delay + 80) - 0.875).abs() < 1e-12);
            assert_eq!(at(delay + REVEAL_MS), 1.);
        }
    }
}

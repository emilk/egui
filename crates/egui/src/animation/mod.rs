mod easing;
mod lerp;
mod manager;

use std::marker::PhantomData;

pub use crate::animation::easing::Easing;
pub use crate::animation::lerp::Lerp;
use crate::Id;
use epaint::mutex::Mutex;
pub use manager::AnimationManager;
use std::sync::Arc;

pub struct AnimationRef<L> {
    pub id: Id,
    _d: PhantomData<L>,
}

impl<L> AnimationRef<L> {
    pub fn new(id: Id) -> AnimationRef<L> {
        AnimationRef {
            id,
            _d: Default::default(),
        }
    }
}

pub struct Animation<L: Lerp + Send + Sync> {
    pub(crate) time: f64,
    pub(crate) animation_time: f32,
    pub(crate) inner: AnimationImpl<L>,
    pub(crate) link: Arc<Mutex<AnimationImpl<L>>>,
}

impl<L: Lerp + Send + Sync> Animation<L> {
    /// Removes any current animation and sets a static value.
    pub fn set_value(&mut self, value: L) -> &mut Self {
        self.inner.start = 0.0;
        self.inner.duration = 0.0;
        self.inner.source = value.clone();
        self.inner.target = value;
        self
    }

    /// Runs the function when the animation has been completed.
    pub fn when_done(&mut self, func: impl FnOnce(&mut Self)) {
        if self.is_finished() {
            func(self)
        }
    }

    /// Gets the current position of the animation
    pub fn get_pos(&self) -> f64 {
        if self.inner.duration == 0.0 {
            1.0
        } else {
            (self.time - self.inner.start) / self.inner.duration
        }
        .clamp(0.0, 1.0)
    }

    /// Gets the current value of the animation
    pub fn get_value(&self) -> L {
        let time_t = self.get_pos();
        let eased_t = self.inner.easing.apply(time_t);
        self.inner.source.lerp(&self.inner.target, eased_t as f32)
    }

    /// Anchors the current position and sets a new target
    pub fn new_target(&mut self, target: L) -> &mut Self {
        self.anchor_source();
        self.with_target(target);
        self
    }

    /// Sets the from value to the current value.
    pub fn anchor_source(&mut self) -> &mut Self {
        self.inner.source = self.get_value();
        self
    }

    /// Sets the to value to the current value.
    pub fn anchor_target(&mut self) -> &mut Self {
        self.inner.target = self.get_value();
        self
    }

    /// Overwrites the current source value
    pub fn with_source(&mut self, source: L) -> &mut Self {
        self.inner.source = source;
        self
    }

    /// Overwrites the current target value
    pub fn with_target(&mut self, target: L) -> &mut Self {
        self.inner.target = target;
        self
    }

    /// Overwrites the current easing
    pub fn with_easing(&mut self, easing: Easing) -> &mut Self {
        self.inner.easing = easing;
        self
    }

    /// Starts a new animation to a new target.
    pub fn start(&mut self) {
        self.start_with_speed(1.0);
    }

    /// Starts a new animation with a given speed modifier.
    pub fn start_with_speed(&mut self, speed: f32) {
        self.inner.start = self.time;
        self.inner.duration = speed as f64 * self.animation_time as f64;
    }

    /// Checks if the animation is currently moving
    pub fn is_active(&self) -> bool {
        let pos = self.get_pos();
        pos > 0.0 && pos < 1.0
    }

    /// Checks if the animation has started moving.
    pub fn has_started(&self) -> bool {
        self.get_pos() > 0.0
    }

    /// Checks if the animation has finished.
    pub fn is_finished(&self) -> bool {
        self.get_pos() >= 1.0
    }

    pub fn target(&self) -> &L {
        &self.inner.target
    }

    pub fn source(&self) -> &L {
        &self.inner.source
    }
}

impl<L: Lerp + Send + Sync> Drop for Animation<L> {
    fn drop(&mut self) {
        *self.link.lock() = self.inner.clone();
    }
}

impl<L: Lerp + Send + Sync + Default> Animation<L> {
    /// Resets any current animation and sets it to the default value.
    pub fn reset(&mut self) -> &mut Self {
        self.set_value(L::default())
    }
}

#[derive(Clone)]
pub struct AnimationImpl<L: Lerp + Send + Sync> {
    pub source: L,
    pub target: L,
    pub easing: Easing,
    // seconds time
    pub(crate) start: f64,
    pub(crate) duration: f64,
}

impl<L: Lerp + Send + Sync> AnimationImpl<L> {
    pub fn new(from: L, to: L, easing: Easing) -> AnimationImpl<L> {
        AnimationImpl {
            source: from,
            target: to,
            easing,
            start: 0.0,
            duration: 0.0,
        }
    }

    pub fn simple(value: L) -> AnimationImpl<L> {
        AnimationImpl {
            source: value.clone(),
            target: value,
            easing: Easing::Linear,
            start: 0.0,
            duration: 0.0,
        }
    }
}

impl<L: Lerp + Send + Sync + Default> Default for AnimationImpl<L> {
    fn default() -> Self {
        Self::new(L::default(), L::default(), Easing::EaseInOut)
    }
}

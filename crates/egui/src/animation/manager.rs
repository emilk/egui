use ahash::AHashMap;
use std::any::{type_name, Any};
use std::sync::Arc;
use epaint::mutex::Mutex;
use crate::animation::{Animation, AnimationImpl};
use crate::animation::lerp::Lerp;
use crate::{Id};

#[derive(Default)]
pub struct AnimationManager {
    inner: Mutex<AnimationManagerInner>,
}

impl AnimationManager {
    pub fn get<L: Lerp + Send + Sync + Default>(&self, id: Id) -> Animation<L> {
        self.get_or(id, AnimationImpl::default)
    }

    pub fn get_or<L: Lerp + Send + Sync>(
        &self,
        id: Id,
        default: impl FnOnce() -> AnimationImpl<L>,
    ) -> Animation<L> {
        let mut inner = self.inner.lock();
        let arc = inner.animations.entry(id).or_insert_with(|| {
            let inner = default();
            Box::new(Arc::new(Mutex::new(inner)))
        });
        let any = &(**arc);
        let link = any
            .downcast_ref::<Arc<Mutex<AnimationImpl<L>>>>()
            .unwrap_or_else(|| panic!("Wrong type <{}> for animation at {id:?}", type_name::<L>()))
            .clone();

        let animation = (link.lock()).clone();
        let animation = Animation {
            time: inner.time,
            animation_time: inner.animation_time,
            inner: animation,
            link,
        };

        if !inner.any_active {
            inner.any_active = animation.is_active();
        }

        animation
    }

    pub fn begin_frame(&self, time: f64, animation_time: f32) {
        let mut inner = self.inner.lock();
        inner.time = time;
        inner.animation_time = animation_time;
        inner.any_active = false;
    }

    pub fn wants_repaint(&self) -> bool {
        self.inner.lock().any_active
    }
}

#[derive(Default)]
struct AnimationManagerInner {
    animations: AHashMap<Id, Box<dyn Any + Send + Sync>>,
    any_active: bool,
    animation_time: f32,
    time: f64,
}
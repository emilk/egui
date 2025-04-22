//! Caches for preventing the same value from being recomputed every frame.
//!
//! Computing the same thing each frame can be expensive,
//! so often you want to save the result from the previous frame and reuse it.
//!
//! Enter [`FrameCache`]: it caches the results of a computation for one frame.
//! If it is still used next frame, it is not recomputed.
//! If it is not used next frame, it is evicted from the cache to save memory.
//!
//! You can access egui's caches via [`crate::Memory::caches`],
//! found with [`crate::Context::memory_mut`].

mod cache_storage;
mod cache_trait;
mod frame_cache;
mod frame_publisher;

pub use cache_storage::CacheStorage;
pub use cache_trait::CacheTrait;
pub use frame_cache::{ComputerMut, FrameCache};
pub use frame_publisher::FramePublisher;

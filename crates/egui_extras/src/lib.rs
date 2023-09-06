//! This is a crate that adds some features on top top of [`egui`](https://github.com/emilk/egui).
//!
//! This crate are for experimental features, and features that require big dependencies that does not belong in `egui`.
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]
#![forbid(unsafe_code)]

#[cfg(feature = "chrono")]
mod datepicker;

pub mod image;
mod layout;
pub mod loaders;
mod sizing;
mod strip;
mod table;

#[cfg(feature = "chrono")]
pub use crate::datepicker::DatePickerButton;

pub use crate::image::RetainedImage;
pub(crate) use crate::layout::StripLayout;
pub use crate::sizing::Size;
pub use crate::strip::*;
pub use crate::table::*;

// ---------------------------------------------------------------------------

mod profiling_scopes {
    #![allow(unused_macros)]
    #![allow(unused_imports)]

    /// Profiling macro for feature "puffin"
    macro_rules! profile_function {
        ($($arg: tt)*) => {
            #[cfg(not(target_arch = "wasm32"))] // Disabled on web because of the coarse 1ms clock resolution there.
            #[cfg(feature = "puffin")]
            puffin::profile_function!($($arg)*);
        };
    }
    pub(crate) use profile_function;

    /// Profiling macro for feature "puffin"
    macro_rules! profile_scope {
        ($($arg: tt)*) => {
            #[cfg(not(target_arch = "wasm32"))] // Disabled on web because of the coarse 1ms clock resolution there.
            #[cfg(feature = "puffin")]
            puffin::profile_scope!($($arg)*);
        };
    }
    pub(crate) use profile_scope;
}

#[allow(unused_imports)]
pub(crate) use profiling_scopes::*;

// ---------------------------------------------------------------------------

/// Log an error with either `log` or `eprintln`
macro_rules! log_err {
    ($fmt: literal, $($arg: tt)*) => {{
        #[cfg(feature = "log")]
        log::error!($fmt, $($arg)*);

        #[cfg(not(feature = "log"))]
        eprintln!(
            concat!("egui_extras: ", $fmt), $($arg)*
        );
    }};
}
pub(crate) use log_err;

/// Panic in debug builds, log otherwise.
macro_rules! log_or_panic {
    ($fmt: literal, $($arg: tt)*) => {{
        if cfg!(debug_assertions) {
            panic!($fmt, $($arg)*);
        } else {
            $crate::log_err!($fmt, $($arg)*);
        }
    }};
}
pub(crate) use log_or_panic;

#[allow(unused_macros)]
macro_rules! log_warn {
    ($fmt: literal) => {$crate::log_warn!($fmt,)};
    ($fmt: literal, $($arg: tt)*) => {{
        #[cfg(feature = "log")]
        log::warn!($fmt, $($arg)*);

        #[cfg(not(feature = "log"))]
        println!(
            concat!("egui_extras: warning: ", $fmt), $($arg)*
        )
    }};
}

#[allow(unused_imports)]
pub(crate) use log_warn;

#[allow(unused_macros)]
macro_rules! log_trace {
    ($fmt: literal) => {$crate::log_trace!($fmt,)};
    ($fmt: literal, $($arg: tt)*) => {{
        #[cfg(feature = "log")]
        log::trace!($fmt, $($arg)*);
    }};
}
#[allow(unused_imports)]
pub(crate) use log_trace;

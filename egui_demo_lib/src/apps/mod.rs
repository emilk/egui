mod color_test;
mod custom_3d;
mod demo;
mod fractal_clock;
#[cfg(feature = "http")]
mod http_app;

pub use color_test::ColorTest;
pub use custom_3d::Custom3dApp;
pub use demo::DemoApp;
pub use fractal_clock::FractalClock;
#[cfg(feature = "http")]
pub use http_app::HttpApp;

pub use demo::DemoWindows; // used for tests

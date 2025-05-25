#![doc = include_str!("../README.md")]

use eframe::{CreationContext, egui};

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // Log to android output
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
    .unwrap()
}

pub struct MyApp {
    demo: egui_demo_lib::DemoWindows,
}

impl MyApp {
    pub fn new(cc: &CreationContext) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Self {
            demo: egui_demo_lib::DemoWindows::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Reserve some space at the top so the demo ui isn't hidden behind the android status bar
        // TODO(lucasmerlin): This is a pretty big hack, should be fixed once safe_area implemented
        // for android:
        // https://github.com/rust-windowing/winit/issues/3910
        egui::TopBottomPanel::top("status_bar_space").show(ctx, |ui| {
            ui.set_height(32.0);
        });

        self.demo.ui(ctx);
    }
}

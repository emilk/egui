#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod ess_style_engine;

use eframe::egui;
use eframe::egui::style::WidgetVisuals;
use eframe::egui::style_trait::{
    Classes, DefaultWidgetStyle, HasClasses, StyleEngine, WidgetContext, WidgetName, WidgetStyle,
};
use eframe::egui::{
    Button, Color32, FontFamily, FontId, Frame, Margin, Response, Stroke, TextFormat,
};
use eframe::emath::TSTransform;
use std::fmt::Display;
use std::sync::Arc;
use crate::ess_style_engine::{EssFile, EssStyleEngine};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    // Our application state:
    let mut name = "Arthur".to_owned();
    let mut age = 42;

    let styles = Arc::new(EssFile::example());

    let mut custom_engine = Some(MyCustomWidgetStyle {
        default: DefaultWidgetStyle,
    });

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        if let Some(custom_engine) = custom_engine.take() {
            // ctx.set_style_engine(custom_engine);

            ctx.set_style_engine(EssStyleEngine::new(custom_engine, styles.clone()));
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("Your name: ");
                ui.text_edit_singleline(&mut name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut age, 0..=120).text("age"));
            if ui.button("Increment").clicked() {
                age += 1;
            }
            ui.label(format!("Hello '{name}', age {age}"));

            ui.horizontal(|ui| {
                ui.add(Button::new("Primary Button").primary());
                ui.add(Button::new("Secondary Button").secondary());
                ui.add(Button::new("Normal Button"));
            });

            ui.horizontal(|ui| {
                ui.add(Button::new("Large Primary").primary().lg());
                ui.add(Button::new("Large Secondary").secondary().lg());
                ui.add(Button::new("Small Normal").sm());
            });

            ui.add(Button::new("Customized via ESS").with_class("blue"));
        });
    })
}

#[derive(Clone)]
struct MyCustomWidgetStyle {
    // Can fallback to DefaultWidgetStyle
    default: DefaultWidgetStyle,
}

enum Variant {
    Primary,
    Secondary,
    Normal,
}

impl Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Primary => write!(f, "primary"),
            Variant::Secondary => write!(f, "secondary"),
            Variant::Normal => write!(f, "normal"),
        }
    }
}

impl Variant {
    fn from_classes(classes: &Classes) -> Self {
        if classes.has("primary") {
            Variant::Primary
        } else if classes.has("secondary") {
            Variant::Secondary
        } else {
            Variant::Normal
        }
    }

    fn color(&self) -> Color32 {
        match self {
            Variant::Primary => Color32::LIGHT_RED,
            Variant::Secondary => Color32::LIGHT_BLUE,
            Variant::Normal => Color32::LIGHT_GRAY,
        }
    }

    fn contrast_color(&self) -> Color32 {
        match self {
            Variant::Primary => Color32::WHITE,
            Variant::Secondary => Color32::BLACK,
            Variant::Normal => Color32::BLACK,
        }
    }
}

enum Size {
    Sm,
    Md,
    Lg,
}

impl Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Size::Sm => write!(f, "sm"),
            Size::Md => write!(f, "md"),
            Size::Lg => write!(f, "lg"),
        }
    }
}

impl Size {
    fn from_classes(classes: &Classes) -> Self {
        if classes.has("sm") {
            Size::Sm
        } else if classes.has("md") {
            Size::Md
        } else if classes.has("lg") {
            Size::Lg
        } else {
            Size::Md // Default size
        }
    }

    fn inner_margin(&self) -> Margin {
        match self {
            Size::Sm => Margin::symmetric(2, 2),
            Size::Md => Margin::symmetric(4, 4),
            Size::Lg => Margin::symmetric(6, 6),
        }
    }

    fn font_size(&self) -> f32 {
        match self {
            Size::Sm => 12.0,
            Size::Md => 14.0,
            Size::Lg => 16.0,
        }
    }

    fn frame_rounding(&self) -> f32 {
        match self {
            Size::Sm => 2.0,
            Size::Md => 4.0,
            Size::Lg => 6.0,
        }
    }
}

impl StyleEngine<WidgetStyle> for MyCustomWidgetStyle {
    fn get(&self, ctx: &WidgetContext) -> WidgetStyle {
        // let mut visuals = self.default.get(ctx);

        let variant = Variant::from_classes(&ctx.classes);

        let size = Size::from_classes(&ctx.classes);

        let mut style = WidgetStyle {
            frame: Frame::new()
                .fill(variant.color())
                .inner_margin(size.inner_margin())
                .corner_radius(size.frame_rounding()),
            text: TextFormat::simple(
                FontId::new(size.font_size(), FontFamily::Proportional),
                variant.contrast_color(),
            ),
            stroke: Stroke::default(),
            transform: TSTransform::default(),
        };

        let state = if ctx.response.is_pointer_button_down_on() {
            -1.0
        } else if ctx.response.hovered() {
            1.0
        } else {
            0.0
        };

        let state_animated =
            ctx.ui
                .ctx()
                .animate_value_with_time(ctx.response.id.with("style_anim"), state, 0.05);

        let lerp_to_darken = Color32::BLACK;
        let lerp_to_lighten = Color32::WHITE;

        if state_animated < 0.0 {
            style.frame.fill = variant
                .color()
                .lerp_to_gamma(lerp_to_darken, 0.03 * -state_animated);
        } else if state_animated > 0.0 {
            style.frame.fill = variant
                .color()
                .lerp_to_gamma(lerp_to_lighten, 0.1 * state_animated);
        }

        match ctx.name {
            WidgetName::Button => {
                let scale = 1.0 + state_animated * 0.02;
                if scale != 1.0 {
                    // style.frame.inner_margin += 4;
                    let center = ctx.response.rect.center().to_vec2();
                    style.transform = TSTransform::from_translation(center)
                        * TSTransform::from_scaling(scale)
                        * TSTransform::from_translation(-center);
                }
            }
            WidgetName::Checkbox => {}
            WidgetName::Slider => {}
            WidgetName::TextInput => {}
            WidgetName::Custom(_) => {}
        }

        style
    }
}

// trait ClassExt {
//     fn primary(self) -> Self;
// }
//
// impl<T> ClassExt for T where T: HasClasses {
//     fn primary(self) -> Self {
//         self.with_class("primary")
//     }
// }

macro_rules! classes {
    ($trait_name:ident: ($($name:ident, )+)) => {
        // pub enum $trait_name {
        //     $(
        //         $name,
        //     )*
        // }

        pub trait $trait_name {
            $(
                fn $name(self) -> Self;
            )*
        }

        impl<T> $trait_name for T
        where
            T: HasClasses,
        {
                $(fn $name(mut self) -> Self {
                    self.with_class(stringify!($name))
                })?
        }
    };
}

classes!(CustomStyle: (primary, secondary, normal, sm, md, lg, ));

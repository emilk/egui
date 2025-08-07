use eframe::egui;
use eframe::egui::Frame;
use eframe::egui::style_trait::{ButtonStyle, StyleEngine, WidgetContext, WidgetStyle};
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

pub trait Stylable {
    fn name() -> &'static str;
    fn set_property(&mut self, key: &[&str], value: &str) -> Result<(), StyleError>;
}

type StyleError = Box<dyn std::error::Error + Send + Sync>;

impl Stylable for Frame {
    fn name() -> &'static str {
        "Frame"
    }

    fn set_property(&mut self, key: &[&str], value: &str) -> Result<(), StyleError> {
        let key = *key.first().ok_or_else(|| "Missing key")?;
        match key {
            "fill" => {
                // Maybe these could also use FromStr?
                self.fill = serde_json::from_str(value)?;
            }
            "stroke" => {
                self.stroke = serde_json::from_str(value)?;
            }
            "corner_radius" => {
                self.corner_radius = serde_json::from_str(value)?;
            }
            "inner_margin" => {
                self.inner_margin = serde_json::from_str(value)?;
            }
            "outer_margin" => {
                self.outer_margin = serde_json::from_str(value)?;
            }
            _ => {
                return Err(Box::from(format!("Unknown property: {}", key)));
            }
        }
        Ok(())
    }
}

impl Stylable for ButtonStyle {
    fn name() -> &'static str {
        "Button"
    }

    fn set_property(&mut self, key: &[&str], value: &str) -> Result<(), StyleError> {
        match key {
            ["frame_fill"] => {
                self.frame.fill = serde_json::from_str(value)?;
            }
            ["frame", param] => {
                self.frame.set_property(&[param], value)?;
            }
            // Etc...
            _ => {
                Err(format!("Unknown property: {:?}", key))?;
            }
        }
        Ok(())
    }
}

impl Stylable for WidgetStyle {
    fn name() -> &'static str {
        "WidgetStyle"
    }

    fn set_property(&mut self, key: &[&str], value: &str) -> Result<(), StyleError> {
        match key {
            ["frame_fill"] => {
                self.frame.fill = serde_json::from_str(value)?;
            }
            ["color"] => {
                self.stroke.color = serde_json::from_str(value)?;
                self.text.color = serde_json::from_str(value)?; // 😬
            }
            ["frame", param] => {
                self.frame.set_property(&[param], value)?;
            }
            // Etc...
            _ => {
                Err(format!("Unknown property: {:?}", key))?;
            }
        }
        Ok(())
    }
}

pub struct EssStyleEngine<E, T> {
    style: Arc<EssFile>,
    wrapped_engine: E,
    _stylable: std::marker::PhantomData<T>,
}

impl<E: StyleEngine<T>, T: Stylable> EssStyleEngine<E, T> {
    pub fn new(engine: E, style: Arc<EssFile>) -> Self {
        Self {
            style,
            wrapped_engine: engine,
            _stylable: std::marker::PhantomData,
        }
    }
}

impl<E: StyleEngine<T>, T: Stylable + Sync + Send> StyleEngine<T> for EssStyleEngine<E, T> {
    fn get(&self, ctx: &WidgetContext) -> T {
        let name = T::name();

        // Ideally there would be some caching here since the set_property calls can be expensive
        // (since they do serde deserialization)

        let mut style = self.wrapped_engine.get(ctx);

        dbg!(name);

        if let Some(rules) = self.style.style.get(name) {
            let rules = self.style.style.get(name).unwrap();

            // This is very primitive, you probably want something like css specificity
            for rule in rules {
                if rule.check(&ctx) {
                    for (keys, value) in &rule.properties {
                        dbg!(keys.as_slice(), value);
                        style
                            .set_property(keys.as_slice(), value)
                            .expect("Failed to set property"); // TODO: Error handling
                    }
                }
            }
        }

        style
    }
}

enum State {
    Active,
    Hovered,
    Focused,
    Disabled,
}

enum Modifier {
    Class(String),
    State(State),
}

struct EssRule {
    modifiers: Vec<Modifier>,
    properties: Vec<(Vec<&'static str>, String)>,
}

impl EssRule {
    fn check(&self, ctx: &WidgetContext) -> bool {
        // Check if the rule matches the context
        for modifier in &self.modifiers {
            match modifier {
                Modifier::Class(class) => {
                    if !ctx.classes.has(class) {
                        return false;
                    }
                }
                Modifier::State(state) => match state {
                    State::Active => {
                        if !ctx.response.is_pointer_button_down_on() {
                            return false;
                        }
                    }
                    State::Hovered => {
                        if !ctx.response.hovered() {
                            return false;
                        }
                    }
                    State::Focused => {
                        if !ctx.response.has_focus() {
                            return false;
                        }
                    }
                    State::Disabled => {
                        if ctx.response.enabled() {
                            return false;
                        }
                    }
                },
            }
        }
        true
    }
}

pub struct EssFile {
    // Rules for different widget types
    style: HashMap<String, Vec<EssRule>>,
}

impl EssFile {
    pub fn parse(file: &str) {
        todo!()
    }

    pub fn example() -> Self {
        let mut style = HashMap::new();

        // Example rule for Button
        style.insert(
            "WidgetStyle".to_string(),
            vec![
                EssRule {
                    modifiers: vec![Modifier::Class("blue".to_string())],
                    properties: vec![
                        (vec!["frame_fill"], "[0, 0, 200, 255]".to_string()),
                        (vec!["color"], "[255, 255, 255, 255]".to_string()),
                    ],
                },
                EssRule {
                    modifiers: vec![
                        Modifier::State(State::Hovered),
                        Modifier::Class("blue".to_string()),
                    ],
                    // Serde implementation of color32 could be improved...
                    properties: vec![
                        (vec!["frame_fill"], "[0, 0, 255, 255]".to_string()),
                    ],
                },
            ],
        );

        Self { style }
    }
}

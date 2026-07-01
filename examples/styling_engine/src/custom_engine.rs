use std::collections::HashMap;

use eframe::egui::{
    Color32, Ui,
    theme_plugin::{ThemeCache, ThemeStyle},
    widget_style::{BaseStyle, ButtonStyle, Classes, HasClasses as _, WidgetState},
};
use logos::Logos;

#[derive(Debug, Default, Clone)]
pub struct ESSEngine {
    info: HashMap<String, Vec<(String, Value)>>,
    cache: ThemeCache,
}

impl ESSEngine {
    pub fn try_parse(ess: &str) -> Result<Self, String> {
        let mut engine = Self::default();
        let mut lexer = Token::lexer(ess);
        let mut hash = HashMap::new();
        while let Some(token) = lexer.next() {
            if token == Ok(Token::Class) {
                let selector = lexer.slice()[1..].to_owned();
                if lexer
                    .next()
                    .is_some_and(|token| token.is_ok_and(|token| token != Token::Open))
                {
                    return Err("No opening bracket found !".to_owned());
                }

                let mut declarations = vec![];

                loop {
                    match lexer.next() {
                        Some(Ok(Token::Property)) => {
                            let property = lexer.slice().to_owned();

                            if lexer
                                .next()
                                .is_some_and(|token| token.is_ok_and(|token| token != Token::Is))
                            {
                                return Err("No separator between property and value !".to_owned());
                            }

                            let value = match lexer.next() {
                                Some(Ok(Token::Number)) => Value::Number(
                                    lexer
                                        .slice()
                                        .to_owned()
                                        .parse::<usize>()
                                        .expect("Should be a positive integer"),
                                ),
                                Some(Ok(Token::Color)) => Value::Color(
                                    Color32::from_hex(lexer.slice())
                                        .expect("Should be a valid hex"),
                                ),
                                Some(Ok(v)) => return Err(format!("Invalid value : {v:?}")),
                                _ => return Err("Error".to_owned()),
                            };

                            declarations.push((property, value));
                        }
                        Some(Ok(Token::Close)) => break,
                        Some(Ok(v)) => {
                            return Err(format!("Missing close bracket, found : {v:?}"));
                        }
                        v => return Err(format!("Error : {v:?}")),
                    }
                }

                hash.insert(selector, declarations);
            }
        }
        engine.info = hash;
        Ok(engine)
    }
}

impl ThemeStyle<ButtonStyle> for ESSEngine {
    fn style(&mut self, ui: &Ui, classes: &Classes, state: WidgetState) -> ButtonStyle {
        self.cache.get(classes, state, || {
            let base = ui.get_widget_style::<BaseStyle>(classes, state);
            let mut default = ButtonStyle {
                frame: base.frame,
                text_style: base.text,
            };
            for class in classes.list() {
                if let Some(properties) = self.info.get(&class.to_string()) {
                    for (property, value) in properties {
                        match property.as_str() {
                            "fill" => {
                                if let Value::Color(color) = value {
                                    default.frame.fill = *color;
                                }
                            }
                            "border" => {
                                if let Value::Number(size) = value {
                                    default.frame.stroke.width = *size as f32;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            default
        })
    }
}

#[derive(Debug, Logos, PartialEq)]
enum Token {
    #[token("{")]
    Open,
    #[token("}")]
    Close,
    #[token(":")]
    Is,
    #[regex(r"\.[a-zA-Z]+")]
    Class,
    #[regex(r"[a-zA-Z]+")]
    Property,
    #[regex(r"[0-9]+")]
    Number,
    #[regex(r"#(?:[0-9a-fA-F]{3}){1,2}")]
    Color,
    #[regex(r"[ \t\n\f;]+", logos::skip)]
    Whitespace,
}

#[derive(Debug, Clone)]
enum Value {
    Number(usize),
    Color(Color32),
}

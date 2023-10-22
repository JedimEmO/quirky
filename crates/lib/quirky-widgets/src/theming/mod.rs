use crate::theming::button_theme::ButtonTheme;
use serde::{Deserialize, Serialize};

pub mod button_theme;
pub mod input_theme;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuirkyTheme {
    pub button: ButtonTheme,
    pub input: input_theme::InputTheme,
}

impl QuirkyTheme {
    pub fn dark_default() -> Self {
        Self {
            button: ButtonTheme::dark_default(),
            input: input_theme::InputTheme::dark_default(),
        }
    }

    pub fn light_default() -> Self {
        Self {
            button: ButtonTheme::light_default(),
            input: input_theme::InputTheme::light_default(),
        }
    }
}

impl Default for QuirkyTheme {
    fn default() -> Self {
        Self::dark_default()
    }
}

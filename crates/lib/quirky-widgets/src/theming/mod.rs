use serde::{Deserialize, Serialize};

pub mod input_theme;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuirkyTheme {
    pub input_theme: input_theme::QuirkyInputTheme,
}

impl QuirkyTheme {
    pub fn dark_default() -> Self {
        Self {
            input_theme: input_theme::QuirkyInputTheme::dark_default(),
        }
    }

    pub fn light_default() -> Self {
        Self {
            input_theme: input_theme::QuirkyInputTheme::light_default(),
        }
    }
}

impl Default for QuirkyTheme {
    fn default() -> Self {
        Self::dark_default()
    }
}

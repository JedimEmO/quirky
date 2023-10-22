use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct QuirkyInputTheme {
    pub background_color: [f32; 4],
    pub background_color_hovered: [f32; 4],
    pub border_color: [f32; 4],
    pub border_color_focused: [f32; 4],
    pub border_width: f32,
    pub label_color: [f32; 4],
    pub label_color_invalid: [f32; 4],
    pub text_color: [f32; 4],
    pub text_size: f32,
    pub text_padding: f32,
    pub text_font: String,
}

impl QuirkyInputTheme {
    pub fn dark_default() -> Self {
        Self {
            background_color: [0.02, 0.02, 0.02, 1.0],
            background_color_hovered: [0.05, 0.05, 0.05, 1.0],
            border_color: [0.05, 0.05, 0.05, 1.0],
            border_color_focused: [0.05, 0.1, 0.1, 1.0],
            border_width: 1.0,
            label_color: [0.3, 0.3, 0.3, 1.0],
            label_color_invalid: [0.9, 0.1, 0.1, 1.0],
            text_color: [0.8, 0.8, 0.8, 1.0],
            text_size: 12.0,
            text_padding: 4.0,
            text_font: "sans-serif".to_string(),
        }
    }

    pub fn light_default() -> Self {
        Self {
            background_color: [0.8, 0.8, 0.8, 1.0],
            background_color_hovered: [0.7, 0.7, 0.7, 1.0],
            border_color: [0.0, 0.0, 0.01, 1.0],
            border_color_focused: [0.0, 0.0, 0.2, 1.0],
            border_width: 1.0,
            label_color: [0.0, 0.00, 0.00, 1.0],
            label_color_invalid: [0.1, 0.001, 0.001, 1.0],
            text_color: [0.0, 0.0, 0.0, 1.0],
            text_size: 12.0,
            text_padding: 4.0,
            text_font: "sans-serif".to_string(),
        }
    }
}

impl Default for QuirkyInputTheme {
    fn default() -> Self {
        Self::dark_default()
    }
}

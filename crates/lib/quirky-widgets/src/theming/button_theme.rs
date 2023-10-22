use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ButtonTheme {
    pub color: [f32; 4],
    pub hover_color: [f32; 4],
    pub pressed_color: [f32; 4],
}

impl ButtonTheme {
    pub fn dark_default() -> Self {
        Self {
            color: [0.1, 0.1, 0.1, 1.0],
            hover_color: [0.2, 0.2, 0.2, 1.0],
            pressed_color: [0.3, 0.3, 0.3, 1.0],
        }
    }

    pub fn light_default() -> Self {
        Self {
            color: [0.8, 0.8, 0.8, 1.0],
            hover_color: [0.7, 0.7, 0.7, 1.0],
            pressed_color: [0.6, 0.6, 0.6, 1.0],
        }
    }
}

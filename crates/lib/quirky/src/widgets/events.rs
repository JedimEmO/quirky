use glam::UVec2;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Num(usize),
}

#[derive(Copy, Clone, PartialEq, Eq, Ord, PartialOrd)]
#[repr(u32)]
pub enum KeyCode {
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    Snapshot,
    Scroll,
    Pause,
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,
    Left,
    Up,
    Right,
    Down,
    Backspace,
    Return,
    Space,
    Compose,
    Caret,
    NumLock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,
    Apostrophe,
    Apps,
    Asterisk,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Period,
    Convert,
    Equals,
    Grave,
    Semicolon,
    At,
    Enter,
    Unknown,
}

#[derive(Clone, Default, Debug)]
pub struct KeyboardModifier {
    pub alt: bool,
    pub shift: bool,
    pub ctrl: bool,
}

#[derive(Clone)]
pub enum KeyboardEvent {
    KeyPressed {
        key_code: KeyCode,
        modifier: KeyboardModifier,
    },
}

#[derive(Clone)]
pub enum MouseEvent {
    Enter {
        pos: UVec2,
    },
    Leave {},
    Move {
        pos: UVec2,
    },
    ButtonDown {
        button: MouseButton,
    },
    ButtonUp {
        button: MouseButton,
    },
    Drag {
        from: UVec2,
        to: UVec2,
        button: MouseButton,
    },
}

#[derive(Clone, Copy, PartialEq)]
pub enum FocusState {
    Focused,
    Unfocused,
}

#[derive(Clone)]
pub enum WidgetEvent {
    KeyboardEvent { event: KeyboardEvent },
    MouseEvent { event: MouseEvent },
    FocusChange(FocusState),
}

#[derive(Clone)]
pub struct EventDispatch {
    pub receiver_id: Uuid,
    pub event: WidgetEvent,
}

use enigo::{Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use std::sync::mpsc::Receiver;

pub enum MouseCmd {
    Move { dx: i32, dy: i32 },
    Click { button: Button },
    Scroll { dy: i32 },
    /// A key press with optional held modifiers, e.g. Ctrl+Cmd+F. An empty
    /// `modifiers` is just a single key tap.
    Combo { modifiers: Vec<Key>, key: Key },
}

pub fn parse_button(s: &str) -> Option<Button> {
    match s {
        "left" => Some(Button::Left),
        "right" => Some(Button::Right),
        "middle" => Some(Button::Middle),
        _ => None,
    }
}

pub fn run_input_thread(rx: Receiver<MouseCmd>) {
    let mut enigo = Enigo::new(&Settings::default()).expect("failed to create Enigo");

    for cmd in rx {
        match cmd {
            MouseCmd::Move { dx, dy } => {
                let _ = enigo.move_mouse(dx, dy, Coordinate::Rel);
            }
            MouseCmd::Click { button } => {
                let _ = enigo.button(button, Direction::Click);
            }
            MouseCmd::Scroll { dy } => {
                let _ = enigo.scroll(dy, enigo::Axis::Vertical);
            }
            MouseCmd::Combo { modifiers, key } => {
                for m in &modifiers {
                    let _ = enigo.key(*m, Direction::Press);
                }
                let _ = enigo.key(key, Direction::Click);
                for m in modifiers.iter().rev() {
                    let _ = enigo.key(*m, Direction::Release);
                }
            }
        }
    }
}

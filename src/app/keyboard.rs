use crate::Message;

pub fn handle_key_event(
    event: iced::event::Event,
    status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    use iced::keyboard::{key::Named, Event as KbEvent, Key};

    let (key, modifiers) = match event {
        iced::event::Event::Keyboard(KbEvent::KeyPressed {
            key, modifiers, ..
        }) => (key, modifiers),
        _ => return None,
    };

    // App-level accelerators fire regardless of widget focus.
    if modifiers.command() {
        if let Key::Character(ref s) = key {
            match s.as_str() {
                "s" | "S" => return Some(Message::FileSave),
                "o" | "O" => return Some(Message::FileOpen),
                _ => {}
            }
        }
    }

    // Keys that arrive while a text input has focus (Status::Captured) —
    // the formula editor's autocomplete consumes Up/Down/Tab/Escape here,
    // bypassing the focused-widget gate below. The `update` arms are
    // no-ops when the formula editor isn't actually open.
    if matches!(status, iced::event::Status::Captured) {
        match key {
            Key::Named(Named::ArrowUp) => {
                return Some(Message::FormulaSuggestionMove(-1))
            }
            Key::Named(Named::ArrowDown) => {
                return Some(Message::FormulaSuggestionMove(1))
            }
            Key::Named(Named::Escape) => return Some(Message::FormulaEscape),
            _ => {}
        }
    }

    // Tab is not claimed by iced's text_input, so its status is Ignored.
    if let Key::Named(Named::Tab) = key {
        return Some(Message::FormulaSuggestionAccept);
    }

    // Navigation and cell-edit keys defer to focused widgets.
    if !matches!(status, iced::event::Status::Ignored) {
        return None;
    }

    match key {
        Key::Named(Named::ArrowUp) => Some(Message::CellMove(-1, 0)),
        Key::Named(Named::ArrowDown) => Some(Message::CellMove(1, 0)),
        Key::Named(Named::ArrowLeft) => Some(Message::CellMove(0, -1)),
        Key::Named(Named::ArrowRight) => Some(Message::CellMove(0, 1)),
        Key::Named(Named::Enter) | Key::Named(Named::F2) => Some(Message::CellEditBegin),
        Key::Named(Named::Delete) | Key::Named(Named::Backspace) => Some(Message::CellClear),
        Key::Named(Named::Escape) => Some(Message::CellEditCancel),
        _ => None,
    }
}

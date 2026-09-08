//! Aura Editor application library.
//!
//! Keeps Iced application logic out of `main` so unit and headless functional
//! tests can exercise state and view without opening a real window.

use iced::widget::{button, column, text};
use iced::Element;

/// Root application state for the editor shell.
#[derive(Debug, Clone, Default)]
pub struct App {
    clicked: bool,
}

/// Messages produced by the hello-world shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    /// User pressed the primary action button.
    Clicked,
}

impl App {
    /// Creates the initial application state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Window title shown by the native shell.
    pub fn title(&self) -> String {
        String::from("Aura Editor")
    }

    /// Applies a message to application state.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Clicked => {
                self.clicked = true;
            }
        }
    }

    /// Builds the hello-world view.
    pub fn view(&self) -> Element<'_, Message> {
        column![
            text("Hello, Aura"),
            text(self.status()),
            button("Click me").on_press(Message::Clicked),
        ]
        .spacing(12)
        .padding(24)
        .into()
    }

    /// Status line shown under the greeting.
    pub fn status(&self) -> &'static str {
        if self.clicked {
            "Button clicked"
        } else {
            "Ready"
        }
    }

    /// Whether the primary button has been clicked.
    pub fn clicked(&self) -> bool {
        self.clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_app_starts_ready() {
        let app = App::new();
        assert!(!app.clicked());
        assert_eq!(app.status(), "Ready");
        assert_eq!(app.title(), "Aura Editor");
    }

    #[test]
    fn clicked_message_updates_status() {
        let mut app = App::new();
        app.update(Message::Clicked);
        assert!(app.clicked());
        assert_eq!(app.status(), "Button clicked");
    }
}

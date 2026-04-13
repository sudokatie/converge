//! In-game chat UI.
//!
//! Provides text input and message history display.

use egui::{Color32, RichText, ScrollArea, Vec2};
use std::collections::VecDeque;

/// Maximum messages to keep in history.
const MAX_MESSAGES: usize = 100;

/// Actions returned by the chat screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatAction {
    /// Send a chat message.
    Send(String),
    /// Close the chat.
    Close,
}

/// A chat message.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    /// Sender name (empty for system messages).
    pub sender: String,
    /// Message content.
    pub content: String,
    /// Whether this is a system message.
    pub is_system: bool,
    /// Timestamp (seconds since start).
    pub timestamp: f64,
}

impl ChatMessage {
    /// Create a player message.
    #[must_use]
    pub fn player(sender: impl Into<String>, content: impl Into<String>, timestamp: f64) -> Self {
        Self {
            sender: sender.into(),
            content: content.into(),
            is_system: false,
            timestamp,
        }
    }

    /// Create a system message.
    #[must_use]
    pub fn system(content: impl Into<String>, timestamp: f64) -> Self {
        Self {
            sender: String::new(),
            content: content.into(),
            is_system: true,
            timestamp,
        }
    }
}

/// Chat screen state.
#[derive(Clone, Debug, Default)]
pub struct ChatScreen {
    /// Whether the chat is open (for input).
    is_open: bool,
    /// Current input text.
    input_text: String,
    /// Message history.
    messages: VecDeque<ChatMessage>,
    /// Whether to scroll to bottom on next draw.
    scroll_to_bottom: bool,
    /// Time messages stay visible when chat is closed (seconds).
    fade_time: f32,
    /// Request focus on next frame.
    request_focus: bool,
}

impl ChatScreen {
    /// Create a new chat screen.
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_open: false,
            input_text: String::new(),
            messages: VecDeque::new(),
            scroll_to_bottom: false,
            fade_time: 10.0,
            request_focus: false,
        }
    }

    /// Check if chat is open (accepting input).
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Open the chat for input.
    pub fn open(&mut self) {
        self.is_open = true;
        self.request_focus = true;
    }

    /// Close the chat.
    pub fn close(&mut self) {
        self.is_open = false;
        self.input_text.clear();
    }

    /// Toggle the chat.
    pub fn toggle(&mut self) {
        if self.is_open {
            self.close();
        } else {
            self.open();
        }
    }

    /// Add a message to the chat.
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push_back(message);

        // Limit history size
        while self.messages.len() > MAX_MESSAGES {
            self.messages.pop_front();
        }

        self.scroll_to_bottom = true;
    }

    /// Add a player message.
    pub fn add_player_message(
        &mut self,
        sender: impl Into<String>,
        content: impl Into<String>,
        timestamp: f64,
    ) {
        self.add_message(ChatMessage::player(sender, content, timestamp));
    }

    /// Add a system message.
    pub fn add_system_message(&mut self, content: impl Into<String>, timestamp: f64) {
        self.add_message(ChatMessage::system(content, timestamp));
    }

    /// Get message count.
    #[must_use]
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Set fade time for messages when chat is closed.
    pub fn set_fade_time(&mut self, seconds: f32) {
        self.fade_time = seconds;
    }

    /// Draw the chat UI.
    ///
    /// Returns an action if the user interacted with the UI.
    pub fn draw(&mut self, ctx: &egui::Context, current_time: f64) -> Option<ChatAction> {
        let mut action = None;

        // Always draw recent messages (overlay style when closed)
        self.draw_messages(ctx, current_time);

        // Draw input box when open
        if self.is_open {
            action = self.draw_input(ctx);
        }

        action
    }

    /// Draw recent messages.
    fn draw_messages(&mut self, ctx: &egui::Context, current_time: f64) {
        let area = egui::Area::new(egui::Id::new("chat_messages"))
            .anchor(egui::Align2::LEFT_BOTTOM, Vec2::new(10.0, -60.0))
            .order(egui::Order::Foreground);

        area.show(ctx, |ui| {
            ui.set_max_width(400.0);

            let messages: Vec<_> = if self.is_open {
                // Show all recent messages when open
                self.messages.iter().rev().take(20).collect()
            } else {
                // Only show messages within fade time when closed
                self.messages
                    .iter()
                    .rev()
                    .filter(|m| current_time - m.timestamp < self.fade_time as f64)
                    .take(5)
                    .collect()
            };

            // Draw in reverse order (oldest first, newer on bottom)
            for msg in messages.into_iter().rev() {
                let text = if msg.is_system {
                    RichText::new(&msg.content).color(Color32::YELLOW).italics()
                } else {
                    let formatted = format!("<{}> {}", msg.sender, msg.content);
                    RichText::new(formatted).color(Color32::WHITE)
                };

                ui.label(text);
            }
        });
    }

    /// Draw the input box.
    fn draw_input(&mut self, ctx: &egui::Context) -> Option<ChatAction> {
        let mut action = None;

        egui::TopBottomPanel::bottom("chat_input")
            .frame(egui::Frame::none().fill(Color32::from_rgba_unmultiplied(0, 0, 0, 200)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Chat:");

                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.input_text)
                            .desired_width(300.0)
                            .hint_text("Type a message..."),
                    );

                    // Request focus if needed
                    if self.request_focus {
                        response.request_focus();
                        self.request_focus = false;
                    }

                    // Send on Enter
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if !self.input_text.trim().is_empty() {
                            action = Some(ChatAction::Send(self.input_text.clone()));
                            self.input_text.clear();
                        }
                        // Keep chat open after sending
                        self.request_focus = true;
                    }

                    // Close on Escape
                    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        action = Some(ChatAction::Close);
                    }

                    if ui.button("Send").clicked() && !self.input_text.trim().is_empty() {
                        action = Some(ChatAction::Send(self.input_text.clone()));
                        self.input_text.clear();
                        self.request_focus = true;
                    }
                });
            });

        // Handle close action
        if matches!(action, Some(ChatAction::Close)) {
            self.close();
        }

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_screen_new() {
        let screen = ChatScreen::new();
        assert!(!screen.is_open());
        assert_eq!(screen.message_count(), 0);
    }

    #[test]
    fn test_open_close() {
        let mut screen = ChatScreen::new();

        screen.open();
        assert!(screen.is_open());

        screen.close();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_toggle() {
        let mut screen = ChatScreen::new();

        screen.toggle();
        assert!(screen.is_open());

        screen.toggle();
        assert!(!screen.is_open());
    }

    #[test]
    fn test_add_player_message() {
        let mut screen = ChatScreen::new();

        screen.add_player_message("Player1", "Hello world", 1.0);

        assert_eq!(screen.message_count(), 1);
        let msg = &screen.messages[0];
        assert_eq!(msg.sender, "Player1");
        assert_eq!(msg.content, "Hello world");
        assert!(!msg.is_system);
    }

    #[test]
    fn test_add_system_message() {
        let mut screen = ChatScreen::new();

        screen.add_system_message("Player joined the game", 1.0);

        assert_eq!(screen.message_count(), 1);
        let msg = &screen.messages[0];
        assert!(msg.is_system);
        assert!(msg.sender.is_empty());
    }

    #[test]
    fn test_message_limit() {
        let mut screen = ChatScreen::new();

        // Add more than MAX_MESSAGES
        for i in 0..150 {
            screen.add_player_message("Test", format!("Message {}", i), i as f64);
        }

        assert!(screen.message_count() <= MAX_MESSAGES);
    }

    #[test]
    fn test_clear_messages() {
        let mut screen = ChatScreen::new();

        screen.add_player_message("Test", "Hello", 1.0);
        screen.add_player_message("Test", "World", 2.0);

        screen.clear();
        assert_eq!(screen.message_count(), 0);
    }

    #[test]
    fn test_chat_message_builders() {
        let player_msg = ChatMessage::player("Alice", "Hi", 1.0);
        assert_eq!(player_msg.sender, "Alice");
        assert!(!player_msg.is_system);

        let system_msg = ChatMessage::system("Server starting", 2.0);
        assert!(system_msg.sender.is_empty());
        assert!(system_msg.is_system);
    }
}

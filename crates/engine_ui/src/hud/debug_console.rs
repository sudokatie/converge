//! In-game debug console for developer commands.
//!
//! Provides a text input for entering debug commands with output history.

use std::collections::VecDeque;

/// Maximum number of lines in console history.
const MAX_HISTORY: usize = 200;

/// A line in the console output.
#[derive(Debug, Clone)]
pub struct ConsoleLine {
    /// Text content.
    pub text: String,
    /// Line type (for coloring).
    pub kind: LineKind,
}

/// Type of console line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// Command input from the user.
    Input,
    /// Normal output.
    Output,
    /// Error message.
    Error,
    /// Debug/trace info.
    Debug,
}

/// Action returned by the console.
#[derive(Debug, Clone)]
pub enum ConsoleAction {
    /// User submitted a command.
    Command(String),
}

/// Debug console state.
#[derive(Debug, Clone)]
pub struct DebugConsole {
    /// Whether the console is visible.
    open: bool,
    /// Output history.
    history: VecDeque<ConsoleLine>,
    /// Current input text.
    input: String,
    /// Command history for up/down arrow navigation.
    command_history: Vec<String>,
    /// Current position in command history (for up/down).
    history_index: Option<usize>,
    /// Scroll position.
    scroll_to_bottom: bool,
}

impl Default for DebugConsole {
    fn default() -> Self {
        let mut console = Self {
            open: false,
            history: VecDeque::with_capacity(MAX_HISTORY),
            input: String::new(),
            command_history: Vec::new(),
            history_index: None,
            scroll_to_bottom: false,
        };
        console.add_line("Debug console ready. Type 'help' for commands.", LineKind::Output);
        console
    }
}

impl DebugConsole {
    /// Create a new debug console.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if the console is open.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Toggle the console open/closed.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Open the console.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Close the console.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Add a line to the console output.
    pub fn add_line(&mut self, text: &str, kind: LineKind) {
        if self.history.len() >= MAX_HISTORY {
            self.history.pop_front();
        }
        self.history.push_back(ConsoleLine {
            text: text.to_string(),
            kind,
        });
        self.scroll_to_bottom = true;
    }

    /// Add an output line.
    pub fn output(&mut self, text: &str) {
        self.add_line(text, LineKind::Output);
    }

    /// Add an error line.
    pub fn error(&mut self, text: &str) {
        self.add_line(text, LineKind::Error);
    }

    /// Add a debug line.
    pub fn debug(&mut self, text: &str) {
        self.add_line(text, LineKind::Debug);
    }

    /// Get the console history.
    #[must_use]
    pub fn history(&self) -> &VecDeque<ConsoleLine> {
        &self.history
    }

    /// Draw the console and return any submitted command.
    pub fn draw(&mut self, ctx: &egui::Context) -> Option<ConsoleAction> {
        if !self.open {
            return None;
        }

        let mut action = None;

        // Position at bottom of screen, taking up ~40% height
        let screen = ctx.screen_rect();
        let console_height = screen.height() * 0.4;

        egui::Area::new(egui::Id::new("debug_console"))
            .fixed_pos(egui::pos2(0.0, screen.height() - console_height))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let frame = egui::Frame::default()
                    .fill(egui::Color32::from_black_alpha(220))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY));

                frame.show(ui, |ui| {
                    ui.set_width(screen.width());
                    ui.set_height(console_height);

                    ui.vertical(|ui| {
                        // Header
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Console")
                                    .size(14.0)
                                    .color(egui::Color32::YELLOW),
                            );
                            ui.add_space(8.0);
                            if ui.small_button("Clear").clicked() {
                                self.history.clear();
                            }
                            if ui.small_button("Close").clicked() {
                                self.open = false;
                            }
                        });

                        ui.separator();

                        // Output area (scrollable)
                        let output_height = console_height - 60.0;
                        egui::ScrollArea::vertical()
                            .max_height(output_height)
                            .stick_to_bottom(self.scroll_to_bottom)
                            .show(ui, |ui| {
                                for line in &self.history {
                                    let color = match line.kind {
                                        LineKind::Input => egui::Color32::LIGHT_GREEN,
                                        LineKind::Output => egui::Color32::WHITE,
                                        LineKind::Error => egui::Color32::RED,
                                        LineKind::Debug => egui::Color32::GRAY,
                                    };
                                    ui.label(egui::RichText::new(&line.text).size(12.0).color(color));
                                }
                            });

                        self.scroll_to_bottom = false;

                        ui.separator();

                        // Input field
                        ui.horizontal(|ui| {
                            ui.label(">");

                            let response = ui.add_sized(
                                egui::vec2(screen.width() - 40.0, 20.0),
                                egui::TextEdit::singleline(&mut self.input)
                                    .font(egui::TextStyle::Monospace),
                            );

                            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                if !self.input.is_empty() {
                                    let cmd = self.input.trim().to_string();
                                    self.add_line(&format!("> {}", cmd), LineKind::Input);
                                    self.command_history.push(cmd.clone());
                                    self.history_index = None;
                                    action = Some(ConsoleAction::Command(cmd));
                                    self.input.clear();
                                }
                            }

                            // Auto-focus the input
                            response.request_focus();
                        });
                    });
                });
            });

        action
    }

    /// Navigate command history (up/down).
    pub fn navigate_history(&mut self, up: bool) {
        if self.command_history.is_empty() {
            return;
        }

        if up {
            // Go back in history (older)
            let idx = self.history_index.unwrap_or(self.command_history.len());
            if idx > 0 {
                self.history_index = Some(idx - 1);
                self.input = self.command_history[idx - 1].clone();
            }
        } else {
            // Go forward (newer)
            if let Some(idx) = self.history_index {
                if idx + 1 < self.command_history.len() {
                    self.history_index = Some(idx + 1);
                    self.input = self.command_history[idx + 1].clone();
                } else {
                    self.history_index = None;
                    self.input.clear();
                }
            }
        }
    }
}

/// Built-in debug command processor.
///
/// Handles common debug commands that don't need game-specific context.
pub fn process_builtin_command(console: &mut DebugConsole, cmd: &str) -> bool {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }

    match parts[0] {
        "help" => {
            console.output("Available commands:");
            console.output("  help        - Show this help");
            console.output("  clear       - Clear console");
            console.output("  echo <msg>  - Print a message");
            console.output("  time        - Show current time");
            console.output("  fps         - Toggle FPS display");
            console.output("  fly         - Toggle fly mode");
            console.output("  give <item> - Give item to player");
            console.output("  tp <x> <y> <z> - Teleport to position");
            true
        }
        "clear" => {
            console.history.clear();
            true
        }
        "echo" => {
            let msg = parts[1..].join(" ");
            console.output(&msg);
            true
        }
        "time" => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            console.output(&format!("Unix time: {}s", now.as_secs()));
            true
        }
        _ => false, // Not a builtin, let game code handle it
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_console() {
        let console = DebugConsole::new();
        assert!(!console.is_open());
        assert!(!console.history.is_empty()); // Has welcome message
    }

    #[test]
    fn test_toggle() {
        let mut console = DebugConsole::new();
        console.toggle();
        assert!(console.is_open());
        console.toggle();
        assert!(!console.is_open());
    }

    #[test]
    fn test_add_output() {
        let mut console = DebugConsole::new();
        let initial_len = console.history.len();
        console.output("test message");
        assert_eq!(console.history.len(), initial_len + 1);
    }

    #[test]
    fn test_add_error() {
        let mut console = DebugConsole::new();
        console.error("error message");
        let last = console.history.back().unwrap();
        assert_eq!(last.kind, LineKind::Error);
        assert_eq!(last.text, "error message");
    }

    #[test]
    fn test_add_debug() {
        let mut console = DebugConsole::new();
        console.debug("debug info");
        let last = console.history.back().unwrap();
        assert_eq!(last.kind, LineKind::Debug);
    }

    #[test]
    fn test_history_max_size() {
        let mut console = DebugConsole::new();
        for i in 0..MAX_HISTORY + 50 {
            console.output(&format!("line {}", i));
        }
        assert!(console.history.len() <= MAX_HISTORY);
    }

    #[test]
    fn test_command_history_navigation() {
        let mut console = DebugConsole::new();
        console.command_history.push("cmd1".to_string());
        console.command_history.push("cmd2".to_string());
        console.command_history.push("cmd3".to_string());

        // Navigate up (older)
        console.navigate_history(true);
        assert_eq!(console.input, "cmd3");

        console.navigate_history(true);
        assert_eq!(console.input, "cmd2");

        console.navigate_history(true);
        assert_eq!(console.input, "cmd1");

        // Can't go further back
        console.navigate_history(true);
        assert_eq!(console.input, "cmd1");

        // Navigate down (newer)
        console.navigate_history(false);
        assert_eq!(console.input, "cmd2");

        console.navigate_history(false);
        assert_eq!(console.input, "cmd3");

        // Back to empty
        console.navigate_history(false);
        assert!(console.input.is_empty());
    }

    #[test]
    fn test_builtin_help() {
        let mut console = DebugConsole::new();
        let handled = process_builtin_command(&mut console, "help");
        assert!(handled);
        assert!(console.history.iter().any(|l| l.text.contains("Available commands")));
    }

    #[test]
    fn test_builtin_clear() {
        let mut console = DebugConsole::new();
        console.output("some text");
        let handled = process_builtin_command(&mut console, "clear");
        assert!(handled);
        assert!(console.history.is_empty());
    }

    #[test]
    fn test_builtin_echo() {
        let mut console = DebugConsole::new();
        let handled = process_builtin_command(&mut console, "echo hello world");
        assert!(handled);
        assert!(console.history.iter().any(|l| l.text == "hello world"));
    }

    #[test]
    fn test_builtin_time() {
        let mut console = DebugConsole::new();
        let handled = process_builtin_command(&mut console, "time");
        assert!(handled);
        assert!(console.history.iter().any(|l| l.text.contains("Unix time")));
    }

    #[test]
    fn test_unknown_command_not_handled() {
        let mut console = DebugConsole::new();
        let handled = process_builtin_command(&mut console, "unknown_cmd");
        assert!(!handled);
    }

    #[test]
    fn test_line_kind_variants() {
        assert_ne!(LineKind::Input, LineKind::Output);
        assert_ne!(LineKind::Error, LineKind::Debug);
    }
}

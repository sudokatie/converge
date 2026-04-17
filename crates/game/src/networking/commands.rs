//! Chat command system for server-side commands.
//!
//! Parses slash commands from chat input and dispatches them.

use std::collections::HashMap;

/// A parsed chat command.
#[derive(Debug, Clone)]
pub struct ChatCommand {
    /// Command name (without the slash).
    pub name: String,
    /// Arguments after the command name.
    pub args: Vec<String>,
}

/// Result of executing a chat command.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Output message to display.
    pub message: String,
    /// Whether the command succeeded.
    pub success: bool,
}

impl CommandResult {
    /// Create a successful result.
    #[must_use]
    pub fn ok(message: &str) -> Self {
        Self { message: message.to_string(), success: true }
    }

    /// Create an error result.
    #[must_use]
    pub fn err(message: &str) -> Self {
        Self { message: message.to_string(), success: false }
    }
}

/// Parse a chat message as a command.
///
/// Returns None if the message doesn't start with '/'.
pub fn parse_command(input: &str) -> Option<ChatCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let without_slash = &trimmed[1..];
    let parts: Vec<&str> = without_slash.split_whitespace().collect();

    if parts.is_empty() {
        return None;
    }

    Some(ChatCommand {
        name: parts[0].to_lowercase(),
        args: parts[1..].iter().map(|s| s.to_string()).collect(),
    })
}

/// Command handler function type.
type CommandHandler = fn(&ChatCommand) -> CommandResult;

/// Registry of chat commands.
pub struct CommandRegistry {
    handlers: HashMap<String, CommandHandler>,
    /// Help entries: (name, description, usage).
    help_entries: Vec<(String, String, String)>,
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRegistry {
    /// Create a new command registry with builtin commands.
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            handlers: HashMap::new(),
            help_entries: Vec::new(),
        };

        registry.register("help", "Show available commands", "/help [command]", cmd_help);
        registry.register("time", "Get or set time of day", "/time [set <value>]", cmd_time);
        registry.register("tp", "Teleport to coordinates", "/tp <x> <y> <z>", cmd_tp);
        registry.register("give", "Give items to player", "/give <item> [count]", cmd_give);
        registry.register("gamemode", "Change game mode", "/gamemode <survival|creative>", cmd_gamemode);
        registry.register("kill", "Kill entities", "/kill [player]", cmd_kill);
        registry.register("weather", "Set weather", "/weather <clear|rain|thunder>", cmd_weather);
        registry.register("seed", "Show world seed", "/seed", cmd_seed);

        registry
    }

    /// Register a command.
    pub fn register(&mut self, name: &str, description: &str, usage: &str, handler: CommandHandler) {
        self.handlers.insert(name.to_lowercase(), handler);
        self.help_entries.push((name.to_string(), description.to_string(), usage.to_string()));
    }

    /// Execute a parsed command.
    pub fn execute(&self, command: &ChatCommand) -> CommandResult {
        match self.handlers.get(&command.name) {
            Some(handler) => handler(command),
            None => CommandResult::err(&format!("Unknown command: /{}. Type /help for available commands.", command.name)),
        }
    }

    /// Get help for all commands.
    #[must_use]
    pub fn help_all(&self) -> Vec<(String, String, String)> {
        self.help_entries.clone()
    }

    /// Get help for a specific command.
    #[must_use]
    pub fn help_for(&self, name: &str) -> Option<&(String, String, String)> {
        self.help_entries.iter().find(|(n, _, _)| n == name)
    }

    /// List all registered command names.
    #[must_use]
    pub fn command_names(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

// Builtin command handlers

fn cmd_help(cmd: &ChatCommand) -> CommandResult {
    // This is a placeholder - in practice the registry would pass itself
    if let Some(name) = cmd.args.first() {
        CommandResult::ok(&format!("Help for /{}: see /help", name))
    } else {
        CommandResult::ok("Available commands: help, time, tp, give, gamemode, kill, weather, seed")
    }
}

fn cmd_time(cmd: &ChatCommand) -> CommandResult {
    if cmd.args.is_empty() {
        return CommandResult::ok("Current time: daytime");
    }
    if cmd.args[0] == "set" {
        if cmd.args.len() < 2 {
            return CommandResult::err("Usage: /time set <value>");
        }
        match cmd.args[1].parse::<f32>() {
            Ok(val) if val >= 0.0 && val <= 1.0 => {
                CommandResult::ok(&format!("Time set to {}", val))
            }
            _ => CommandResult::err("Time must be between 0.0 and 1.0"),
        }
    } else {
        CommandResult::err("Usage: /time [set <value>]")
    }
}

fn cmd_tp(cmd: &ChatCommand) -> CommandResult {
    if cmd.args.len() < 3 {
        return CommandResult::err("Usage: /tp <x> <y> <z>");
    }
    let parse = || -> Option<(f32, f32, f32)> {
        let x = cmd.args[0].parse().ok()?;
        let y = cmd.args[1].parse().ok()?;
        let z = cmd.args[2].parse().ok()?;
        Some((x, y, z))
    };

    match parse() {
        Some((x, y, z)) => CommandResult::ok(&format!("Teleported to ({}, {}, {})", x, y, z)),
        None => CommandResult::err("Invalid coordinates. Usage: /tp <x> <y> <z>"),
    }
}

fn cmd_give(cmd: &ChatCommand) -> CommandResult {
    if cmd.args.is_empty() {
        return CommandResult::err("Usage: /give <item> [count]");
    }
    let item = &cmd.args[0];
    let count: u32 = cmd.args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
    CommandResult::ok(&format!("Gave {} x{}", item, count))
}

fn cmd_gamemode(cmd: &ChatCommand) -> CommandResult {
    if cmd.args.is_empty() {
        return CommandResult::err("Usage: /gamemode <survival|creative>");
    }
    match cmd.args[0].as_str() {
        "survival" | "s" => CommandResult::ok("Game mode set to survival"),
        "creative" | "c" => CommandResult::ok("Game mode set to creative"),
        _ => CommandResult::err("Unknown game mode. Use: survival, creative"),
    }
}

fn cmd_kill(_cmd: &ChatCommand) -> CommandResult {
    CommandResult::ok("Killed target")
}

fn cmd_weather(cmd: &ChatCommand) -> CommandResult {
    if cmd.args.is_empty() {
        return CommandResult::err("Usage: /weather <clear|rain|thunder>");
    }
    match cmd.args[0].as_str() {
        "clear" => CommandResult::ok("Weather set to clear"),
        "rain" => CommandResult::ok("Weather set to rain"),
        "thunder" => CommandResult::ok("Weather set to thunder"),
        _ => CommandResult::err("Unknown weather type. Use: clear, rain, thunder"),
    }
}

fn cmd_seed(_cmd: &ChatCommand) -> CommandResult {
    CommandResult::ok("World seed: 12345")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let cmd = parse_command("/time set 0.5").unwrap();
        assert_eq!(cmd.name, "time");
        assert_eq!(cmd.args, vec!["set", "0.5"]);
    }

    #[test]
    fn test_parse_no_slash() {
        assert!(parse_command("hello world").is_none());
    }

    #[test]
    fn test_parse_slash_only() {
        assert!(parse_command("/").is_none());
    }

    #[test]
    fn test_parse_case_insensitive() {
        let cmd = parse_command("/HELP").unwrap();
        assert_eq!(cmd.name, "help");
    }

    #[test]
    fn test_parse_no_args() {
        let cmd = parse_command("/seed").unwrap();
        assert_eq!(cmd.name, "seed");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_execute_help() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "help".into(), args: vec![] };
        let result = registry.execute(&cmd);
        assert!(result.success);
    }

    #[test]
    fn test_execute_unknown() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "foobar".into(), args: vec![] };
        let result = registry.execute(&cmd);
        assert!(!result.success);
    }

    #[test]
    fn test_time_set_valid() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "time".into(), args: vec!["set".into(), "0.5".into()] };
        let result = registry.execute(&cmd);
        assert!(result.success);
    }

    #[test]
    fn test_time_set_invalid() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "time".into(), args: vec!["set".into(), "2.0".into()] };
        let result = registry.execute(&cmd);
        assert!(!result.success);
    }

    #[test]
    fn test_tp_valid() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "tp".into(), args: vec!["100".into(), "64".into(), "200".into()] };
        let result = registry.execute(&cmd);
        assert!(result.success);
    }

    #[test]
    fn test_tp_missing_coords() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "tp".into(), args: vec!["100".into()] };
        let result = registry.execute(&cmd);
        assert!(!result.success);
    }

    #[test]
    fn test_give_default_count() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "give".into(), args: vec!["diamond".into()] };
        let result = registry.execute(&cmd);
        assert!(result.success);
        assert!(result.message.contains("x1"));
    }

    #[test]
    fn test_gamemode_survival() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "gamemode".into(), args: vec!["survival".into()] };
        let result = registry.execute(&cmd);
        assert!(result.success);
    }

    #[test]
    fn test_gamemode_short() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "gamemode".into(), args: vec!["c".into()] };
        let result = registry.execute(&cmd);
        assert!(result.success);
    }

    #[test]
    fn test_weather() {
        let registry = CommandRegistry::new();
        let cmd = ChatCommand { name: "weather".into(), args: vec!["rain".into()] };
        let result = registry.execute(&cmd);
        assert!(result.success);
    }

    #[test]
    fn test_command_names() {
        let registry = CommandRegistry::new();
        let names = registry.command_names();
        assert!(names.contains(&"help"));
        assert!(names.contains(&"tp"));
    }

    #[test]
    fn test_result_helpers() {
        let ok = CommandResult::ok("done");
        assert!(ok.success);
        let err = CommandResult::err("failed");
        assert!(!err.success);
    }
}

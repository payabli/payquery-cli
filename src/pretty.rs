use serde_json::Value;
use colored::*;
use serde_yaml;

pub fn prettify_json(json_str: &str) -> Result<String, String> {
    serde_json::from_str::<Value>(json_str)
        .map_err(|e| e.to_string())
        .and_then(|parsed| serde_json::to_string_pretty(&parsed).map_err(|e| e.to_string()))
        .map(|formatted| colorize_output(&formatted))
}

pub fn prettify_yaml(json_str: &str) -> Result<String, String> {
    serde_json::from_str::<Value>(json_str)
        .map_err(|e| e.to_string())
        .and_then(|parsed| serde_yaml::to_string(&parsed).map_err(|e| e.to_string()))
        .map(|formatted| colorize_output(&formatted))
}

fn colorize_output(text: &str) -> String {
    text.lines()
        .map(|line| {
            let indent_level = line.chars().take_while(|c| c.is_whitespace()).count();
            let depth = indent_level / 2; // Assuming 2 spaces per indentation level for YAML

            let color = match depth {
                0 => Color::White,
                1 => Color::Green,
                2 => Color::Blue,
                _ => Color::Yellow,
            };

            line.color(color).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn fancy_status(status: &str) -> String {
    let status_code = status.split_whitespace().nth(1).unwrap_or("0");
    let (emoji, color) = match status_code.chars().next() {
        Some('2') => (Some("✅"), Color::Green),
        Some('3') => (Some("⚠️"), Color::Yellow),
        Some('4') => (Some("❌"), Color::Red),
        Some('5') => (Some("🔥"), Color::Red),
        _ => (None, Color::White),
    };

    let trimmed_status = status.trim();
    let boxed_message = boxed_message(emoji, trimmed_status);

    boxed_message.color(color).bold().to_string()
}

pub fn boxed_message(emoji: Option<&str>, message: &str) -> String {
    let emoji_str = emoji.unwrap_or("");
    let total_length = emoji_str.len() + message.len() + if emoji.is_some() { 2 } else { 0 }; // 2 for spaces around the emoji if present
    let box_width = total_length - 1; 

    let top_border =    format!("┌─{:─<width$}┐", "", width = box_width);
    let middle_line =   if emoji.is_some() {
        format!("│ {} {:<width$} │", emoji_str, message, width = message.len() + emoji_str.len() - 3)
    } else {
        format!("│ {:<width$} │", message, width = message.len() - 3)
    };
    let bottom_border = format!("└─{:─<width$}┘", "", width = box_width);

    format!(
        "{}\n\
         {}\n\
         {}",
        top_border, middle_line, bottom_border
    )
}

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

            let color = match depth % 5 {
                0 => Color::Red,
                1 => Color::Green,
                2 => Color::Blue,
                3 => Color::Yellow,
                _ => Color::Magenta,
            };

            line.color(color).to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}


use std::collections::HashMap;
use std::env;
use serde::de::Error as SerdeDeError;

use std::process;
mod pretty;

use crate::pretty::{prettify_json, prettify_yaml, fancy_status};
use reqwest::blocking::Client;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let format = args.get(0)
        .filter(|arg| arg.starts_with("--"))
        .map(|arg| arg.as_str())
        .unwrap_or("--json");

    let api_token = env::var("PAYABLI_API_TOKEN").unwrap_or_else(|_| {
        eprintln!("Error: PAYABLI_API_TOKEN environment variable must be set");
        process::exit(1);
    });

    let base_url = env::var("PAYABLI_ENVIRONMENT")
        .map_or("https://api-sandbox.payabli.com", |env| match env.as_str() {
            "production" => "https://api-payabli.com",
            "qa" => "https://api-qa.payabli.com",
            _ => "https://api-sandbox.payabli.com",
        });

    let start_index = args.iter().position(|arg| !arg.starts_with("--")).unwrap_or(0);
    let (route_parts, filter_args, sort_clause) = split_args(&args[start_index..]);
    let url = build_url(base_url, &route_parts);
    let query_params = parse_filters(&filter_args).unwrap_or_else(|e| {
        eprintln!("Error parsing filters: {}", e);
        process::exit(1);
    });

    let client = Client::new();
    let result = client.get(&url)
        .header("requestToken", api_token)
        .query(&query_params)
        .send();

    match result {
        Ok(resp) => {
            println!("{}", fancy_status(&format!("Status: {}", resp.status())));
            match resp.text() {
                Ok(text) => {
                    let records: Vec<serde_json::Value> = serde_json::from_str(&text)
                        .and_then(|v: serde_json::Value| v["Records"].as_array().cloned().ok_or_else(|| "Invalid response format".to_string()).map_err(|e| SerdeDeError::custom(e)))
                        .unwrap_or_else(|e| {
                            eprintln!("Error parsing response: {}", e);
                            process::exit(1);
                        });

                    let mut records = records.clone();

                    if let Some((key, order)) = sort_clause {
                        records.sort_by(|a, b| {
                            let default_value = serde_json::Value::default();
                            let a_value = get_nested_value(&a, &key).unwrap_or(&default_value);
                            let b_value = get_nested_value(&b, &key).unwrap_or(&default_value);
                            if order == "desc" {
                                b_value.to_string().cmp(&a_value.to_string())
                            } else {
                                a_value.to_string().cmp(&b_value.to_string())
                            }
                        });
                    }

                    let sorted_text = serde_json::to_string(&records).unwrap_or_else(|e| {
                        eprintln!("Error serializing sorted records: {}", e);
                        process::exit(1);
                    });

                    match format {
                        "--json" => process_output(prettify_json(&sorted_text)),
                        "--yaml" => process_output(prettify_yaml(&sorted_text)),
                        _ => eprintln!("Unsupported format"),
                    }
                }
                Err(e) => eprintln!("Error reading response: {}", e),
            }
        }
        Err(e) => eprintln!("Request failed: {}", e),
    }
}
            
fn get_nested_value<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    key.split('.').fold(Some(value), |acc, k| acc.and_then(|v| v.get(k)))
}

fn get_keyword_map() -> HashMap<&'static str, String> {
    //let now = Utc::now();
    //let today = now.date_naive().and_hms_opt(0, 0, 0).unwrap(); // Use `and_hms_opt` with `unwrap()`
    //let yesterday = today - Duration::try_days(1).unwrap(); // Use `try_days` with `unwrap()`

    let map = HashMap::new();
    //map.insert("now", now.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    //map.insert("today", today.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    //map.insert("yesterday", yesterday.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());

    map
}

fn replace_keywords(arg: &str, keyword_map: &HashMap<&str, String>) -> String {
    keyword_map.get(arg).cloned().unwrap_or_else(|| arg.to_string())
}

fn process_output(result: Result<String, String>) {
    match result {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Error processing output: {}", e),
    }
}

fn split_args(args: &[String]) -> (Vec<String>, Vec<String>, Option<(String, String)>) {
    let where_pos = args.iter().position(|x| x == "where");
    let by_pos = args.iter().position(|x| x == "by");

    let route_parts = match (where_pos, by_pos) {
        (Some(w_pos), Some(b_pos)) if b_pos < w_pos => args[..b_pos].to_vec(),
        (Some(w_pos), _) => args[..w_pos].to_vec(),
        (None, Some(b_pos)) => args[..b_pos].to_vec(),
        (None, None) => args.to_vec(),
    };

    let filter_args = match (where_pos, by_pos) {
        (Some(w_pos), Some(b_pos)) if b_pos > w_pos => args[w_pos + 1..b_pos].to_vec(),
        (Some(w_pos), _) => args[w_pos + 1..].to_vec(),
        _ => vec![],
    };

    let sort_clause = by_pos.map(|pos| {
        let joined_args = args[pos + 1..].join(" ");
        let parts: Vec<&str> = joined_args.split_whitespace().collect();
        (parts[0].to_string(), parts.get(1).cloned().unwrap_or("asc").to_string())
    });

    (route_parts, filter_args, sort_clause)
}

fn build_url(base: &str, route_parts: &[String]) -> String {
    let path = route_parts.join("/");
    format!("{}/api/Query/{}/", base, path)
}

fn parse_filters(args: &[String]) -> Result<Vec<(String, String)>, String> {
    let keyword_map = get_keyword_map();
    let replaced_args: Vec<String> = args.iter()
        .map(|arg| replace_keywords(arg, &keyword_map))
        .collect();

    replaced_args.join(" ")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_filter_clause)
        .collect()
}

fn parse_filter_clause(clause: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = clause.split_whitespace().collect();
    match parts.as_slice() {
        [field, condition, value] => get_condition(condition)
            .map(|cond| (format!("{}({})", field, cond), value.to_string()))
            .ok_or_else(|| format!("Invalid condition '{}' for field '{}'", condition, field)),
        [field, value] => Ok((format!("{}(eq)", field), value.to_string())),
        _ => Err(format!("Invalid filter clause: '{}'", clause)),
    }
}

fn get_condition(arg: &str) -> Option<&'static str> {
    match arg {
        "=" | "eq" => Some("eq"),
        ">" | "gt" => Some("gt"),
        ">=" | "ge" => Some("ge"),
        "<" | "lt" => Some("lt"),
        "<=" | "le" => Some("le"),
        "!=" | "ne" => Some("ne"),
        "contains" | "ct" => Some("ct"),
        "not_contains" | "nct" => Some("nct"),
        "in" => Some("in"),
        "not_in" | "nin" => Some("nin"),
        _ => None,
    }
}

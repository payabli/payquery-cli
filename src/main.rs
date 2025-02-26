use std::env;
use std::process;
use serde::de::Error as SerdeDeError;
use reqwest::blocking::Client;
use crate::pretty::{prettify_json, prettify_yaml, fancy_status};
use crate::args::{split_args, build_url, parse_filters, get_nested_value};

mod pretty;
mod args;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (only_records, format) = parse_args(&args);
    let api_token = get_env_var("PAYABLI_API_TOKEN");
    let base_url = get_base_url();
    let (route_parts, filter_args, sort_clause) = process_route_args(&args);
    let url = build_url(&base_url, &route_parts);
    let query_params = parse_filters(&filter_args).unwrap_or_else(|e| handle_error(format!("Error parsing filters: {}", e)));

    let client = Client::new();
    let result = client.get(&url)
        .header("requestToken", api_token)
        .query(&query_params)
        .send();

    match result {
        Ok(resp) => handle_response(resp, only_records, sort_clause, format, &args),
        Err(e) => eprintln!("Request failed: {}", e),
    }
}

fn parse_args(args: &[String]) -> (Option<usize>, &str) {
    let mut only_records = None;
    let mut format = "--json";

    let mut args_iter = args.iter().peekable();
    while let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--only" => {
                if let Some(value) = args_iter.peek() {
                    only_records = value.parse::<usize>().ok();
                    args_iter.next(); // Consume the number after --only
                }
            }
            "--yaml" => {
                format = "--yaml";
            }
            _ => {}
        }
    }

    (only_records, format)
}

fn get_env_var(var: &str) -> String {
    env::var(var).unwrap_or_else(|_| handle_error(format!("Error: {} environment variable must be set", var)))
}

fn get_base_url() -> &'static str {
    match env::var("PAYABLI_ENVIRONMENT").as_deref() {
        Ok("production") => "https://api-payabli.com",
        Ok("qa") => "https://api-qa.payabli.com",
        _ => "https://api-sandbox.payabli.com",
    }
}

fn process_route_args(args: &[String]) -> (Vec<String>, Vec<String>, Option<(String, String)>) {
    let start_index = args.iter().position(|arg| !arg.starts_with("--") && !arg.parse::<usize>().is_ok()).unwrap_or(0);
    let (mut route_parts, filter_args, sort_clause) = split_args(&args[start_index..]);

    if let Some(last_param) = route_parts.last() {
        if last_param == "org" {
            let org_id = get_env_var("PAYABLI_ORG_ID");
            route_parts.push(org_id);
        } else if !last_param.chars().any(|c| c.is_digit(10)) {
            let entrypoint = get_env_var("PAYABLI_ENTRYPOINT");
            route_parts.push(entrypoint);
        }
    }

    (route_parts, filter_args, sort_clause)
}

fn handle_response(resp: reqwest::blocking::Response, only_records: Option<usize>, sort_clause: Option<(String, String)>, format: &str, args: &[String]) {
    println!("{}", fancy_status(&format!("Status: {}", resp.status())));
    match resp.text() {
        Ok(text) => process_text(&text, only_records, sort_clause, format, args),
        Err(e) => eprintln!("Error reading response: {}", e),
    }
}

fn process_text(text: &str, only_records: Option<usize>, sort_clause: Option<(String, String)>, format: &str, args: &[String]) {
    let records: Vec<serde_json::Value> = serde_json::from_str(text)
        .and_then(|v: serde_json::Value| v["Records"].as_array().cloned().ok_or_else(|| "Invalid response format".to_string()).map_err(|e| SerdeDeError::custom(e)))
        .unwrap_or_else(|e| handle_error(format!("Error parsing response: {}", e)));

    let records = only_records.map_or(records.clone(), |limit| records.iter().take(limit).cloned().collect());
    let records = sort_clause.as_ref().map_or(records.clone(), |(key, order)| sort_records(records.clone(), key, order));

    if sort_clause.as_ref().map_or(false, |(_, _)| args.contains(&"crop".to_string())) {
        records.iter().filter_map(|record| get_nested_value(record, &sort_clause.as_ref().unwrap().0)).for_each(|value| println!("{}", value));
    } else {
        let sorted_text = serde_json::to_string(&records).unwrap_or_else(|e| handle_error(format!("Error serializing sorted records: {}", e)));
        match format {
            "--json" => process_output(prettify_json(&sorted_text)),
            "--yaml" => process_output(prettify_yaml(&sorted_text)),
            _ => eprintln!("Unsupported format"),
        }
    }
}

fn sort_records(records: Vec<serde_json::Value>, key: &str, order: &str) -> Vec<serde_json::Value> {
    let mut records = records;
    records.sort_by(|a, b| {
        let default_value = serde_json::Value::default();
        let a_value = get_nested_value(&a, key).unwrap_or(&default_value);
        let b_value = get_nested_value(&b, key).unwrap_or(&default_value);
        if order == "desc" {
            b_value.to_string().cmp(&a_value.to_string())
        } else {
            a_value.to_string().cmp(&b_value.to_string())
        }
    });
    records
}

fn handle_error(message: String) -> ! {
    eprintln!("{}", message);
    process::exit(1);
}

fn process_output(result: Result<String, String>) {
    match result {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Error processing output: {}", e),
    }
}

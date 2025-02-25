use std::env;
use std::process;
mod pretty;

use crate::pretty::{prettify_json, prettify_yaml};


fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let format = if let Some(arg) = args.get(0) {
        if arg.starts_with("--") {
            arg.as_str()
        } else {
            "--json"
        }
    } else {
        "--json"
    };

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
    let (route_parts, filter_args) = split_args(&args[start_index..]);
    let url = build_url(base_url, &route_parts);
    let query_params = parse_filters(&filter_args).unwrap_or_else(|e| {
        eprintln!("Error parsing filters: {}", e);
        process::exit(1);
    });

    let client = reqwest::blocking::Client::new();
    match client.get(&url)
        .header("requestToken", api_token)
        .query(&query_params)
        .send()
    {
        Ok(resp) => {
            println!("Status: {}", resp.status());
            match resp.text() {
                Ok(text) => match format {
                    "--json" => process_output(prettify_json(&text)),
                    "--yaml" => process_output(prettify_yaml(&text)),
                    
                    _ => eprintln!("Unsupported format"),
                },
                Err(e) => eprintln!("Error reading response: {}", e),
            }
        }
        Err(e) => eprintln!("Request failed: {}", e),
    }
}

fn process_output(result: Result<String, String>) {
    match result {
        Ok(output) => println!("{}", output),
        Err(e) => eprintln!("Error processing output: {}", e),
    }
}



fn split_args(args: &[String]) -> (Vec<String>, Vec<String>) {
    args.iter()
        .position(|x| x == "where")
        .map(|pos| (args[..pos].to_vec(), args[pos + 1..].to_vec()))
        .unwrap_or((args.to_vec(), vec![]))
}

fn build_url(base: &str, route_parts: &[String]) -> String {
    let path = route_parts.join("/");
    format!("{}/api/Query/{}/", base, path)
}

fn parse_filters(args: &[String]) -> Result<Vec<(String, String)>, String> {
    args.join(" ")
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

use std::env;
use serde::de::Error as SerdeDeError;

use std::process;
mod pretty;
mod args;

use crate::pretty::{prettify_json, prettify_yaml, fancy_status};
use reqwest::blocking::Client;
use crate::args::{split_args, build_url, parse_filters, get_nested_value};

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut only_records: Option<usize> = None;
    let mut format = "--json";

    let mut args_iter = args.iter().peekable();
    while let Some(arg) = args_iter.next() {
        if arg == "--only" {
            if let Some(value) = args_iter.peek() {
                only_records = value.parse::<usize>().ok();
            args_iter.next(); // Consume the number after --only
            }
        } else if arg.starts_with("--") {
            format = arg;
        }
    }

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

    let start_index = args.iter().position(|arg| !arg.starts_with("--") && !arg.parse::<usize>().is_ok()).unwrap_or(0);
    let (mut route_parts, filter_args, mut sort_clause) = split_args(&args[start_index..]);
    let mut crop = false;

    if let Some((key, order)) = &sort_clause {
        if args.contains(&"crop".to_string()) {
            crop = true;
            sort_clause = Some((key.clone(), order.clone()));
        }
    }
    if let Some(last_param) = route_parts.last() {
        if last_param == "org" {
            let org_id = env::var("PAYABLI_ORG_ID").unwrap_or_else(|_| {
                eprintln!("Error: PAYABLI_ORG_ID environment variable must be set");
                process::exit(1);
            });
            route_parts.push(org_id);
        } else {
            let entrypoint = env::var("PAYABLI_ENTRYPOINT").unwrap_or_else(|_| {
                eprintln!("Error: PAYABLI_ENTRYPOINT environment variable must be set");
                process::exit(1);
            });
            route_parts.push(entrypoint);
        }
    }

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

                    if let Some(limit) = only_records {
                        records.truncate(limit);
                    }

                    if let Some((ref key, ref order)) = sort_clause {
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

                    if crop {
                        for record in &records {
                            if let Some((key, _)) = &sort_clause {
                                if let Some(value) = get_nested_value(record, key) {
                                    println!("{}", value);
                                }
                            }
                        }
                    } else {
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
                }
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


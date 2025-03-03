use std::env;
use std::process;
use serde::de::Error as SerdeDeError;
use reqwest::blocking::Client;
use crate::pretty::{prettify_json, prettify_yaml, fancy_status, boxed_message};
use crate::args::{split_args, build_url, parse_filters, get_nested_value, extract_only_clause, extract_for_clause, replace_keywords_in_args};
use crate::config::{Config, EnvironmentConfig};

mod pretty;
mod args;
mod config;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.contains(&"help".to_string()) {
        print_help();
        return;
    }
    if args.contains(&"list".to_string()) {
        list_configs();
        return;
    }
    if args.contains(&"new".to_string()) {
        Config::create_new_config();
        return;
    }
    let replaced_args_string = replace_keywords_in_args(&args);
    let replaced_args: Vec<String> = replaced_args_string.split_whitespace().map(String::from).collect();
    let (format, quiet) = parse_args(&replaced_args);
    let config_name = extract_for_clause(&replaced_args);
    let only_records = extract_only_clause(&replaced_args);
    let config = Config::load();
    let env_config = config.environments.get(&config_name).unwrap_or_else(|| {
        handle_error(format!("Configuration '{}' not found. Please provide a valid configuration name.", config_name))
    });
    let api_token = &env_config.api_token;
    let base_url = get_base_url(&env_config.environment);
    let (route_parts, filter_args, sort_clause) = process_route_args(&replaced_args, &env_config);
    let url = build_url(&base_url, &route_parts);
    let query_params = parse_filters(&filter_args).unwrap_or_else(|e| handle_error(format!("Error parsing filters: {}", e)));

    let client = Client::new();
    let result = client.get(&url)
        .header("requestToken", api_token)
        .query(&query_params)
        .send();

    if !quiet {
        println!("{}", boxed_message(Some("🔧"), &format!("Config: {}", config_name)));
        println!("{}", boxed_message(Some("🚀"), &url));
    }

    match result {
        Ok(resp) => handle_response(resp, only_records, sort_clause, format, &args, quiet),
        Err(e) => eprintln!("Request failed: {}", e),
    }
}

fn print_help() {
    println!(
        "payquery - A command-line interface for calling Payabli's Query APIs.\n\
        \n\
        USAGE:\n\
          payquery [SUBCOMMAND] [OPTIONS] [only N] API endpoint [CLAUSES]\n\
        \n\
        SUBCOMMANDS:\n\
          new                   Create a new configuration.\n\
          list                  List all available configurations.\n\
          help                  Show this help message.\n\
        \n\
        OPTIONS:\n\
          --json                Output in JSON format (default).\n\
          --yaml                Output in YAML format.\n\
          --quiet               Don't output information besides the query result.\n\
        \n\
        CLAUSES:\n\
          only N ...            Limit the number of records to N.\n\
          (comes before the API endpoint)\n\
          ... for NAME          Use the configuration named NAME.\n\
          ... where FILTERS     FILTER records based on the given conditions.\n\
          (https://docs.payabli.com/developer-guides/reporting-filters-and-conditions-reference)\n\
          ... by FIELD          Sort records by FIELD in ascending order.\n\
          ... by FIELD desc     Sort records by FIELD in descending order.\n\
          ... crop              Output only the sorted field values.\n\
          (must come after a BY clause)\n\
        \n\
        EXAMPLES:\n\
          payquery new\n\
          payquery list\n\
          payquery only 5 transactions\n\
          payquery chargebacks where method eq card\n\
          payquery batches for ISV_Pizzabli by TransactionDate\n\
          payquery only 10 customers for ISV_Pizzabli where firstname eq John by Lastname crop\n\
        \n\
        CONFIGURATION:\n\
          Configurations are stored in a YAML file located in your home directory as 'payquery.yml'.\n\
          Each configuration contains API token, organization ID, entrypoint, and environment.\n\
          Use the 'new' subcommand to create or update configurations.\n"
    );
}

fn list_configs() {
    let config = Config::load();
    println!("Available configurations:");
    for (name, _) in config.environments {
        println!("  - {}", name);
    }
    println!();
}

fn parse_args(args: &[String]) -> (&str, bool) {
    let mut format = "--json";
    let mut quiet = false;

    for arg in args {
        match arg.as_str() {
            "--yaml" => {
                format = "--yaml";
            }
            "--quiet" => {
                quiet = true;
            }
            _ => {}
        }
    }

    (format, quiet)
}

fn get_base_url(environment: &str) -> &'static str {
    match environment {
        "production" => "https://api-payabli.com",
        "qa" => "https://api-qa.payabli.com",
        _ => "https://api-sandbox.payabli.com",
    }
}



fn process_route_args(args: &[String], env_config: &EnvironmentConfig) -> (Vec<String>, Vec<String>, Option<(String, String)>) {
    let start_index = args.iter().position(|arg| !arg.starts_with("--") && !arg.parse::<usize>().is_ok()).unwrap_or(0);
    let (mut route_parts, filter_args, sort_clause) = split_args(&args[start_index..]);

    if let Some(last_param) = route_parts.last() {
        if last_param == "org" {
            let org_id = env_config.org_id.clone();
            route_parts.push(org_id);
        } else if !last_param.chars().any(|c| c.is_digit(10)) {
            let entrypoint = env_config.entrypoint.clone();
            route_parts.push(entrypoint);
        }
    }

    (route_parts, filter_args, sort_clause)
}

fn handle_response(resp: reqwest::blocking::Response, only_records: Option<usize>, sort_clause: Option<(String, String)>, format: &str, args: &[String], quiet: bool) {
    if !quiet {
        println!("{}", fancy_status(&format!("Status: {}", resp.status())));
    }
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

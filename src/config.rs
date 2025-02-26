use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::io::{self, Write};
use colored::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub environments: std::collections::HashMap<String, EnvironmentConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnvironmentConfig {
    pub api_token: String,
    pub org_id: String,
    pub entrypoint: String,
    pub environment: String,
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        if config_path.exists() {
            let config_content = fs::read_to_string(config_path).expect("Failed to read config file");
            serde_yaml::from_str(&config_content).expect("Failed to parse config file")
        } else {
            Self::create_default_config()
        }
    }

    fn get_config_path() -> PathBuf {
        dirs::home_dir().expect("Failed to get home directory").join("payquery.yml")
    }

    fn create_default_config() -> Self {
        println!("{}", "No configuration found. Let's create one!".bold().green());
        let default_config = Self::prompt_for_config("default");
        let mut environments = std::collections::HashMap::new();
        environments.insert("default".to_string(), default_config.clone());

        let config = Config {
            environments,
        };

        let config_path = Self::get_config_path();
        let config_content = serde_yaml::to_string(&config).expect("Failed to serialize config");
        fs::write(config_path, config_content).expect("Failed to write config file");

        config
    }

    pub fn create_new_config() {
        let config_path = Self::get_config_path();
        let mut config = if config_path.exists() {
            let config_content = fs::read_to_string(&config_path).expect("Failed to read config file");
            serde_yaml::from_str(&config_content).expect("Failed to parse config file")
        } else {
            Self::create_default_config()
        };

        let config_name = Self::prompt("Enter new configuration name: ");
        let new_config = Self::prompt_for_config(&config_name);
        config.environments.insert(config_name, new_config);

        let config_content = serde_yaml::to_string(&config).expect("Failed to serialize config");
        fs::write(config_path, config_content).expect("Failed to write config file");

        println!("{}", "New configuration created successfully!".bold().green());
    }


    fn prompt_for_config(name: &str) -> EnvironmentConfig {
        println!("{}", format!("Creating configuration for '{}'", name).bold().blue());
        let api_token = Self::prompt("Enter API token: ");
        let org_id = Self::prompt("Enter Org ID: ");
        let entrypoint = Self::prompt("Enter Entrypoint: ");
        let environment = Self::prompt("Enter Environment (production/qa/sandbox): ");

        EnvironmentConfig {
            api_token,
            org_id,
            entrypoint,
            environment,
        }
    }

    fn prompt(message: &str) -> String {
        print!("{}", message);
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");
        input.trim().to_string()
    }
}


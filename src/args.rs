use std::collections::HashMap;
use chrono::{Utc, Duration, Datelike, NaiveDate, Month};
use std::str::FromStr;

pub fn get_keyword_map() -> HashMap<&'static str, String> {
    let now = Utc::now();
    let today = now.date_naive().and_hms_opt(0, 0, 0).unwrap(); 
    let yesterday = today - Duration::try_days(1).unwrap(); 
    let tomorrow = today + Duration::try_days(1).unwrap(); 

    let mut map = HashMap::new();
    map.insert("now", now.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("today", today.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("yesterday", yesterday.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("tomorrow", tomorrow.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());

    // Add time frame keywords
    let start_of_week = today - Duration::days(today.weekday().num_days_from_monday() as i64);
    let start_of_next_week = start_of_week + Duration::weeks(1);
    let start_of_last_week = start_of_week - Duration::weeks(1);

    let start_of_month = today.with_day(1).unwrap();
    let start_of_next_month = start_of_month + Duration::days(31);
    let start_of_last_month = start_of_month - Duration::days(1);

    let start_of_year = today.with_ordinal(1).unwrap();
    let start_of_next_year = start_of_year + Duration::days(365);
    let start_of_last_year = start_of_year - Duration::days(1);

    map.insert("this week", start_of_week.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("next week", start_of_next_week.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("last week", start_of_last_week.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());

    map.insert("this month", start_of_month.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("next month", start_of_next_month.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("last month", start_of_last_month.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());

    map.insert("this year", start_of_year.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("next year", start_of_next_year.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    map.insert("last year", start_of_last_year.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());

    map
}

pub fn replace_keywords(arg: &str, keyword_map: &HashMap<&str, String>) -> String {
    let mut replaced_arg = arg.to_string();
    let mut sorted_keywords: Vec<_> = keyword_map.keys().collect();
    sorted_keywords.sort_by_key(|k| std::cmp::Reverse(k.len()));
    for keyword in sorted_keywords {
        if let Some(replacement) = keyword_map.get(keyword) {
            replaced_arg = replaced_arg.replace(keyword, replacement);
        }
    }
    replaced_arg
}

pub fn extract_only_clause(args: &[String]) -> Option<usize> {
    let only_pos = args.iter().position(|x| x == "only");
    only_pos.and_then(|pos| args.get(pos + 1).and_then(|value| value.parse::<usize>().ok()))
}

pub fn extract_for_clause(args: &[String]) -> String {
    let for_pos = args.iter().position(|x| x == "for");
    for_pos.and_then(|pos| args.get(pos + 1).cloned()).unwrap_or_else(|| "default".to_string())
}

pub fn split_args(args: &[String]) -> (Vec<String>, Vec<String>, Option<(String, String)>) {
    let only_pos = args.iter().position(|x| x == "only").map(|pos| pos + 2);
    let for_pos = args.iter().position(|x| x == "for");
    let where_pos = args.iter().position(|x| x == "where");
    let by_pos = args.iter().position(|x| x == "by");

    let first_clause_pos = [for_pos, where_pos, by_pos]
        .iter()
        .filter_map(|&pos| pos)
        .min();
    
    let route_parts = match (only_pos, first_clause_pos) {
        (Some(only_pos), Some(first_pos)) => args[only_pos..first_pos].to_vec(),
        (Some(only_pos), None) => args[only_pos..].to_vec(),
        (None, Some(first_pos)) => args[..first_pos].to_vec(),
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

pub fn build_url(base: &str, route_parts: &[String]) -> String {
    let path = route_parts.join("/");
    format!("{}/api/Query/{}/", base, path)
}

pub fn replace_keywords_in_args(args: &[String]) -> String {
    let args_string = args.join(" ");
    let keyword_map = get_keyword_map();
    let replaced_args = replace_keywords(&args_string, &keyword_map);
    replace_human_readable_dates(&replaced_args)
}

fn replace_human_readable_dates(input: &str) -> String {
    let mut result = input.to_string();
    let re = regex::Regex::new(r"(?i)\b([A-Za-z]+) (\d{1,2})(?: (\d{4}))?(?: @ (\d{1,2}):(\d{2}))?\b").unwrap();
    let now = Utc::now();
    let current_year = now.year();

    for cap in re.captures_iter(input) {
        let month_str = &cap[1];
        let day: u32 = cap[2].parse().unwrap();
        let year: i32 = cap.get(3).map_or(current_year, |m| m.as_str().parse().unwrap());

        if let Ok(month) = Month::from_str(month_str) {
            if let Some(date) = NaiveDate::from_ymd_opt(year, month.number_from_month(), day) {
                let hour: u32 = cap.get(4).map_or(0, |m| m.as_str().parse().unwrap());
                let minute: u32 = cap.get(5).map_or(0, |m| m.as_str().parse().unwrap());
                let datetime = date.and_hms_opt(hour, minute, 0).unwrap();
                let utc_datetime = datetime.format("%Y-%m-%dT%H:%M:%S%.3f").to_string();
                result = result.replace(&cap[0], &utc_datetime);
            }
        }
    }

    result
}

pub fn parse_filters(args: &[String]) -> Result<Vec<(String, String)>, String> {
    let replaced_args_string = replace_keywords_in_args(args);

    replaced_args_string.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_filter_clause)
        .collect()
}

pub fn parse_filter_clause(clause: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = clause.split_whitespace().collect();
    match parts.as_slice() {
        [field, condition, value] => get_condition(condition)
            .map(|cond| (format!("{}({})", field, cond), value.to_string()))
            .ok_or_else(|| format!("Invalid condition '{}' for field '{}'", condition, field)),
        [field, value] => Ok((format!("{}(eq)", field), value.to_string())),
        _ => Err(format!("Invalid filter clause: '{}'", clause)),
    }
}

pub fn get_condition(arg: &str) -> Option<&'static str> {
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
            
pub fn get_nested_value<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a serde_json::Value> {
    key.split('.').fold(Some(value), |acc, k| acc.and_then(|v| v.get(k)))
}

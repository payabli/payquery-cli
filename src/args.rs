use std::collections::HashMap;

pub fn get_keyword_map() -> HashMap<&'static str, String> {
    //let now = Utc::now();
    //let today = now.date_naive().and_hms_opt(0, 0, 0).unwrap(); // Use `and_hms_opt` with `unwrap()`
    //let yesterday = today - Duration::try_days(1).unwrap(); // Use `try_days` with `unwrap()`

    let map = HashMap::new();
    //map.insert("now", now.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    //map.insert("today", today.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());
    //map.insert("yesterday", yesterday.format("%Y-%m-%dT%H:%M:%S%.3f").to_string());

    map
}

pub fn extract_only_clause(args: &[String]) -> Option<usize> {
    let only_pos = args.iter().position(|x| x == "only");
    only_pos.and_then(|pos| args.get(pos + 1).and_then(|value| value.parse::<usize>().ok()))
}

pub fn extract_for_clause(args: &[String]) -> String {
    let for_pos = args.iter().position(|x| x == "for");
    for_pos.and_then(|pos| args.get(pos + 1).cloned()).unwrap_or_else(|| "default".to_string())
}

pub fn replace_keywords(arg: &str, keyword_map: &HashMap<&str, String>) -> String {
    keyword_map.get(arg).cloned().unwrap_or_else(|| arg.to_string())
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

pub fn parse_filters(args: &[String]) -> Result<Vec<(String, String)>, String> {
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

use crate::model::ModelStat;

pub fn compact_number(value: u64) -> String {
    let n = value as f64;
    if value >= 1_000_000_000 {
        format!("{:.1}B", n / 1_000_000_000.0)
    } else if value >= 1_000_000 {
        format!("{:.1}M", n / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", n / 1_000.0)
    } else {
        value.to_string()
    }
}

pub fn comma(value: u64) -> String {
    let s = value.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

pub fn dollars(value: f64) -> String {
    format!("${value:.2}")
}

pub fn display_cost(value: f64, formatted: &Option<String>) -> String {
    formatted.clone().unwrap_or_else(|| dollars(value))
}

pub fn sorted_models(rows: &[ModelStat]) -> Vec<ModelStat> {
    let mut rows = rows.to_vec();
    rows.sort_by(|a, b| {
        b.cost
            .partial_cmp(&a.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

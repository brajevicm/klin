use std::fmt;

use serde_json::{Map, Value};

use crate::config::{Config, Error};

/// The value a measure may reach, either a number a person pinned or the lowest step of a dated
/// schedule that is due today. Every gate reads its ceiling here, so both shapes are accepted
/// everywhere, and every line that names the ceiling names the step that set it.
pub struct Ceiling {
    pub value: u64,
    pub step: Option<String>,
}

impl Ceiling {
    /// What a line adds after the number when a dated step set it, and nothing when a person
    /// pinned the number.
    pub fn note(&self) -> String {
        match &self.step {
            Some(date) => format!(" (the {date} step)"),
            None => String::new(),
        }
    }
}

impl fmt::Display for Ceiling {
    fn fmt(&self, out: &mut fmt::Formatter) -> fmt::Result {
        write!(out, "{}{}", self.value, self.note())
    }
}

pub fn read(
    config: &Config,
    section: &str,
    key: &str,
    value: &Value,
    unit: &str,
) -> Result<Ceiling, Error> {
    if let Some(value) = value.as_u64() {
        return Ok(Ceiling { value, step: None });
    }
    let steps = value.as_object().ok_or_else(|| {
        config.malformed(section, key, &format!("{unit} or an object of dated steps"))
    })?;
    let today = today()?;
    let (value, date) = due(config, section, key, steps, unit, &today)?
        .into_iter()
        .min()
        .ok_or_else(|| {
            Error(format!(
                "{}: the \"{section}\" \"{key}\" schedule has no step due on {today}, so the \
                 gate would have no ceiling — add a step on or before today",
                config.file.display()
            ))
        })?;
    Ok(Ceiling {
        value,
        step: Some(date),
    })
}

/// The steps a schedule holds that today has reached, lowest value first once sorted. A step
/// higher than an earlier one is allowed and never wins.
fn due(
    config: &Config,
    section: &str,
    key: &str,
    steps: &Map<String, Value>,
    unit: &str,
    today: &str,
) -> Result<Vec<(u64, String)>, Error> {
    let mut out = Vec::new();
    for (date, step) in steps {
        if !is_date(date) {
            return Err(step_error(
                config,
                section,
                key,
                date,
                "a date as YYYY-MM-DD",
            ));
        }
        let step = step
            .as_u64()
            .ok_or_else(|| step_error(config, section, key, date, unit))?;
        if date.as_str() <= today {
            out.push((step, date.clone()));
        }
    }
    Ok(out)
}

/// The dated steps in force, for a line that names no ceiling of its own. Empty while every
/// ceiling is a number.
pub fn in_force(named: &[(&str, &Ceiling)]) -> String {
    let steps: Vec<String> = named
        .iter()
        .filter(|(_, ceiling)| ceiling.step.is_some())
        .map(|(key, ceiling)| format!("{key} {ceiling}"))
        .collect();
    match steps.is_empty() {
        true => String::new(),
        false => format!(" under {}", steps.join(" and ")),
    }
}

fn step_error(config: &Config, section: &str, key: &str, date: &str, must_be: &str) -> Error {
    Error(format!(
        "{}: the \"{section}\" \"{key}\" step \"{date}\" must be {must_be}",
        config.file.display()
    ))
}

/// Today in UTC, so two machines on one day choose the same step. `KLIN_TODAY` overrides it.
pub fn today() -> Result<String, Error> {
    let Ok(pinned) = std::env::var("KLIN_TODAY") else {
        return Ok(system_date());
    };
    match is_date(&pinned) {
        true => Ok(pinned),
        false => Err(Error(format!(
            "KLIN_TODAY is \"{pinned}\", which is not a date as YYYY-MM-DD"
        ))),
    }
}

fn system_date() -> String {
    let days = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() / 86_400) as i64;
    let (year, month, day) = civil(days);
    format!("{year:04}-{month:02}-{day:02}")
}

/// The civil date a count of days since 1970-01-01 names, by Howard Hinnant's algorithm.
fn civil(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let months = (5 * of_year + 2) / 153;
    let day = of_year - (153 * months + 2) / 5 + 1;
    let month = months + if months < 10 { 3 } else { -9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn is_date(text: &str) -> bool {
    let [year, month, day] = *text.split('-').collect::<Vec<&str>>() else {
        return false;
    };
    let (Some(_), Some(month), Some(day)) = (digits(year, 4), digits(month, 2), digits(day, 2))
    else {
        return false;
    };
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn digits(part: &str, width: usize) -> Option<u32> {
    match part.len() == width && part.bytes().all(|byte| byte.is_ascii_digit()) {
        true => part.parse().ok(),
        false => None,
    }
}

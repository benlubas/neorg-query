// this probably belongs in the rust norg parser...

use anyhow::anyhow;
use anyhow::{bail, Result};
use chrono::prelude::*;
use log::info;
use regex::Regex;

fn weekday_string(w: Weekday) -> String {
    match w {
        Weekday::Mon => "monday",
        Weekday::Tue => "tuesday",
        Weekday::Wed => "wednesday",
        Weekday::Thu => "thursday",
        Weekday::Fri => "friday",
        Weekday::Sat => "saturday",
        Weekday::Sun => "sunday",
    }
    .to_string()
}

// Parse a norg date string (as described in the spec) into a chrono DateTime UTC
pub fn parse(s: &str) -> Result<DateTime<Utc>> {
    let months = [
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];

    // `<day>?,? <day-of-month> <month> -?<year> <time> <timezone>`
    let re = Regex::new(
        r"(?x)
        ^(?<day>[[:alpha:]]+\>)?,?\s?
        (?<dom>\<\d{1,2})?(?:(?:(?:th|nd|st)\>)?|\>)\s?
        (?<month>\<[[:alpha:]]+\>)?\s?
        -?(?<year>\<\d{4,}\>)?\s?
        (?<full_time>(?<hour>\<\d{1,2}):(?<min>\d{2})(?:\.(?<seconds>\d{1,2}))?\>)?\s?
        (?<tz>[A-Z]{3,4})?$",
    )
    .unwrap();

    // Implementation Details:
    // - Skipping recurrences for now
    // - Since we're skipping recurrences, we can auto fill things with default values (except the
    // day/dom b/c those two depend on each other)

    if let Some(m) = re.captures(s) {
        let date = Local::now();
        let mut date = date.naive_utc();
        info!("current date in UTC: {date:?}");
        // TODO: respect time zones. Currently we just treat everything like it's local time and
        // store in UTC.
        // if let Some(tz) = m.name("tz") {
        // };

        if let Some(year) = m.name("year") {
            let year = year.as_str().parse::<i32>()?;
            date = date.with_year(year).ok_or(anyhow!("Incompatible Year"))?;
        }
        if let Some(month) = m.name("month") {
            let month = month.as_str();
            for (i, m) in months.iter().enumerate() {
                // NOTE: we're not checking for ambiguous month names, we just use the first month
                // that matches. Probably should fail here if it's ambiguous
                if m.starts_with(&month.to_lowercase()) {
                    date = date
                        .with_month0(i.try_into().unwrap())
                        .ok_or(anyhow!("Incompatible Month"))?;
                }
            }
        }
        if let (Some(hour), Some(min), Some(sec)) = (m.name("hour"), m.name("min"), m.name("sec")) {
            let hour = hour.as_str().parse::<i32>()?;
            date = date
                .with_hour(hour.try_into().unwrap())
                .ok_or(anyhow!("Incompatible Hour"))?;

            let min = min.as_str().parse::<i32>().unwrap();
            date = date
                .with_minute(min.try_into().unwrap())
                .ok_or(anyhow!("Incompatible Minute"))?;

            let sec = sec.as_str().parse::<i32>().unwrap();
            date = date
                .with_second(sec.try_into().unwrap())
                .ok_or(anyhow!("Incompatible Second"))?;
        }

        let days = (m.name("day"), m.name("dom"));
        let dom = match days {
            (None, None) => 1,
            (Some(_), None) => {
                bail!("Can't infer day of month from weekday");
            }
            (_, Some(dom)) => dom.as_str().parse::<u32>()?,
        };
        date = date.with_day(dom).ok_or(anyhow!("Incompatible Day"))?;

        if let Some(day) = days.0 {
            if !weekday_string(date.weekday()).starts_with(&day.as_str().to_lowercase()) {
                bail!("Weekday doesn't match day of month");
            }
        }

        info!("Parsed {s} into {date:?}");
        return Ok(Utc.from_local_datetime(&date).unwrap());
    }

    bail!("date string doesn't match date regex")
}

#[test]
fn vaild_date_parsing() {
    let examples = [
        "Thursday, 29 October 2020 16:20.10",
        "Thurs, 29 Oct 2020 12:43.31 GMT",
        "We 12th Jan 2022",
        "Friday, 03 Jan 2025 13:44.30 EST",
        "Wed 1st Jan 2025",
        "Wed 1st Jan", // NOTE, this case will break in 2026. I sure hope we implement proper
        // repetition parsing before then :D
        "1st Jan 2025",
        "1st Jan 2025 12:54 EST",
    ];
    examples
        .iter()
        .map(|s| format!("{:?}", parse(s).unwrap()))
        .collect::<Vec<_>>()
        .join("\n");
}

#[test]
fn invalid_date_parsing() {
    let examples = [
        "Thursday, October 29th 2020", // month and day out of order
        "Friday, 29th October 2020",   // Weekday and day of month don't match
        "30st February 2020",          // Day of month that doesn't exist
    ];

    examples.iter().for_each(|e| assert!(parse(e).is_err()))
}

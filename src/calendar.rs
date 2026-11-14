//! Dates for calculation sheets (see `calc.rs`): other calendars, time
//! zones by city or abbreviation, and the display of dates and times.
//!
//! Calendar conversions follow Reingold/Dershowitz, *Calendrical
//! Calculations*: every date maps to a fixed day number (R.D., day 1 =
//! 1 January of year 1 in the Gregorian calendar).

use crate::i18n::{lang, Lang};
use jiff::civil::Date;
use jiff::tz::{Offset, TimeZone};
use jiff::Zoned;
use std::collections::HashMap;
use std::sync::OnceLock;

// ===== fixed day numbers ====================================================

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

const UNIX_EPOCH_RD: i64 = 719163;

pub fn fixed(date: Date) -> i64 {
    days_from_civil(date.year() as i64, date.month() as i64, date.day() as i64) + UNIX_EPOCH_RD
}

pub fn from_fixed(rd: i64) -> Result<Date, String> {
    let (y, m, d) = civil_from_days(rd - UNIX_EPOCH_RD);
    date(y, m, d)
}

pub fn date(y: i64, m: i64, d: i64) -> Result<Date, String> {
    let bad = || t!("invalid date {}-{}-{}", y, m, d);
    let (y, m, d) = (i16::try_from(y).map_err(|_| bad())?, i8::try_from(m).map_err(|_| bad())?, i8::try_from(d).map_err(|_| bad())?);
    Date::new(y, m, d).map_err(|_| bad())
}

/// Julian day number of the date (the day starting at noon).
pub fn julian_day_number(date: Date) -> i64 {
    fixed(date) + 1721425
}

pub fn from_julian_day_number(jdn: i64) -> Result<Date, String> {
    from_fixed(jdn - 1721425)
}

// ===== Julian calendar ======================================================

const JULIAN_EPOCH: i64 = -1;

fn julian_leap(y: i64) -> bool {
    y.rem_euclid(4) == if y > 0 { 0 } else { 3 }
}

pub fn fixed_from_julian(y: i64, m: i64, d: i64) -> i64 {
    let yy = if y < 0 { y + 1 } else { y };
    let correction = if m <= 2 {
        0
    } else if julian_leap(y) {
        -1
    } else {
        -2
    };
    JULIAN_EPOCH - 1 + 365 * (yy - 1) + (yy - 1).div_euclid(4) + (367 * m - 362).div_euclid(12) + correction + d
}

pub fn julian_from_fixed(date: i64) -> (i64, i64, i64) {
    let approx = (4 * (date - JULIAN_EPOCH) + 1464).div_euclid(1461);
    let year = if approx <= 0 { approx - 1 } else { approx };
    let prior = date - fixed_from_julian(year, 1, 1);
    let correction = if date < fixed_from_julian(year, 3, 1) {
        0
    } else if julian_leap(year) {
        1
    } else {
        2
    };
    let month = (12 * (prior + correction) + 373).div_euclid(367);
    (year, month, date - fixed_from_julian(year, month, 1) + 1)
}

// ===== Hebrew calendar ======================================================
// Months are numbered from Nisan (1); the year starts with Tishrei (7).

fn hebrew_epoch() -> i64 {
    fixed_from_julian(-3761, 10, 7)
}

fn hebrew_leap(y: i64) -> bool {
    (7 * y + 1).rem_euclid(19) < 7
}

fn hebrew_last_month(y: i64) -> i64 {
    if hebrew_leap(y) {
        13
    } else {
        12
    }
}

fn hebrew_elapsed_days(y: i64) -> i64 {
    let months = (235 * y - 234).div_euclid(19);
    let parts = 12084 + 13753 * months;
    let days = 29 * months + parts.div_euclid(25920);
    if (3 * (days + 1)).rem_euclid(7) < 3 {
        days + 1
    } else {
        days
    }
}

fn hebrew_new_year(y: i64) -> i64 {
    let (ny0, ny1, ny2) = (hebrew_elapsed_days(y - 1), hebrew_elapsed_days(y), hebrew_elapsed_days(y + 1));
    let correction = if ny2 - ny1 == 356 {
        2
    } else if ny1 - ny0 == 382 {
        1
    } else {
        0
    };
    hebrew_epoch() + ny1 + correction
}

fn hebrew_last_day(m: i64, y: i64) -> i64 {
    let days = hebrew_new_year(y + 1) - hebrew_new_year(y);
    let short = [2, 4, 6, 10, 13].contains(&m)
        || (m == 12 && !hebrew_leap(y))
        || (m == 8 && !matches!(days, 355 | 385))
        || (m == 9 && matches!(days, 353 | 383));
    if short {
        29
    } else {
        30
    }
}

pub fn fixed_from_hebrew(y: i64, m: i64, d: i64) -> i64 {
    let mut f = hebrew_new_year(y) + d - 1;
    if m < 7 {
        f += (7..=hebrew_last_month(y)).map(|mm| hebrew_last_day(mm, y)).sum::<i64>();
        f += (1..m).map(|mm| hebrew_last_day(mm, y)).sum::<i64>();
    } else {
        f += (7..m).map(|mm| hebrew_last_day(mm, y)).sum::<i64>();
    }
    f
}

pub fn hebrew_from_fixed(date: i64) -> (i64, i64, i64) {
    let approx = ((date - hebrew_epoch()) as f64 / (35975351.0 / 98496.0)).floor() as i64 + 1;
    let mut year = approx - 1;
    while hebrew_new_year(year + 1) <= date {
        year += 1;
    }
    let mut month = if date < fixed_from_hebrew(year, 1, 1) { 7 } else { 1 };
    while month < 13 && date > fixed_from_hebrew(year, month, hebrew_last_day(month, year)) {
        month += 1;
    }
    (year, month, date - fixed_from_hebrew(year, month, 1) + 1)
}

fn hebrew_month_name(m: i64, y: i64) -> &'static str {
    const NAMES: [&str; 13] =
        ["Nisan", "Iyyar", "Sivan", "Tammuz", "Av", "Elul", "Tishrei", "Cheshvan", "Kislev", "Tevet", "Shevat", "Adar", "Adar II"];
    if m == 12 && hebrew_leap(y) {
        return "Adar I";
    }
    NAMES.get((m - 1) as usize).copied().unwrap_or("?")
}

// ===== Islamic calendar (arithmetic) =========================================

fn islamic_epoch() -> i64 {
    fixed_from_julian(622, 7, 16)
}

pub fn fixed_from_islamic(y: i64, m: i64, d: i64) -> i64 {
    d + 29 * (m - 1) + (6 * m - 1).div_euclid(11) + (y - 1) * 354 + (3 + 11 * y).div_euclid(30) + islamic_epoch() - 1
}

pub fn islamic_from_fixed(date: i64) -> (i64, i64, i64) {
    let year = (30 * (date - islamic_epoch()) + 10646).div_euclid(10631);
    let prior = date - fixed_from_islamic(year, 1, 1);
    let month = (11 * prior + 330).div_euclid(325);
    (year, month, date - fixed_from_islamic(year, month, 1) + 1)
}

const ISLAMIC_MONTHS: [&str; 12] = [
    "Muharram",
    "Safar",
    "Rabi al-Awwal",
    "Rabi al-Thani",
    "Jumada al-Ula",
    "Jumada al-Akhira",
    "Rajab",
    "Shaban",
    "Ramadan",
    "Shawwal",
    "Dhu al-Qada",
    "Dhu al-Hijja",
];

// ===== text =================================================================

const WEEKDAYS_EN: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
const WEEKDAYS_DE: [&str; 7] = ["Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag", "Sonntag"];
const MONTHS_EN: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const MONTHS_DE: [&str; 12] =
    ["Januar", "Februar", "März", "April", "Mai", "Juni", "Juli", "August", "September", "Oktober", "November", "Dezember"];

pub fn weekday_name(date: Date) -> &'static str {
    let i = (date.weekday().to_monday_one_offset() - 1) as usize;
    match lang() {
        Lang::De => WEEKDAYS_DE[i],
        Lang::En => WEEKDAYS_EN[i],
    }
}

fn weekday_short(date: Date) -> String {
    let name = weekday_name(date);
    match lang() {
        Lang::De => format!("{}.", name.chars().take(2).collect::<String>()),
        Lang::En => name.chars().take(3).collect(),
    }
}

fn month_name(m: i64) -> &'static str {
    let i = (m - 1).clamp(0, 11) as usize;
    match lang() {
        Lang::De => MONTHS_DE[i],
        Lang::En => MONTHS_EN[i],
    }
}

/// "Thu 2026-12-24" / "Do. 24.12.2026".
pub fn format_date(date: Date) -> String {
    match lang() {
        Lang::De => format!("{} {:02}.{:02}.{}", weekday_short(date), date.day(), date.month(), date.year()),
        Lang::En => format!("{} {}", weekday_short(date), date),
    }
}

/// A date with time and zone abbreviation; `time_only` leaves the date out
/// and notes a day change against `base` instead ("07:00 JST (+1 d)").
pub fn format_moment(at: &Zoned, time_only: bool, base: Option<Date>) -> String {
    let time = if at.second() != 0 && !time_only { at.strftime("%H:%M:%S") } else { at.strftime("%H:%M") }.to_string();
    let zone = at.strftime("%Z").to_string();
    if !time_only {
        return format!("{} {time} {zone}", format_date(at.date()));
    }
    let shift = base.map_or(0, |b| fixed(at.date()) - fixed(b));
    match shift {
        0 => format!("{time} {zone}"),
        s if s > 0 => format!("{time} {zone} (+{s} d)"),
        s => format!("{time} {zone} (−{} d)", -s),
    }
}

/// "24 December 2026 (Julian)" etc.
fn calendar_text(day: i64, month: &str, year: i64, calendar: &str) -> String {
    match lang() {
        Lang::De => format!("{day}. {month} {year} ({calendar})"),
        Lang::En => format!("{day} {month} {year} ({calendar})"),
    }
}

pub fn julian_text(date: Date) -> String {
    let (y, m, d) = julian_from_fixed(fixed(date));
    calendar_text(d, month_name(m), y, t!("Julian"))
}

pub fn hebrew_text(date: Date) -> String {
    let (y, m, d) = hebrew_from_fixed(fixed(date));
    calendar_text(d, hebrew_month_name(m, y), y, t!("Hebrew"))
}

pub fn islamic_text(date: Date) -> String {
    let (y, m, d) = islamic_from_fixed(fixed(date));
    let month = ISLAMIC_MONTHS.get((m - 1).clamp(0, 11) as usize).copied().unwrap_or("?");
    calendar_text(d, month, y, t!("Islamic, arithmetic"))
}

/// ISO week, e.g. "week 52 of 2026".
pub fn week_text(date: Date) -> String {
    let w = date.iso_week_date();
    t!("week {} of {}", w.week(), w.year())
}

// ===== time zones ===========================================================

const ABBREVIATIONS: &[(&str, &str)] = &[
    ("cet", "Europe/Berlin"),
    ("cest", "Europe/Berlin"),
    ("mez", "Europe/Berlin"),
    ("mesz", "Europe/Berlin"),
    ("wet", "Europe/Lisbon"),
    ("west", "Europe/Lisbon"),
    ("bst", "Europe/London"),
    ("eet", "Europe/Helsinki"),
    ("eest", "Europe/Helsinki"),
    ("msk", "Europe/Moscow"),
    ("est", "America/New_York"),
    ("edt", "America/New_York"),
    ("cst", "America/Chicago"),
    ("cdt", "America/Chicago"),
    ("mst", "America/Denver"),
    ("mdt", "America/Denver"),
    ("pst", "America/Los_Angeles"),
    ("pdt", "America/Los_Angeles"),
    ("akst", "America/Anchorage"),
    ("hst", "Pacific/Honolulu"),
    ("brt", "America/Sao_Paulo"),
    ("ist", "Asia/Kolkata"),
    ("jst", "Asia/Tokyo"),
    ("kst", "Asia/Seoul"),
    ("hkt", "Asia/Hong_Kong"),
    ("sgt", "Asia/Singapore"),
    ("aest", "Australia/Sydney"),
    ("aedt", "Australia/Sydney"),
    ("acst", "Australia/Adelaide"),
    ("awst", "Australia/Perth"),
    ("nzst", "Pacific/Auckland"),
    ("nzdt", "Pacific/Auckland"),
];

/// Cities that have no time zone of their own name.
const CITIES: &[(&str, &str)] = &[
    ("München", "Europe/Berlin"),
    ("Munich", "Europe/Berlin"),
    ("Hamburg", "Europe/Berlin"),
    ("Frankfurt", "Europe/Berlin"),
    ("Köln", "Europe/Berlin"),
    ("Cologne", "Europe/Berlin"),
    ("Düsseldorf", "Europe/Berlin"),
    ("Stuttgart", "Europe/Berlin"),
    ("Leipzig", "Europe/Berlin"),
    ("Dresden", "Europe/Berlin"),
    ("Hannover", "Europe/Berlin"),
    ("Nürnberg", "Europe/Berlin"),
    ("Wien", "Europe/Vienna"),
    ("Zürich", "Europe/Zurich"),
    ("Genf", "Europe/Zurich"),
    ("Geneva", "Europe/Zurich"),
    ("Rom", "Europe/Rome"),
    ("Mailand", "Europe/Rome"),
    ("Milan", "Europe/Rome"),
    ("Moskau", "Europe/Moscow"),
    ("Prag", "Europe/Prague"),
    ("Warschau", "Europe/Warsaw"),
    ("Kopenhagen", "Europe/Copenhagen"),
    ("Brüssel", "Europe/Brussels"),
    ("Lissabon", "Europe/Lisbon"),
    ("Athen", "Europe/Athens"),
    ("Barcelona", "Europe/Madrid"),
    ("San Francisco", "America/Los_Angeles"),
    ("Seattle", "America/Los_Angeles"),
    ("Las Vegas", "America/Los_Angeles"),
    ("San Diego", "America/Los_Angeles"),
    ("Washington", "America/New_York"),
    ("Boston", "America/New_York"),
    ("Miami", "America/New_York"),
    ("Atlanta", "America/New_York"),
    ("Dallas", "America/Chicago"),
    ("Houston", "America/Chicago"),
    ("Peking", "Asia/Shanghai"),
    ("Beijing", "Asia/Shanghai"),
    ("Mumbai", "Asia/Kolkata"),
    ("Delhi", "Asia/Kolkata"),
    ("New Delhi", "Asia/Kolkata"),
    ("Bangalore", "Asia/Kolkata"),
    ("Osaka", "Asia/Tokyo"),
    ("Kyoto", "Asia/Tokyo"),
    ("Rio de Janeiro", "America/Sao_Paulo"),
    ("Kapstadt", "Africa/Johannesburg"),
    ("Cape Town", "Africa/Johannesburg"),
];

fn normalize(name: &str) -> String {
    name.replace('_', " ").split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Display name → IANA name of every known city.
fn city_index() -> &'static HashMap<String, (String, String)> {
    static INDEX: OnceLock<HashMap<String, (String, String)>> = OnceLock::new();
    INDEX.get_or_init(|| {
        let mut map = HashMap::new();
        for (city, iana) in CITIES {
            map.insert(normalize(city), ((*city).to_owned(), (*iana).to_owned()));
        }
        for name in jiff::tz::db().available() {
            let iana = name.as_str();
            let Some((region, city)) = iana.rsplit_once('/') else { continue };
            if region.starts_with("Etc") || region.starts_with("posix") || region.starts_with("right") || region == "SystemV" {
                continue;
            }
            let display = city.replace('_', " ");
            map.entry(normalize(&display)).or_insert((display, iana.to_owned()));
        }
        map
    })
}

/// City names for completion, sorted.
pub fn city_names() -> Vec<&'static str> {
    let mut names: Vec<&str> = city_index().values().map(|(display, _)| display.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Resolves a time zone: a city ("Tokyo", "New York"), an IANA name
/// ("Europe/Berlin"), an abbreviation ("CET", "PST"), `UTC`, `UTC+2`,
/// `UTC-5:30` or `local`.
pub fn zone(name: &str) -> Option<TimeZone> {
    let key = normalize(name);
    if key.is_empty() {
        return None;
    }
    if matches!(key.as_str(), "local" | "lokal") {
        return Some(TimeZone::system());
    }
    if key == "z" {
        return Some(TimeZone::UTC);
    }
    for prefix in ["utc", "gmt"] {
        if let Some(rest) = key.strip_prefix(prefix) {
            return fixed_offset(rest.trim());
        }
    }
    if let Some((_, iana)) = ABBREVIATIONS.iter().find(|(a, _)| *a == key) {
        return TimeZone::get(iana).ok();
    }
    if name.contains('/') {
        if let Ok(tz) = TimeZone::get(name.trim()) {
            return Some(tz);
        }
    }
    let (_, iana) = city_index().get(&key)?;
    TimeZone::get(iana).ok()
}

/// `""`, `+2`, `-5:30`, `+0530`.
fn fixed_offset(rest: &str) -> Option<TimeZone> {
    if rest.is_empty() {
        return Some(TimeZone::UTC);
    }
    let sign = match rest.chars().next()? {
        '+' => 1,
        '-' | '−' => -1,
        _ => return None,
    };
    let digits = &rest[rest.chars().next()?.len_utf8()..];
    let (h, m) = match digits.split_once(':') {
        Some((h, m)) => (h.parse::<i32>().ok()?, m.parse::<i32>().ok()?),
        None if digits.len() == 4 => (digits[..2].parse().ok()?, digits[2..].parse().ok()?),
        None => (digits.parse().ok()?, 0),
    };
    if h > 14 || m > 59 {
        return None;
    }
    Offset::from_seconds(sign * (h * 3600 + m * 60)).ok().map(TimeZone::fixed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_days() {
        assert_eq!(fixed(date(1970, 1, 1).unwrap()), 719163);
        assert_eq!(fixed(date(1, 1, 1).unwrap()), 1);
        assert_eq!(from_fixed(710347).unwrap(), date(1945, 11, 12).unwrap());
        assert_eq!(julian_day_number(date(2000, 1, 1).unwrap()), 2451545);
    }

    #[test]
    fn calendars() {
        // Sample date of Calendrical Calculations: R.D. 710347 = 1945-11-12.
        assert_eq!(julian_from_fixed(710347), (1945, 10, 30));
        assert_eq!(hebrew_from_fixed(710347), (5706, 9, 7));
        assert_eq!(islamic_from_fixed(710347), (1364, 12, 6));
        assert_eq!(fixed_from_julian(1945, 10, 30), 710347);
        assert_eq!(fixed_from_hebrew(5706, 9, 7), 710347);
        assert_eq!(fixed_from_islamic(1364, 12, 6), 710347);
        // Rosh Hashanah 5785
        assert_eq!(hebrew_from_fixed(fixed(date(2024, 10, 3).unwrap())), (5785, 7, 1));
    }

    #[test]
    fn zones() {
        assert_eq!(zone("Tokyo").unwrap().iana_name(), Some("Asia/Tokyo"));
        assert_eq!(zone("new york").unwrap().iana_name(), Some("America/New_York"));
        assert_eq!(zone("Europe/Berlin").unwrap().iana_name(), Some("Europe/Berlin"));
        assert_eq!(zone("PST").unwrap().iana_name(), Some("America/Los_Angeles"));
        assert_eq!(zone("München").unwrap().iana_name(), Some("Europe/Berlin"));
        assert!(zone("UTC+5:30").is_some());
        assert!(zone("Nowhere").is_none());
    }
}

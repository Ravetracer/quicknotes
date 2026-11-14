//! Euro reference exchange rates of the European Central Bank for currency
//! units in calculation sheets.
//!
//! The rates are fetched only when a sheet uses another currency than the
//! euro, cached in the data directory and used offline from there. The ECB
//! publishes them once per working day.

use gtk::{gio, glib};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";
/// Cached rates older than this are fetched again.
const MAX_AGE: i64 = 12 * 3600;
/// Wait this long after a failed download before the next attempt.
const RETRY: i64 = 10 * 60;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rates {
    /// Day of the rates (YYYY-MM-DD).
    pub date: String,
    /// When they were downloaded (Unix seconds).
    pub fetched: i64,
    /// Units of the currency per euro.
    pub rates: BTreeMap<String, f64>,
}

#[derive(Default)]
struct State {
    rates: Option<Rates>,
    cache_read: bool,
    fetching: bool,
    last_failure: Option<i64>,
}

static STATE: Mutex<State> = Mutex::new(State { rates: None, cache_read: false, fetching: false, last_failure: None });

thread_local! {
    static REQUESTED: Cell<bool> = const { Cell::new(false) };
    static WAITING: RefCell<Vec<Box<dyn FnOnce()>>> = RefCell::new(Vec::new());
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn cache_path() -> PathBuf {
    crate::store::data_dir().join("exchange-rates.json")
}

fn with_state<T>(f: impl FnOnce(&mut State) -> T) -> T {
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if !state.cache_read {
        state.cache_read = true;
        if state.rates.is_none() && !cfg!(test) {
            state.rates = std::fs::read(cache_path()).ok().and_then(|b| serde_json::from_slice(&b).ok());
        }
    }
    f(&mut state)
}

/// Units of `code` per euro.
pub fn rate(code: &str) -> Result<f64, String> {
    if code == "EUR" {
        return Ok(1.0);
    }
    REQUESTED.with(|r| r.set(true));
    with_state(|s| match &s.rates {
        Some(r) => r.rates.get(code).copied().ok_or_else(|| t!("no exchange rate for {}", code)),
        None if s.fetching || s.last_failure.is_none() => Err(t!("loading exchange rates…").to_owned()),
        None => Err(t!("exchange rates unavailable (offline?)").to_owned()),
    })
}

/// Whether a currency rate was asked for since the last call.
pub fn take_requested() -> bool {
    REQUESTED.with(|r| r.replace(false))
}

/// Date of the rates in use.
pub fn date() -> Option<String> {
    with_state(|s| s.rates.as_ref().map(|r| r.date.clone()))
}

fn needs_refresh(s: &State) -> bool {
    let t = now();
    !s.fetching
        && s.rates.as_ref().is_none_or(|r| t - r.fetched > MAX_AGE)
        && s.last_failure.is_none_or(|f| t - f > RETRY)
}

/// Downloads the rates if they are missing or old and calls `done` (on the
/// main thread) once new rates are there. Does nothing if they are fresh.
pub fn refresh(done: impl FnOnce() + 'static) {
    let start = with_state(|s| {
        let start = needs_refresh(s);
        if start {
            s.fetching = true;
        }
        start
    });
    let fetching = with_state(|s| s.fetching);
    if !fetching {
        return;
    }
    WAITING.with(|w| w.borrow_mut().push(Box::new(done)));
    if !start {
        return;
    }
    glib::spawn_future_local(async move {
        let result = gio::spawn_blocking(fetch).await.unwrap_or_else(|_| Err("download failed".into()));
        with_state(|s| {
            s.fetching = false;
            match result {
                Ok(rates) => {
                    if let Ok(json) = serde_json::to_vec_pretty(&rates) {
                        let _ = std::fs::write(cache_path(), json);
                    }
                    s.rates = Some(rates);
                    s.last_failure = None;
                }
                Err(e) => {
                    eprintln!("exchange rates: {e}");
                    s.last_failure = Some(now());
                }
            }
        });
        // Also after a failure: the sheets then show "unavailable".
        let waiting = WAITING.with(|w| std::mem::take(&mut *w.borrow_mut()));
        for done in waiting {
            done();
        }
    });
}

fn fetch() -> Result<Rates, String> {
    let xml = ureq::get(URL)
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    let mut rates = parse(&xml)?;
    rates.fetched = now();
    Ok(rates)
}

/// Reads `<Cube time='…'>` and `<Cube currency='USD' rate='1.08'/>`.
pub fn parse(xml: &str) -> Result<Rates, String> {
    let attr = |tag: &str, name: &str| -> Option<String> {
        let start = tag.find(&format!("{name}="))? + name.len() + 1;
        let quote = tag[start..].chars().next()?;
        let rest = &tag[start + 1..];
        Some(rest[..rest.find(quote)?].to_owned())
    };
    let mut rates = Rates::default();
    for tag in xml.split('<').filter(|t| t.starts_with("Cube ")) {
        if let Some(date) = attr(tag, "time") {
            rates.date = date;
        }
        if let (Some(code), Some(rate)) = (attr(tag, "currency"), attr(tag, "rate")) {
            if let Ok(v) = rate.parse::<f64>() {
                if v > 0.0 {
                    rates.rates.insert(code, v);
                }
            }
        }
    }
    if rates.rates.is_empty() {
        return Err("no rates in the ECB response".into());
    }
    Ok(rates)
}

#[cfg(test)]
pub fn set_for_tests(rates: &[(&str, f64)]) {
    with_state(|s| {
        s.rates = Some(Rates {
            date: "2026-11-13".into(),
            fetched: now(),
            rates: rates.iter().map(|(c, r)| ((*c).to_owned(), *r)).collect(),
        })
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ecb_xml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<gesmes:Envelope xmlns:gesmes="http://www.gesmes.org/xml/2002-08-01" xmlns="http://www.ecb.int/vocabulary/2002-08-01/eurofxref">
	<Cube>
		<Cube time='2026-11-13'>
			<Cube currency='USD' rate='1.0812'/>
			<Cube currency='JPY' rate='165.20'/>
		</Cube>
	</Cube>
</gesmes:Envelope>"#;
        let r = parse(xml).unwrap();
        assert_eq!(r.date, "2026-11-13");
        assert_eq!(r.rates.get("USD"), Some(&1.0812));
        assert_eq!(r.rates.len(), 2);
        assert!(parse("<html/>").is_err());
    }
}

//! `Date` static methods: `now`, `parse`, and `UTC`.
//!
//! Time math uses the civil-calendar algorithm from Howard Hinnant
//! (`days_from_civil`), so no external date crate is required. All values are
//! milliseconds since the Unix epoch (1970-01-01T00:00:00Z); parsed strings are
//! interpreted as UTC (the sandbox has no local-timezone concept).

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::VmErr;
use crate::interpreter::{Environment, Interpreter};
use crate::value::Value;

pub(super) fn install(e: &mut Environment) {
    if let Some(d) = e.get("Date") {
        d.set_prop("now".to_string(), super::nf("now", date_now))
            .expect("built-in Date property");
        d.set_prop("parse".to_string(), super::nf("parse", date_parse))
            .expect("built-in Date property");
        d.set_prop("UTC".to_string(), super::nf("UTC", date_utc))
            .expect("built-in Date property");
    }
}

fn date_now(_: &mut Interpreter, _: Value, _: Vec<Value>) -> Result<Value, VmErr> {
    Ok(Value::Number(now_ms()))
}

/// Milliseconds since the Unix epoch. On native targets this reads the system
/// clock; on `wasm32` (where `SystemTime` panics) it asks the JS host.
#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn date_utc(_: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let num = |i: usize, dflt: f64| a.get(i).map(|v| v.to_number()).unwrap_or(dflt);
    let mut year = num(0, 1970.0) as i64;
    // Two-digit years map into the 1900s, matching JavaScript.
    if (0..=99).contains(&year) {
        year += 1900;
    }
    let month = num(1, 0.0) as i64 + 1; // month index is zero-based
    let day = num(2, 1.0) as i64;
    let hour = num(3, 0.0) as i64;
    let minute = num(4, 0.0) as i64;
    let second = num(5, 0.0) as i64;
    let millis = num(6, 0.0) as i64;
    Ok(Value::Number(utc_ms(
        year, month, day, hour, minute, second, millis,
    )))
}

fn date_parse(interp: &mut Interpreter, _: Value, a: Vec<Value>) -> Result<Value, VmErr> {
    let s = match a.first() {
        Some(Value::String(s)) => s.clone(),
        Some(v) => interp.vs(v),
        None => return Ok(Value::Number(f64::NAN)),
    };
    Ok(Value::Number(parse_iso(&s).unwrap_or(f64::NAN)))
}

/// Milliseconds since the epoch for a UTC civil date/time.
fn utc_ms(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64, ms: i64) -> f64 {
    let days = days_from_civil(year, month, day);
    let secs = days * 86_400 + hour * 3_600 + minute * 60 + second;
    (secs * 1_000 + ms) as f64
}

/// Days since 1970-01-01 for a civil date (Hinnant's algorithm). Negative for
/// dates before the epoch.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - i64::from(m <= 2);
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400; // [0, 399]
    let mshift = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mshift + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

/// Parse an ISO-8601 string (`YYYY-MM-DD[THH:mm[:ss[.sss]]][Z|±HH:mm]`) into
/// epoch milliseconds. Returns `None` when the string is not recognizable.
fn parse_iso(s: &str) -> Option<f64> {
    let s = s.trim();
    let (date_part, time_part) = if let Some(idx) = s.find('T') {
        (&s[..idx], Some(&s[idx + 1..]))
    } else if let Some(idx) = s.find(' ') {
        (&s[..idx], Some(&s[idx + 1..]))
    } else {
        (s, None)
    };

    let dseps: Vec<&str> = date_part.split(['-', '/']).collect();
    let year: i64 = dseps.first()?.parse().ok()?;
    let month: i64 = dseps.get(1).and_then(|x| x.parse().ok()).unwrap_or(1);
    let day: i64 = dseps.get(2).and_then(|x| x.parse().ok()).unwrap_or(1);

    let mut hour = 0i64;
    let mut minute = 0i64;
    let mut second = 0i64;
    let mut millis = 0i64;
    let mut tz_offset_secs = 0i64;

    if let Some(mut t) = time_part {
        // Peel off a trailing timezone designator first.
        if let Some(zidx) = t.find('Z') {
            t = &t[..zidx];
        } else if let Some(sign_idx) = t.rfind(['+', '-']) {
            let sign = &t[sign_idx..sign_idx + 1];
            let tz_parts: Vec<&str> = t[sign_idx + 1..].split(':').collect();
            let tzh: i64 = tz_parts.first().and_then(|x| x.parse().ok()).unwrap_or(0);
            let tzm: i64 = tz_parts.get(1).and_then(|x| x.parse().ok()).unwrap_or(0);
            let mag = tzh * 3_600 + tzm * 60;
            tz_offset_secs = if sign == "-" { -mag } else { mag };
            t = &t[..sign_idx];
        }

        let (hms, frac) = match t.find('.') {
            Some(dot) => (&t[..dot], Some(&t[dot + 1..])),
            None => (t, None),
        };
        let parts: Vec<&str> = hms.split(':').collect();
        hour = parts.first().and_then(|x| x.parse().ok()).unwrap_or(0);
        minute = parts.get(1).and_then(|x| x.parse().ok()).unwrap_or(0);
        second = parts.get(2).and_then(|x| x.parse().ok()).unwrap_or(0);
        if let Some(fr) = frac {
            let digits: String = fr.chars().take(3).collect();
            millis = format!("{:0<3}", digits).parse().unwrap_or(0);
        }
    }

    let base = utc_ms(year, month, day, hour, minute, second, millis);
    Some(base - f64::from(tz_offset_secs as i32) * 1_000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_zero() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn known_dates() {
        // 2000-01-01 is 10957 days after the epoch.
        assert_eq!(days_from_civil(2000, 1, 1), 10_957);
        // 1969-12-31 is one day before.
        assert_eq!(days_from_civil(1969, 12, 31), -1);
    }

    #[test]
    fn parse_date_only() {
        assert_eq!(parse_iso("1970-01-01"), Some(0.0));
        assert_eq!(parse_iso("2000-01-01"), Some(946_684_800_000.0));
    }

    #[test]
    fn parse_with_time_and_z() {
        assert_eq!(parse_iso("1970-01-01T00:00:01Z"), Some(1_000.0));
        assert_eq!(parse_iso("1970-01-01T01:00:00Z"), Some(3_600_000.0));
    }

    #[test]
    fn parse_with_offset() {
        // 01:00 at +01:00 is midnight UTC.
        assert_eq!(parse_iso("1970-01-01T01:00:00+01:00"), Some(0.0));
    }

    #[test]
    fn parse_millis() {
        assert_eq!(parse_iso("1970-01-01T00:00:00.500Z"), Some(500.0));
    }
}

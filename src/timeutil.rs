//! Unix milliseconds and RFC3339 formatting without a time crate.

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(i64);

impl Timestamp {
    pub fn from_millis(ms: i64) -> Self {
        Self(ms)
    }

    pub fn as_millis(self) -> i64 {
        self.0
    }

    pub fn saturating_add_ms(self, ms: i64) -> Self {
        Self(self.0.saturating_add(ms))
    }
}

pub trait WallClock: Send + Sync {
    fn now_ms(&self) -> i64;
}

#[derive(Clone, Copy, Debug)]
pub struct SystemClock;

impl WallClock for SystemClock {
    fn now_ms(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }
}

#[derive(Debug)]
pub struct ManualClock {
    ms: std::sync::atomic::AtomicI64,
}

impl ManualClock {
    pub fn new(ms: i64) -> Self {
        Self {
            ms: std::sync::atomic::AtomicI64::new(ms),
        }
    }

    pub fn set(&self, ms: i64) {
        self.ms.store(ms, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn advance(&self, ms: i64) {
        self.ms.fetch_add(ms, std::sync::atomic::Ordering::SeqCst);
    }
}

impl WallClock for ManualClock {
    fn now_ms(&self) -> i64 {
        self.ms.load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub fn format_rfc3339(ms: i64) -> String {
    let negative = ms < 0;
    let abs = ms.unsigned_abs();
    let secs = abs / 1000;
    let millis = abs % 1000;
    let days = (secs / 86_400) as i64;
    let tod = secs % 86_400;
    let (year, month, day) = civil_from_days(if negative { -days } else { days });
    let hour = tod / 3600;
    let min = (tod % 3600) / 60;
    let sec = tod % 60;
    if negative {
        format!("-{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{millis:03}Z")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}.{millis:03}Z")
    }
}

/// Howard Hinnant's `civil_from_days`, shifted so day 0 is 1970-01-01.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

pub fn parse_rfc3339(value: &str) -> Option<i64> {
    if value.len() < 24 || !value.ends_with('Z') {
        return None;
    }
    let year: i32 = value.get(0..4)?.parse().ok()?;
    let month: u32 = value.get(5..7)?.parse().ok()?;
    let day: u32 = value.get(8..10)?.parse().ok()?;
    let hour: i64 = value.get(11..13)?.parse().ok()?;
    let min: i64 = value.get(14..16)?.parse().ok()?;
    let sec: i64 = value.get(17..19)?.parse().ok()?;
    let millis: i64 = value.get(20..23)?.parse().ok()?;
    let days = days_from_civil(year, month, day)?;
    Some(days * 86_400_000 + hour * 3_600_000 + min * 60_000 + sec * 1_000 + millis)
}

fn days_from_civil(year: i32, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || day == 0 || day > 31 {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp as u64 + 2) / 5 + day as u64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe as i64 - 719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_and_known_instant() {
        assert_eq!(format_rfc3339(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(
            format_rfc3339(1_700_000_000_000),
            "2023-11-14T22:13:20.000Z"
        );
        assert_eq!(
            parse_rfc3339("2023-11-14T22:13:20.000Z"),
            Some(1_700_000_000_000)
        );
        assert_eq!(parse_rfc3339(&format_rfc3339(0)), Some(0));
    }
}

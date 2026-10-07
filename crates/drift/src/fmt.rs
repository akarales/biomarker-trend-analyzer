//! Formatting for explanation text: ISO dates from epoch seconds (pure
//! calendar arithmetic, no clock) and values at a readable precision.

pub const DAY_SECONDS: i64 = 86_400;

/// `YYYY-MM-DD` (UTC) for epoch seconds — Howard Hinnant's civil-from-days.
pub fn date(epoch_seconds: i64) -> String {
    let z = epoch_seconds.div_euclid(DAY_SECONDS) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// A lab value at a precision that matches its magnitude.
pub fn value(v: f64) -> String {
    let a = v.abs();
    if a >= 100.0 {
        format!("{v:.0}")
    } else if a >= 10.0 {
        format!("{v:.1}")
    } else {
        format!("{v:.2}")
    }
}

/// A UCUM unit code as clinicians read it in prose (`m[IU]/L` → `mIU/L`,
/// `umol/L` → `µmol/L`); report fields keep the UCUM code.
pub fn unit(ucum: &str) -> String {
    ucum.replace("m[IU]/L", "mIU/L").replace("umol/L", "µmol/L")
}

/// A fraction as a signed percentage, e.g. `+12.3 %`.
pub fn pct(fraction: f64) -> String {
    format!("{:+.1} %", fraction * 100.0)
}

/// A CV fraction as an unsigned percentage, e.g. `4.4 %`.
pub fn cv(fraction: f64) -> String {
    format!("{:.1} %", fraction * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(1_767_657_600), "2026-01-06");
        assert_eq!(date(951_782_400), "2000-02-29");
        assert_eq!(date(-DAY_SECONDS), "1969-12-31");
    }

    #[test]
    fn values() {
        assert_eq!(value(127.14), "127");
        assert_eq!(value(12.345), "12.3");
        assert_eq!(value(5.594), "5.59");
        assert_eq!(pct(0.1234), "+12.3 %");
        assert_eq!(pct(-0.05), "-5.0 %");
        assert_eq!(unit("m[IU]/L"), "mIU/L");
        assert_eq!(unit("umol/L"), "µmol/L");
        assert_eq!(unit("mg/dL"), "mg/dL");
    }
}

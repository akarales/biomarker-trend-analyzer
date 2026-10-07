//! Small, dependency-free statistics used by the detectors.

/// z for a two-sided 95 % interval.
pub const Z95: f64 = 1.959_963_985;
/// z for a two-sided 99 % interval.
pub const Z99: f64 = 2.575_829_304;

pub fn median(values: &[f64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(sorted[n / 2]),
        _ => Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2.0),
    }
}

/// Complementary error function (Numerical Recipes `erfcc`, fractional
/// error < 1.2e-7 everywhere) — plenty for p-values.
pub fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let poly = -z * z - 1.265_512_23
        + t * (1.000_023_68
            + t * (0.374_091_96
                + t * (0.096_784_18
                    + t * (-0.186_288_06
                        + t * (0.278_868_07
                            + t * (-1.135_203_98
                                + t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
    let r = t * poly.exp();
    if x >= 0.0 { r } else { 2.0 - r }
}

/// Standard normal CDF.
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * erfc(-x / std::f64::consts::SQRT_2)
}

/// Mann–Kendall trend test (with the tie correction).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MannKendall {
    pub s: i64,
    pub var_s: f64,
    /// Kendall's τ (S over the number of pairs), −1 … 1.
    pub tau: f64,
    /// two-sided p-value (normal approximation with continuity correction)
    pub p_value: f64,
}

pub fn mann_kendall(values: &[f64]) -> Option<MannKendall> {
    let n = values.len();
    if n < 3 {
        return None;
    }
    let mut s = 0i64;
    for i in 0..n {
        for j in i + 1..n {
            s += match values[j].total_cmp(&values[i]) {
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Equal => 0,
            };
        }
    }
    let nf = n as f64;
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mut ties = 0.0;
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j + 1 < n && sorted[j + 1] == sorted[i] {
            j += 1;
        }
        let t = (j - i + 1) as f64;
        ties += t * (t - 1.0) * (2.0 * t + 5.0);
        i = j + 1;
    }
    let var_s = (nf * (nf - 1.0) * (2.0 * nf + 5.0) - ties) / 18.0;
    let z = match s {
        0 => 0.0,
        _ if var_s <= 0.0 => 0.0,
        _ if s > 0 => (s - 1) as f64 / var_s.sqrt(),
        _ => (s + 1) as f64 / var_s.sqrt(),
    };
    Some(MannKendall {
        s,
        var_s,
        tau: s as f64 / (nf * (nf - 1.0) / 2.0),
        p_value: (2.0 * (1.0 - normal_cdf(z.abs()))).clamp(0.0, 1.0),
    })
}

/// Sen's slope (median of pairwise slopes, per unit of `x`) with the
/// distribution-free 95 % confidence interval (Gilbert 1987, §16.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SenSlope {
    pub slope: f64,
    pub ci_low: f64,
    pub ci_high: f64,
}

pub fn sen_slope(x: &[f64], y: &[f64], var_s: f64) -> Option<SenSlope> {
    let mut slopes = Vec::new();
    for i in 0..x.len() {
        for j in i + 1..x.len() {
            let dx = x[j] - x[i];
            if dx.abs() > f64::EPSILON {
                slopes.push((y[j] - y[i]) / dx);
            }
        }
    }
    if slopes.is_empty() {
        return None;
    }
    slopes.sort_by(f64::total_cmp);
    let slope = median(&slopes)?;
    let n = slopes.len() as f64;
    let c = Z95 * var_s.max(0.0).sqrt();
    let rank = |r: f64| slopes[(r.round() as usize).clamp(1, slopes.len()) - 1];
    Some(SenSlope {
        slope,
        ci_low: rank((n - c) / 2.0),
        ci_high: rank((n + c) / 2.0 + 1.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_cdf_matches_known_quantiles() {
        assert!((normal_cdf(0.0) - 0.5).abs() < 1e-7);
        assert!((normal_cdf(Z95) - 0.975).abs() < 1e-6);
        assert!((normal_cdf(-Z99) - 0.005).abs() < 1e-6);
    }

    #[test]
    fn medians() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), Some(2.5));
    }

    #[test]
    fn mann_kendall_on_a_monotone_series() {
        // n = 10 strictly increasing: S = 45, Var = 125, z = 44/√125 = 3.94
        let mk = mann_kendall(&(0..10).map(f64::from).collect::<Vec<_>>()).expect("mk");
        assert_eq!(mk.s, 45);
        assert!((mk.var_s - 125.0).abs() < 1e-9);
        assert!((mk.tau - 1.0).abs() < 1e-12);
        assert!(mk.p_value < 1e-4);
    }

    #[test]
    fn mann_kendall_ties_and_flat() {
        let mk = mann_kendall(&[1.0, 1.0, 1.0, 1.0]).expect("mk");
        assert_eq!(mk.s, 0);
        assert!((mk.p_value - 1.0).abs() < 1e-6);
    }

    #[test]
    fn sen_slope_ignores_a_spike_and_brackets_the_truth() {
        let x: Vec<f64> = (0..20).map(f64::from).collect();
        let mut y: Vec<f64> = x.iter().map(|v| 2.0 + 0.5 * v).collect();
        y[10] = 100.0;
        let mk = mann_kendall(&y).expect("mk");
        let sen = sen_slope(&x, &y, mk.var_s).expect("sen");
        assert!((sen.slope - 0.5).abs() < 0.05);
        assert!(sen.ci_low <= 0.5 && 0.5 <= sen.ci_high);
    }
}

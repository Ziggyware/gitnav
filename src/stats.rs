//! Statistics for commit streams, modelled as point processes in time.
use std::f64::consts::TAU;

/// Recent-vs-baseline rate comparison.
pub struct Trend {
    pub ratio: f64,
    pub p: f64,
}

/// Exact two-sided test of equal Poisson rates. Conditional on the total n, recent ~ Bin(n, t_r / (t_r + t_b)).
pub fn trend(recent: u32, base: u32, t_recent: f64, t_base: f64) -> Option<Trend> {
    let n = recent + base;
    if n < 5 {
        return None;
    }
    let p0 = t_recent / (t_recent + t_base);
    let lnf: Vec<f64> = (0..=n)
        .scan(0.0, |s, i| {
            *s += (i.max(1) as f64).ln();
            Some(*s)
        })
        .collect();
    let pmf = |k: u32| {
        (lnf[n as usize] - lnf[k as usize] - lnf[(n - k) as usize]
            + k as f64 * p0.ln()
            + (n - k) as f64 * (1.0 - p0).ln())
        .exp()
    };
    let obs = pmf(recent);
    let p = (0..=n)
        .map(pmf)
        .filter(|&x| x <= obs * (1.0 + 1e-9))
        .sum::<f64>()
        .min(1.0);
    let ratio = (recent as f64 / t_recent) / (base as f64 / t_base);
    Some(Trend { ratio, p })
}

/// Exponentially weighted commits/week; half-life in weeks.
pub fn rate(weeks: &[u32], half_life: f64) -> f64 {
    let w = |age: usize| 0.5f64.powf(age as f64 / half_life);
    let (num, den) = weeks
        .iter()
        .rev()
        .enumerate()
        .fold((0.0, 0.0), |(n, d), (age, &c)| (n + w(age) * c as f64, d + w(age)));
    num / den
}

/// P(no commit for `gap` days | Poisson rate n / window): e^(−λ·gap).
pub fn quiet(n: u32, window_days: f64, gap_days: f64) -> f64 {
    (-(n as f64 / window_days) * gap_days).exp()
}

/// B = (σ − μ) / (σ + μ) of inter-event gaps: −1 periodic, 0 Poisson, → 1 bursty.
pub fn burstiness(ts: impl Iterator<Item = i64>) -> Option<f32> {
    let mut t: Vec<i64> = ts.collect();
    t.sort_unstable();
    let g: Vec<f64> = t.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
    if g.len() < 10 {
        return None;
    }
    let mu = g.iter().sum::<f64>() / g.len() as f64;
    let sd = (g.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / g.len() as f64).sqrt();
    (sd + mu > 0.0).then(|| ((sd - mu) / (sd + mu)) as f32)
}

/// Circular mean hour and resultant length R ∈ [0, 1] from Σcos, Σsin over n events.
pub fn circular(c: f32, s: f32, n: u32) -> Option<(f32, f32)> {
    if n < 8 {
        return None;
    }
    let hour = (s.atan2(c) as f64).rem_euclid(TAU) / TAU * 24.0;
    Some((hour as f32, (c * c + s * s).sqrt() / n as f32))
}

/// Smallest k such that the k largest shares cover `share` of the total.
pub fn focus(mut counts: Vec<u32>, share: f64) -> usize {
    counts.sort_unstable_by(|a, b| b.cmp(a));
    let total = counts.iter().sum::<u32>() as f64;
    let mut acc = 0.0;
    counts
        .iter()
        .take_while(|&&c| {
            let before = acc;
            acc += c as f64;
            before < share * total
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trend_detects_only_real_shifts() {
        let t = trend(14, 3, 4.0, 12.0).unwrap();
        assert!(t.p < 1e-4 && (t.ratio - 14.0).abs() < 1e-9);
        assert!(trend(3, 9, 4.0, 12.0).unwrap().p > 0.99);
        // Bin(20, 0.25): P(X ≤ 1) + P(X ≥ 9) = 0.0243 + 0.0409.
        assert!((trend(9, 11, 4.0, 12.0).unwrap().p - 0.0652).abs() < 1e-3);
        assert!(trend(1, 2, 4.0, 12.0).is_none());
    }

    #[test]
    fn process_shape_metrics() {
        assert_eq!(burstiness((0..12).map(|i| i * 3600)), Some(-1.0));
        // One huge gap among 10 hits the finite-n ceiling (√(n−1) − 1) / (√(n−1) + 1) = 0.5.
        let b = burstiness([0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 100_000].into_iter()).unwrap();
        assert!((b - 0.5).abs() < 1e-3);
        // 23:00 and 01:00 average to midnight, not noon.
        let a = |h: f64| h / 24.0 * TAU;
        let (c, s) = [23.0, 1.0]
            .iter()
            .cycle()
            .take(8)
            .fold((0.0, 0.0), |(c, s), &h| (c + a(h).cos(), s + a(h).sin()));
        let (hour, r) = circular(c as f32, s as f32, 8).unwrap();
        assert!((hour.min(24.0 - hour)) < 1e-3 && (r - 0.9659).abs() < 1e-3);
        assert!((quiet(16, 112.0, 7.0) - (-1.0f64).exp()).abs() < 1e-12);
        assert!((rate(&[4, 4, 4], 4.0) - 4.0).abs() < 1e-12);
        assert_eq!(focus(vec![1, 80, 5, 14], 0.8), 1);
        assert_eq!(focus(vec![25, 25, 25, 25], 0.8), 4);
    }
}

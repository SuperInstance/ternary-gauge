#![forbid(unsafe_code)]

#[derive(Clone, Debug, PartialEq)]
pub struct Gauge {
    pub samples: Vec<i8>,
    pub max_history: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum GaugeHealth {
    Healthy,
    Drifting,
    Stuck,
    Oscillating,
}

pub fn new(capacity: usize) -> Gauge {
    Gauge { samples: Vec::with_capacity(capacity), max_history: capacity }
}

pub fn sample(g: &mut Gauge, value: i8) {
    if g.samples.len() >= g.max_history {
        g.samples.remove(0);
    }
    g.samples.push(value);
}

pub fn mean(g: &Gauge) -> f64 {
    if g.samples.is_empty() { return 0.0; }
    let sum: f64 = g.samples.iter().map(|&v| v as f64).sum();
    sum / g.samples.len() as f64
}

pub fn variance(g: &Gauge) -> f64 {
    if g.samples.is_empty() { return 0.0; }
    let m = mean(g);
    let sum: f64 = g.samples.iter().map(|&v| (v as f64 - m).powi(2)).sum();
    sum / g.samples.len() as f64
}

pub fn histogram(g: &Gauge) -> [usize; 3] {
    let mut h = [0usize; 3];
    for &v in &g.samples {
        match v {
            -1 => h[0] += 1,
            0 => h[1] += 1,
            1 => h[2] += 1,
            _ => {}
        }
    }
    h
}

pub fn mode(g: &Gauge) -> i8 {
    let h = histogram(g);
    if h[0] >= h[1] && h[0] >= h[2] { -1 }
    else if h[2] >= h[0] && h[2] >= h[1] { 1 }
    else { 0 }
}

pub fn is_stationary(g: &Gauge, window: usize) -> bool {
    if g.samples.len() < window { return true; }
    let recent: Vec<i8> = g.samples.iter().rev().take(window).copied().collect();
    let sum: f64 = recent.iter().map(|&v| v as f64).sum();
    let m = sum / recent.len() as f64;
    let var: f64 = recent.iter().map(|&v| (v as f64 - m).powi(2)).sum::<f64>() / recent.len() as f64;
    var < 0.1
}

pub fn drift_rate(g: &Gauge) -> f64 {
    if g.samples.len() < 2 { return 0.0; }
    let n = g.samples.len();
    let first_half_mean: f64 = g.samples[..n/2].iter().map(|&v| v as f64).sum::<f64>() / (n/2) as f64;
    let second_half_mean: f64 = g.samples[n/2..].iter().map(|&v| v as f64).sum::<f64>() / (n - n/2) as f64;
    second_half_mean - first_half_mean
}

pub fn health(g: &Gauge) -> GaugeHealth {
    if g.samples.len() < 4 { return GaugeHealth::Healthy; }
    // Stuck: all same
    if g.samples.iter().all(|&v| v == g.samples[0]) {
        return GaugeHealth::Stuck;
    }
    // Oscillating: alternating pattern
    let mut alternations = 0;
    for w in g.samples.windows(2) {
        if w[0] != w[1] { alternations += 1; }
    }
    let alt_ratio = alternations as f64 / (g.samples.len() - 1) as f64;
    if alt_ratio > 0.8 {
        return GaugeHealth::Oscillating;
    }
    // Drifting vs Healthy based on drift_rate magnitude
    let dr = drift_rate(g);
    if dr.abs() > 0.3 {
        return GaugeHealth::Drifting;
    }
    GaugeHealth::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_gauge() {
        let g = new(10);
        assert!(g.samples.is_empty());
        assert_eq!(g.max_history, 10);
    }

    #[test]
    fn test_sample_and_len() {
        let mut g = new(5);
        sample(&mut g, 1);
        sample(&mut g, 0);
        assert_eq!(g.samples.len(), 2);
    }

    #[test]
    fn test_max_history_eviction() {
        let mut g = new(3);
        sample(&mut g, 1);
        sample(&mut g, 0);
        sample(&mut g, -1);
        sample(&mut g, 1);
        assert_eq!(g.samples.len(), 3);
        assert_eq!(g.samples, vec![0, -1, 1]);
    }

    #[test]
    fn test_mean() {
        let mut g = new(10);
        sample(&mut g, -1);
        sample(&mut g, 1);
        assert!((mean(&g) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_variance_zero() {
        let mut g = new(10);
        sample(&mut g, 1);
        sample(&mut g, 1);
        assert!((variance(&g) - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_variance_nonzero() {
        let mut g = new(10);
        sample(&mut g, -1);
        sample(&mut g, 1);
        assert!(variance(&g) > 0.0);
    }

    #[test]
    fn test_histogram() {
        let mut g = new(10);
        sample(&mut g, -1);
        sample(&mut g, 0);
        sample(&mut g, 1);
        sample(&mut g, 1);
        assert_eq!(histogram(&g), [1, 1, 2]);
    }

    #[test]
    fn test_mode() {
        let mut g = new(10);
        sample(&mut g, 1);
        sample(&mut g, 1);
        sample(&mut g, 0);
        assert_eq!(mode(&g), 1);
    }

    #[test]
    fn test_is_stationary_flat() {
        let mut g = new(10);
        for _ in 0..5 { sample(&mut g, 0); }
        assert!(is_stationary(&g, 4));
    }

    #[test]
    fn test_is_stationary_changing() {
        let mut g = new(10);
        sample(&mut g, -1);
        sample(&mut g, 1);
        sample(&mut g, -1);
        sample(&mut g, 1);
        assert!(!is_stationary(&g, 4));
    }

    #[test]
    fn test_drift_rate_rising() {
        let mut g = new(10);
        sample(&mut g, -1);
        sample(&mut g, -1);
        sample(&mut g, 1);
        sample(&mut g, 1);
        assert!(drift_rate(&g) > 0.0);
    }

    #[test]
    fn test_health_stuck() {
        let mut g = new(10);
        for _ in 0..5 { sample(&mut g, 1); }
        assert_eq!(health(&g), GaugeHealth::Stuck);
    }

    #[test]
    fn test_health_oscillating() {
        let mut g = new(10);
        for i in 0..8 { sample(&mut g, if i % 2 == 0 { 1 } else { -1 }); }
        assert_eq!(health(&g), GaugeHealth::Oscillating);
    }

    #[test]
    fn test_health_healthy() {
        let mut g = new(10);
        for _ in 0..8 { sample(&mut g, 0); }
        assert_eq!(health(&g), GaugeHealth::Stuck); // all same = stuck, which is a form of stable

        let mut g2 = new(10);
        sample(&mut g2, 0);
        sample(&mut g2, 1);
        sample(&mut g2, 0);
        sample(&mut g2, 1);
        sample(&mut g2, 0);
        // alternations: 4/4 = 1.0 > 0.8 so oscillating. Need something else.

        // Truly healthy: mostly constant with slight noise
        let mut g3 = new(10);
        sample(&mut g3, 0);
        sample(&mut g3, 0);
        sample(&mut g3, 1);
        sample(&mut g3, 0);
        sample(&mut g3, 0);
        sample(&mut g3, 0);
        sample(&mut g3, 0);
        sample(&mut g3, 0);
        // drift: first 4 mean=0.25, second 4 mean=0.0, drift=-0.25 < 0.3
        assert_eq!(health(&g3), GaugeHealth::Healthy);
    }

    #[test]
    fn test_mean_empty() {
        let g = new(10);
        assert_eq!(mean(&g), 0.0);
    }
}

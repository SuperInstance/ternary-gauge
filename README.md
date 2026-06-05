# ternary-gauge

**Instrumentation for ternary signals. Mean, variance, health, drift detection.**

You can't improve what you don't measure. A ternary gauge tracks a sliding window of `{-1, 0, +1}` values and computes statistics in real-time: mean, variance, histogram, mode, and — critically — *health*. Is the signal healthy and varied? Drifting toward one value? Stuck in a rut? Oscillating? The gauge tells you.

## What's Inside

- **`Gauge`** — sliding-window sampler with configurable capacity
- **`sample(gauge, value)`** — add a reading
- **`mean(gauge)`** / **`variance(gauge)`** — basic statistics
- **`histogram(gauge)`** — count of each value `[neg, zero, pos]`
- **`mode(gauge)`** — most frequent recent value
- **`health(gauge)`** — diagnose: `Healthy`, `Drifting`, `Stuck`, `Oscillating`
- **`trend(gauge)`** — is the mean rising, falling, or flat?

## Quick Example

```rust
use ternary_gauge::*;

let mut g = new(100); // 100-sample window

// Feed it readings
for v in [1, 1, 0, -1, 1, 0, 0, 1, -1, 0] {
    sample(&mut g, v);
}

println!("Mean: {:.2}", mean(&g));     // ~0.2
println!("Variance: {:.2}", variance(&g));
println!("Mode: {}", mode(&g));       // 0 or 1
println!("Health: {:?}", health(&g)); // Healthy

// A stuck signal
let mut stuck = new(50);
for _ in 0..50 { sample(&mut stuck, 0); }
assert_eq!(health(&stuck), GaugeHealth::Stuck);
```

## The Insight

**Gauges detect problems early.** In a multi-agent system, each agent's state stream is a ternary signal. If the gauge says "Stuck," the agent is trapped at 0 (the spindle). If "Oscillating," it's cycling between +1 and -1 without finding equilibrium. Health detection is the first step toward self-repair.

**Use cases:**
- **System monitoring** — gauge the health of any ternary-valued process
- **Agent diagnostics** — is an agent behaving normally or degrading?
- **Quality control** — gauge production line outputs
- **Anomaly detection** — drift detection as early warning
- **Trading signals** — gauge market sentiment streams

## See Also

- **ternary-metrics** — broader metrics collection
- **ternary-entropy** — information-theoretic health measures
- **ternary-vu** — audio-level metering (a specialized gauge)

## Install

```bash
cargo add ternary-gauge
```

## License

MIT

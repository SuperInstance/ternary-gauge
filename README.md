# ternary-gauge

Instrumentation for ternary signals. Sliding-window statistics, health detection, drift alerts.

You can't fix what you can't see. A ternary gauge sits on any stream of {-1, 0, +1} values and tells you what's happening *right now*: the mean, the variance, the mode, and—most importantly—the **health**. Is the signal healthy and varied? Stuck at one value? Drifting toward an extreme? Oscillating without settling?

Think of it as a dashboard instrument for ternary processes. You wouldn't drive a car without a fuel gauge. Don't run a multi-agent system without gauging its state streams.

## Why this exists

Multi-agent systems produce streams of ternary decisions: accept/reject/abstain, cooperate/defect/wait, healthy/degraded/failed. When something goes wrong—an agent gets stuck voting 0, or starts oscillating between +1 and -1—you need to detect it early, before it cascades.

Statistics like mean and variance tell you *what's happening*. The health classifier tells you *what to worry about*.

## The key insight

Four health states cover the failure modes of any ternary stream:

| Health | Pattern | Meaning |
|--------|---------|---------|
| `Healthy` | Varied, moderate drift | System operating normally |
| `Stuck` | All values identical | Agent trapped, sensor broken, loop detected |
| `Oscillating` | Rapid alternation (>80% changes) | Instability, contention, thrashing |
| `Drifting` | Mean shifting by >0.3 between halves | Systematic bias growing or shrinking |

The detection thresholds are deliberately conservative. A stuck signal is trivially detectable (all same). Oscillation needs >80% alternation. Drift needs the mean to shift by more than 0.3 between the first and second half of the window. These thresholds catch real problems without false positives from normal noise.

## Quick start

```rust
use ternary_gauge::*;

let mut g = new(100);  // 100-sample sliding window

// Feed it readings
for v in [1, 1, 0, -1, 1, 0, 0, 1, -1, 0] {
    sample(&mut g, v);
}

println!("Mean:      {:.2}", mean(&g));       // ~0.10
println!("Variance:  {:.2}", variance(&g));   // ~0.56
println!("Histogram: {:?}", histogram(&g));   // [2, 4, 4] → neg/zero/pos counts
println!("Mode:      {}", mode(&g));          // 0 (or 1, tied)
println!("Health:    {:?}", health(&g));      // Healthy
```

## Detecting problems

```rust
// A stuck signal — agent trapped at 0
let mut stuck = new(50);
for _ in 0..50 { sample(&mut stuck, 0); }
assert_eq!(health(&stuck), GaugeHealth::Stuck);

// An oscillating signal — agent thrashing between accept/reject
let mut oscillating = new(20);
for i in 0..20 { sample(&mut oscillating, if i % 2 == 0 { 1 } else { -1 }); }
assert_eq!(health(&oscillating), GaugeHealth::Oscillating);

// A drifting signal — bias growing over time
let mut drifting = new(20);
for i in 0..20 { sample(&mut drifting, if i < 10 { -1 } else { 1 }); }
assert_eq!(health(&drifting), GaugeHealth::Drifting);
```

## API reference

### Construction and sampling

```rust
let mut g = new(capacity);        // create gauge with sliding window of `capacity` samples
sample(&mut g, value);            // add a reading (evicts oldest when full)
```

### Statistics

```rust
mean(&g)                          // → f64, average of all samples in window
variance(&g)                      // → f64, population variance
histogram(&g)                     // → [usize; 3], count of {-1, 0, +1}
mode(&g)                          // → i8, most frequent value (ties → 0)
```

### Health and trends

```rust
health(&g)                        // → GaugeHealth { Healthy, Stuck, Oscillating, Drifting }
drift_rate(&g)                    // → f64, how fast the mean is shifting (second_half - first_half)
is_stationary(&g, window)         // → bool, variance in last `window` samples < 0.1
```

### Gauge state

```rust
g.samples                         // → &[i8], direct access to the sliding window
g.max_history                     // → usize, the window capacity
```

## Architecture

The gauge is a fixed-capacity ring buffer backed by `Vec<i8>`. When the buffer fills, the oldest sample is evicted (via `remove(0)`, which is O(n)). This is intentional—simplicity over micro-optimization. For windows under 1000 samples, the overhead is negligible.

All functions take `&Gauge` (or `&mut Gauge` for `sample`), making it easy to read from multiple threads with a lock or share by cloning.

```
sample stream → [sliding window] → mean / variance / histogram
                                       ↓
                                  health classifier
                                       ↓
                              { Healthy | Stuck | Oscillating | Drifting }
```

## Real-world example: Agent health monitor

```rust
use ternary_gauge::*;

struct AgentMonitor {
    decision_gauge: Gauge,
    action_gauge: Gauge,
}

impl AgentMonitor {
    fn new() -> Self {
        Self {
            decision_gauge: new(50),  // last 50 decisions
            action_gauge: new(100),   // last 100 actions
        }
    }

    fn record_decision(&mut self, decision: i8) {
        sample(&mut self.decision_gauge, decision);
    }

    fn health_report(&self) -> String {
        let d_health = health(&self.decision_gauge);
        let d_mean = mean(&self.decision_gauge);
        let d_mode = mode(&self.decision_gauge);
        let d_drift = drift_rate(&self.decision_gauge);

        format!(
            "Decision gauge: health={:?}, mean={:.2}, mode={}, drift={:.2}",
            d_health, d_mean, d_mode, d_drift
        )
    }

    fn needs_attention(&self) -> bool {
        matches!(health(&self.decision_gauge), 
            GaugeHealth::Stuck | GaugeHealth::Oscillating)
    }
}

// Usage in a simulation loop
let mut monitor = AgentMonitor::new();
for round in 0..1000 {
    let decision = agent.decide();  // returns -1, 0, or +1
    monitor.record_decision(decision);

    if monitor.needs_attention() {
        println!("Round {}: Agent needs attention! {}", round, monitor.health_report());
    }
}
```

## Ecosystem connections

- **ternary-warp** — transform signals before gauging them (smooth → gauge for cleaner readings)
- **ternary-classifier** — use gauges to build `BehaviorProfile`s for species classification
- **ternary-resilience** — cascade failure detection is essentially a network-level gauge
- **ternary-rate-limiter** — the rate limiter's ternary signal (`SpeedUp`/`Normal`/`Throttle`) can be gauged for load patterns

## Performance

| Operation | Complexity |
|-----------|-----------|
| `sample` | O(n) due to `remove(0)` on full buffer |
| `mean` | O(n) |
| `variance` | O(n) |
| `histogram` | O(n) |
| `health` | O(n) |

For production use with large windows, consider replacing the `Vec` with a `VecDeque`. The current design prioritizes simplicity—the entire crate is ~150 lines.

## Open questions

- **Online statistics**: `mean` and `variance` recompute from scratch each call. Welford's algorithm could make these O(1) per sample, but would add state complexity.
- **Adaptive thresholds**: Health detection uses fixed thresholds (0.3 for drift, 0.8 for oscillation, 0.1 for stationarity). These could auto-tune based on signal history.
- **Multi-signal correlation**: A gauge monitors one signal. Correlated health across multiple gauges could detect systemic issues.

## Stats

| Metric | Value |
|--------|-------|
| Tests | 16 |
| Public functions | 9 |
| Public types | 2 (`Gauge`, `GaugeHealth`) |
| Lines of code | ~180 |
| License | MIT |
| Unsafe | 0 |

## Installation

```toml
[dependencies]
ternary-gauge = "0.1.0"
```

## License

MIT

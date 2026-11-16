//! Fixed-memory frame timing; summaries are requested outside the hot path.
pub(super) struct FrameTelemetry {
    tick: [f64; 64],
    interval: [f64; 64],
    next: usize,
    count: usize,
    total: u64,
}
pub(super) struct Summary {
    pub sample_count: usize,
    pub total_frames: u64,
    pub tick_p50_ms: f64,
    pub tick_p95_ms: f64,
    pub interval_p50_ms: f64,
    pub interval_p95_ms: f64,
}
impl Default for FrameTelemetry {
    fn default() -> Self {
        Self {
            tick: [0.; 64],
            interval: [0.; 64],
            next: 0,
            count: 0,
            total: 0,
        }
    }
}
impl FrameTelemetry {
    pub(super) fn record(&mut self, tick_ms: f64, interval_ms: f64) {
        if !tick_ms.is_finite() || !interval_ms.is_finite() || tick_ms < 0. || interval_ms < 0. {
            return;
        }
        self.tick[self.next] = tick_ms;
        self.interval[self.next] = interval_ms;
        self.next = (self.next + 1) % 64;
        self.count = (self.count + 1).min(64);
        self.total = self.total.saturating_add(1);
    }
    pub(super) fn summary(&self) -> Summary {
        let mut tick = self.tick;
        let mut interval = self.interval;
        tick[..self.count].sort_unstable_by(f64::total_cmp);
        interval[..self.count].sort_unstable_by(f64::total_cmp);
        let quantile = |values: &[f64; 64], p: f64| {
            if self.count == 0 {
                0.
            } else {
                values[((self.count as f64 * p).ceil() as usize).saturating_sub(1)]
            }
        };
        Summary {
            sample_count: self.count,
            total_frames: self.total,
            tick_p50_ms: quantile(&tick, 0.5),
            tick_p95_ms: quantile(&tick, 0.95),
            interval_p50_ms: quantile(&interval, 0.5),
            interval_p95_ms: quantile(&interval, 0.95),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_window_replaces_old_samples() {
        let mut f = FrameTelemetry::default();
        for n in 1..=100 {
            f.record(n as f64, n as f64 * 2.);
        }
        let s = f.summary();
        assert_eq!(s.sample_count, 64);
        assert_eq!(s.total_frames, 100);
        assert_eq!(s.tick_p50_ms, 68.);
        assert_eq!(s.tick_p95_ms, 97.);
        assert_eq!(s.interval_p50_ms, 136.);
        assert_eq!(s.interval_p95_ms, 194.);
    }
    #[test]
    fn refuses_invalid_samples_without_losing_zero_time_measurements() {
        let mut f = FrameTelemetry::default();
        f.record(f64::NAN, 16.);
        f.record(1., f64::INFINITY);
        f.record(-1., 16.);
        assert_eq!(f.summary().sample_count, 0);
        f.record(0., 16.);
        assert_eq!(f.summary().tick_p95_ms, 0.);
        assert_eq!(f.summary().sample_count, 1);
    }
    #[test]
    fn summary_does_not_change_the_window() {
        let mut f = FrameTelemetry::default();
        f.record(4., 20.);
        f.record(2., 30.);
        assert_eq!(f.summary().tick_p50_ms, 2.);
        assert_eq!(f.summary().tick_p95_ms, 4.);
        f.record(3., 10.);
        assert_eq!(f.summary().tick_p50_ms, 3.);
        assert_eq!(f.total, 3);
    }
}

//! Fixed-memory frame timing and device capability telemetry; summaries are requested outside the hot path.

pub(crate) const READING_FALLBACK_URL: &str = "/publication/reading.html";
pub(crate) const DROPPED_FRAME_THRESHOLD_MS: f64 = 33.3;

#[allow(dead_code)]
pub(crate) type FrameWindow = FrameTelemetry;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TierBudgetTelemetry {
    pub tier_id: &'static str,
    pub max_resident_triangles: u64,
    pub max_gpu_memory_bytes: usize,
    pub max_first_paint_bytes: usize,
    pub max_texture_dim: u32,
    pub max_dpr: f64,
    pub preferred_lod: &'static str,
}

impl Default for TierBudgetTelemetry {
    fn default() -> Self {
        Self {
            tier_id: "tier-b",
            max_resident_triangles: 320_000,
            max_gpu_memory_bytes: 192 * 1024 * 1024,
            max_first_paint_bytes: 17 * 1024 * 1024,
            max_texture_dim: 1024,
            max_dpr: 1.35,
            preferred_lod: "lod1",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DeviceTelemetry {
    pub selected_backend: Option<String>,
    pub requested_backend: &'static str,
    pub device_tier: &'static str,
    pub tier_budget: TierBudgetTelemetry,
    pub is_mobile: bool,
    pub reading_fallback: &'static str,
}

impl Default for DeviceTelemetry {
    fn default() -> Self {
        Self {
            selected_backend: None,
            requested_backend: "webgpu-or-webgl2",
            device_tier: "tier-b",
            tier_budget: TierBudgetTelemetry::default(),
            is_mobile: false,
            reading_fallback: READING_FALLBACK_URL,
        }
    }
}

pub(crate) struct FrameTelemetry {
    tick: [f64; 64],
    interval: [f64; 64],
    next: usize,
    count: usize,
    total: u64,
    dropped: u64,
    device: DeviceTelemetry,
}

#[allow(dead_code)]
pub(crate) struct Summary {
    pub sample_count: usize,
    pub total_frames: u64,
    pub dropped_frames: u64,
    pub fps: f64,
    pub avg_frame_ms: f64,
    pub max_frame_ms: f64,
    pub tick_p50_ms: f64,
    pub tick_p95_ms: f64,
    pub interval_p50_ms: f64,
    pub interval_p95_ms: f64,
    pub selected_backend: Option<String>,
    pub requested_backend: &'static str,
    pub device_tier: &'static str,
    pub tier_budget: TierBudgetTelemetry,
    pub is_mobile: bool,
    pub reading_fallback: &'static str,
}

impl Default for FrameTelemetry {
    fn default() -> Self {
        Self {
            tick: [0.; 64],
            interval: [0.; 64],
            next: 0,
            count: 0,
            total: 0,
            dropped: 0,
            device: DeviceTelemetry::default(),
        }
    }
}

impl FrameTelemetry {
    #[allow(dead_code)]
    pub(crate) fn with_device_context(device: DeviceTelemetry) -> Self {
        Self {
            device,
            ..Self::default()
        }
    }

    pub(crate) fn record(&mut self, tick_ms: f64, interval_ms: f64) {
        if !tick_ms.is_finite() || !interval_ms.is_finite() || tick_ms < 0. || interval_ms < 0. {
            return;
        }
        if interval_ms > DROPPED_FRAME_THRESHOLD_MS {
            self.dropped = self.dropped.saturating_add(1);
        }
        self.tick[self.next] = tick_ms;
        self.interval[self.next] = interval_ms;
        self.next = (self.next + 1) % 64;
        self.count = (self.count + 1).min(64);
        self.total = self.total.saturating_add(1);

        #[cfg(target_arch = "wasm32")]
        if self.total <= 4 || self.total % 12 == 0 {
            self.publish_frame_stats_js();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn avg_frame_ms(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let mut sum = 0.0;
        let mut i = 0;
        while i < self.count {
            sum += self.interval[i];
            i += 1;
        }
        sum / self.count as f64
    }

    #[allow(dead_code)]
    pub(crate) fn fps(&self) -> f64 {
        let avg = self.avg_frame_ms();
        if !avg.is_finite() || avg <= 0.0 {
            0.0
        } else {
            1000.0 / avg
        }
    }

    #[allow(dead_code)]
    pub(crate) fn p95_frame_ms(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let mut interval = self.interval;
        interval[..self.count].sort_unstable_by(f64::total_cmp);
        interval[((self.count as f64 * 0.95).ceil() as usize).saturating_sub(1)]
    }

    #[allow(dead_code)]
    pub(crate) fn max_frame_ms(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let mut max_v = 0.0_f64;
        let mut i = 0;
        while i < self.count {
            if self.interval[i] > max_v {
                max_v = self.interval[i];
            }
            i += 1;
        }
        max_v
    }

    #[allow(dead_code)]
    pub(crate) fn dropped_frames(&self) -> u64 {
        self.dropped
    }

    #[allow(dead_code)]
    pub(crate) fn total_frames(&self) -> u64 {
        self.total
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn publish_frame_stats_js(&self) {
        let Some(win) = web_sys::window() else {
            return;
        };
        let obj = js_sys::Object::new();
        let set_f64 = |key: &str, value: f64| {
            let _ = js_sys::Reflect::set(
                &obj,
                &wasm_bindgen::JsValue::from_str(key),
                &wasm_bindgen::JsValue::from_f64(value),
            );
        };
        set_f64("fps", (self.fps() * 10.0).round() / 10.0);
        set_f64("avg_ms", (self.avg_frame_ms() * 100.0).round() / 100.0);
        set_f64("p95_ms", (self.p95_frame_ms() * 100.0).round() / 100.0);
        set_f64("max_ms", (self.max_frame_ms() * 100.0).round() / 100.0);
        set_f64("dropped", self.dropped_frames() as f64);
        set_f64("total_frames", self.total_frames() as f64);
        let _ = js_sys::Reflect::set(
            &win,
            &wasm_bindgen::JsValue::from_str("__webizenFrameStats"),
            &obj,
        );
    }

    pub(crate) fn summary(&self) -> Summary {
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
        let avg_frame_ms = self.avg_frame_ms();
        let fps = if !avg_frame_ms.is_finite() || avg_frame_ms <= 0.0 {
            0.0
        } else {
            1000.0 / avg_frame_ms
        };
        let max_frame_ms = if self.count == 0 {
            0.0
        } else {
            interval[self.count - 1]
        };
        Summary {
            sample_count: self.count,
            total_frames: self.total,
            dropped_frames: self.dropped,
            fps,
            avg_frame_ms,
            max_frame_ms,
            tick_p50_ms: quantile(&tick, 0.5),
            tick_p95_ms: quantile(&tick, 0.95),
            interval_p50_ms: quantile(&interval, 0.5),
            interval_p95_ms: quantile(&interval, 0.95),
            selected_backend: self.device.selected_backend.clone(),
            requested_backend: self.device.requested_backend,
            device_tier: self.device.device_tier,
            tier_budget: self.device.tier_budget,
            is_mobile: self.device.is_mobile,
            reading_fallback: self.device.reading_fallback,
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

    #[test]
    fn summary_carries_device_tier_and_reading_fallback() {
        let f = FrameTelemetry::with_device_context(DeviceTelemetry {
            selected_backend: Some("webgl2".to_string()),
            requested_backend: "webgl2",
            device_tier: "tier-c",
            tier_budget: TierBudgetTelemetry {
                tier_id: "tier-c",
                max_resident_triangles: 150_000,
                max_gpu_memory_bytes: 96 * 1024 * 1024,
                max_first_paint_bytes: 12 * 1024 * 1024,
                max_texture_dim: 1024,
                max_dpr: 1.15,
                preferred_lod: "lod1",
            },
            is_mobile: true,
            reading_fallback: READING_FALLBACK_URL,
        });
        let s = f.summary();
        assert_eq!(s.selected_backend.as_deref(), Some("webgl2"));
        assert_eq!(s.requested_backend, "webgl2");
        assert_eq!(s.device_tier, "tier-c");
        assert_eq!(s.tier_budget.max_dpr, 1.15);
        assert!(s.is_mobile);
        assert_eq!(s.reading_fallback, "/publication/reading.html");
    }

    #[test]
    fn live_fps_and_average_frame_time_match_60hz_cadence() {
        let mut f = FrameWindow::default();
        assert_eq!(f.fps(), 0.0);
        assert_eq!(f.avg_frame_ms(), 0.0);
        assert_eq!(f.max_frame_ms(), 0.0);
        assert_eq!(f.total_frames(), 0);

        for _ in 0..20 {
            f.record(3.2, 16.0);
        }
        assert!((f.avg_frame_ms() - 16.0).abs() < 1e-9);
        assert!((f.fps() - 62.5).abs() < 1e-9);
        assert!((f.max_frame_ms() - 16.0).abs() < 1e-9);
        assert_eq!(f.total_frames(), 20);
        assert_eq!(f.dropped_frames(), 0);
    }

    #[test]
    fn p95_frame_ms_and_max_frame_ms_reflect_tail_latency() {
        let mut f = FrameWindow::default();
        for i in 1..=20 {
            f.record(2.0, i as f64 * 2.0);
        }
        // Sorted intervals: 2, 4, ..., 38, 40 -> ceil(20 * 0.95) - 1 = 18 -> 38.0 ms
        assert_eq!(f.p95_frame_ms(), 38.0);
        assert_eq!(f.max_frame_ms(), 40.0);
        assert_eq!(f.summary().interval_p95_ms, 38.0);
    }

    #[test]
    fn dropped_frames_counts_intervals_strictly_above_33_3ms() {
        let mut f = FrameWindow::default();
        f.record(2.0, 16.6);
        f.record(2.0, 33.3); // boundary: not dropped
        f.record(2.0, 33.4); // dropped #1
        f.record(2.0, 50.0); // dropped #2
        f.record(2.0, f64::NAN); // invalid: ignored
        assert_eq!(f.dropped_frames(), 2);
        assert_eq!(f.total_frames(), 4);
        assert_eq!(f.summary().dropped_frames, 2);
    }
}

//! Metrics collection using HdrHistogram for accurate percentile calculations.

use hdrhistogram::Histogram;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Thread-safe metrics collector for benchmark results
pub struct MetricsCollector {
    /// Histogram for total request latency (microseconds)
    latency_histogram: Mutex<Histogram<u64>>,
    /// Histogram for time to first byte (microseconds)
    ttfb_histogram: Mutex<Histogram<u64>>,
    /// Total successful requests
    success_count: AtomicU64,
    /// Total failed requests
    error_count: AtomicU64,
    /// Total bytes received
    bytes_received: AtomicU64,
    /// Start time of the benchmark
    start_time: Mutex<Option<Instant>>,
    /// End time of the benchmark
    end_time: Mutex<Option<Instant>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            // Histogram for latencies up to 10 minutes with 3 significant figures
            latency_histogram: Mutex::new(
                Histogram::new_with_bounds(1, 600_000_000, 3).unwrap(),
            ),
            ttfb_histogram: Mutex::new(
                Histogram::new_with_bounds(1, 600_000_000, 3).unwrap(),
            ),
            success_count: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            start_time: Mutex::new(None),
            end_time: Mutex::new(None),
        }
    }

    /// Mark the start of the benchmark
    pub fn start(&self) {
        *self.start_time.lock().unwrap() = Some(Instant::now());
    }

    /// Mark the end of the benchmark
    pub fn stop(&self) {
        *self.end_time.lock().unwrap() = Some(Instant::now());
    }

    /// Record a successful request
    pub fn record_success(&self, latency: Duration, ttfb: Option<Duration>, bytes: u64) {
        let latency_us = latency.as_micros() as u64;
        if let Ok(mut hist) = self.latency_histogram.lock() {
            let _ = hist.record(latency_us.max(1));
        }

        if let Some(ttfb) = ttfb {
            let ttfb_us = ttfb.as_micros() as u64;
            if let Ok(mut hist) = self.ttfb_histogram.lock() {
                let _ = hist.record(ttfb_us.max(1));
            }
        }

        self.success_count.fetch_add(1, Ordering::Relaxed);
        self.bytes_received.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a failed request
    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the total number of successful requests
    pub fn success_count(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed)
    }

    /// Get the total number of failed requests
    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::Relaxed)
    }

    /// Get the total number of requests
    pub fn total_requests(&self) -> u64 {
        self.success_count() + self.error_count()
    }

    /// Get the success rate as a percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.total_requests();
        if total == 0 {
            return 100.0;
        }
        (self.success_count() as f64 / total as f64) * 100.0
    }

    /// Get the elapsed duration
    pub fn elapsed(&self) -> Duration {
        let start = self.start_time.lock().unwrap();
        let end = self.end_time.lock().unwrap();
        match (*start, *end) {
            (Some(s), Some(e)) => e.duration_since(s),
            (Some(s), None) => s.elapsed(),
            _ => Duration::ZERO,
        }
    }

    /// Get requests per second
    pub fn requests_per_second(&self) -> f64 {
        let elapsed = self.elapsed().as_secs_f64();
        if elapsed == 0.0 {
            return 0.0;
        }
        self.success_count() as f64 / elapsed
    }

    /// Get latency percentile in milliseconds
    pub fn latency_percentile(&self, percentile: f64) -> f64 {
        let hist = self.latency_histogram.lock().unwrap();
        hist.value_at_percentile(percentile) as f64 / 1000.0
    }

    /// Get TTFB percentile in milliseconds
    pub fn ttfb_percentile(&self, percentile: f64) -> f64 {
        let hist = self.ttfb_histogram.lock().unwrap();
        hist.value_at_percentile(percentile) as f64 / 1000.0
    }

    /// Get total bytes received
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Reset all metrics
    pub fn reset(&self) {
        if let Ok(mut hist) = self.latency_histogram.lock() {
            hist.reset();
        }
        if let Ok(mut hist) = self.ttfb_histogram.lock() {
            hist.reset();
        }
        self.success_count.store(0, Ordering::Relaxed);
        self.error_count.store(0, Ordering::Relaxed);
        self.bytes_received.store(0, Ordering::Relaxed);
        *self.start_time.lock().unwrap() = None;
        *self.end_time.lock().unwrap() = None;
    }

    /// Create a snapshot of current metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            success_count: self.success_count(),
            error_count: self.error_count(),
            success_rate: self.success_rate(),
            requests_per_second: self.requests_per_second(),
            latency_p50: self.latency_percentile(50.0),
            latency_p95: self.latency_percentile(95.0),
            latency_p99: self.latency_percentile(99.0),
            ttfb_p50: self.ttfb_percentile(50.0),
            ttfb_p95: self.ttfb_percentile(95.0),
            ttfb_p99: self.ttfb_percentile(99.0),
            bytes_received: self.bytes_received(),
            elapsed_secs: self.elapsed().as_secs_f64(),
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// A snapshot of metrics at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub success_count: u64,
    pub error_count: u64,
    pub success_rate: f64,
    pub requests_per_second: f64,
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub latency_p99: f64,
    pub ttfb_p50: f64,
    pub ttfb_p95: f64,
    pub ttfb_p99: f64,
    pub bytes_received: u64,
    pub elapsed_secs: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_collector() {
        let collector = MetricsCollector::new();
        collector.start();

        // Record some successes
        collector.record_success(Duration::from_millis(100), Some(Duration::from_millis(50)), 1000);
        collector.record_success(Duration::from_millis(150), Some(Duration::from_millis(60)), 1000);
        collector.record_success(Duration::from_millis(200), Some(Duration::from_millis(70)), 1000);

        // Record an error
        collector.record_error();

        collector.stop();

        assert_eq!(collector.success_count(), 3);
        assert_eq!(collector.error_count(), 1);
        assert_eq!(collector.total_requests(), 4);
        assert!((collector.success_rate() - 75.0).abs() < 0.01);
        assert_eq!(collector.bytes_received(), 3000);
    }
}

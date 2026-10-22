//! Shared measurement-only System allocator forwarding and finite probe output.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};
struct Allocator;
static COUNT: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
static TRACK: AtomicBool = AtomicBool::new(false);
// SAFETY: all pointers/layouts are forwarded unchanged to System. Only lock-free
// counters are touched; they do not allocate or affect allocator ownership.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        if TRACK.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(l.size() as u64, Ordering::Relaxed);
        }
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        if TRACK.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
            BYTES.fetch_add(n as u64, Ordering::Relaxed);
        }
        unsafe { System.realloc(p, l, n) }
    }
}
#[global_allocator]
static ALLOC: Allocator = Allocator;

const MAX_ITERATIONS: usize = 1 << 26;
const DEFAULT_TARGET_NS: u128 = 20_000_000;

fn output_only() -> bool {
    std::env::var_os("MATEA_PROBE_OUTPUT_ONLY").is_some_and(|value| value == "1")
}

fn parse_iterations(value: Option<&str>, default: usize) -> Result<usize, &'static str> {
    let count = match value {
        Some(text) => text.parse().map_err(|_| "invalid iteration count")?,
        None => default,
    };
    // Existing callers multiply the starting count by at most ten.
    if count == 0 || count > MAX_ITERATIONS / 10 {
        return Err("starting iterations must be 1..=6710886 (before caller scaling)");
    }
    Ok(count)
}

pub(crate) fn iterations(default: usize) -> usize {
    if output_only() {
        return default;
    }
    let text = std::env::var("MATEA_PERF_ITERATIONS");
    let value = match &text {
        Ok(value) => Some(value.as_str()),
        Err(std::env::VarError::NotPresent) => None,
        Err(_) => panic!("MATEA_PERF_ITERATIONS must be Unicode decimal digits"),
    };
    parse_iterations(value, default).unwrap_or_else(|error| panic!("{error}"))
}

#[derive(Clone, Copy, Debug)]
struct Config {
    target_ns: u128,
    max_n: usize,
}

impl Config {
    fn parse(value: Option<&str>) -> Result<Self, &'static str> {
        let target_ns = match value {
            None => DEFAULT_TARGET_NS,
            Some(text) => text.parse().map_err(|_| "invalid sample target")?,
        };
        if target_ns > 1_000_000_000 {
            return Err("MATEA_PERF_MIN_SAMPLE_NS must be 0..=1000000000; 0 selects fixed batches");
        }
        Ok(Self {
            target_ns,
            max_n: MAX_ITERATIONS,
        })
    }

    fn environment() -> Self {
        let text = std::env::var("MATEA_PERF_MIN_SAMPLE_NS");
        let value = match &text {
            Ok(value) => Some(value.as_str()),
            Err(std::env::VarError::NotPresent) => None,
            Err(_) => panic!("MATEA_PERF_MIN_SAMPLE_NS must be Unicode decimal digits"),
        };
        Self::parse(value).unwrap_or_else(|error| panic!("{error}"))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct CalibrationBatch {
    iterations: usize,
    observations_ns: [u128; 3],
}

#[derive(Debug)]
struct CalibrationError {
    reason: &'static str,
    history: Vec<CalibrationBatch>,
}

fn calibrate(
    starting: usize,
    config: Config,
    mut batch: impl FnMut(usize) -> u128,
) -> Result<(usize, Vec<CalibrationBatch>), CalibrationError> {
    let mut history = Vec::new();
    if starting == 0 || starting > config.max_n || config.target_ns == 0 {
        return Err(CalibrationError {
            reason: "invalid calibration bounds",
            history,
        });
    }
    let mut count = starting;
    loop {
        let observations_ns = std::array::from_fn(|_| batch(count));
        let ready = observations_ns
            .iter()
            .all(|&elapsed| elapsed >= config.target_ns);
        history.push(CalibrationBatch {
            iterations: count,
            observations_ns,
        });
        if ready {
            return Ok((count, history));
        }
        if count == config.max_n {
            return Err(CalibrationError {
                reason: "sample target not reached at iteration limit",
                history,
            });
        }
        count = if count > config.max_n / 2 {
            config.max_n
        } else {
            count * 2
        };
    }
}

// One monomorphized non-inlined hot loop is used by both calibration and samples.
#[inline(never)]
fn timed_batch(count: usize, operation: &mut impl FnMut() -> u64) -> u128 {
    let start = Instant::now();
    for _ in 0..count {
        black_box(operation());
    }
    start.elapsed().as_nanos()
}

fn retained_samples(mut batch: impl FnMut() -> u128) -> Result<[u128; 31], usize> {
    let mut samples = [0; 31];
    for (index, elapsed) in samples.iter_mut().enumerate() {
        *elapsed = batch();
        if *elapsed == 0 {
            return Err(index);
        }
    }
    Ok(samples)
}

struct AllocationPass;

impl AllocationPass {
    fn start() -> Self {
        assert!(
            TRACK
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok(),
            "nested allocation pass"
        );
        COUNT.store(0, Ordering::Relaxed);
        BYTES.store(0, Ordering::Relaxed);
        Self
    }
}

impl Drop for AllocationPass {
    fn drop(&mut self) {
        TRACK.store(false, Ordering::Relaxed);
    }
}
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}
pub(crate) fn measure(
    id: &str,
    iterations: usize,
    mut operation: impl FnMut() -> u64,
    output: &str,
    semantics: &str,
) {
    // CI verifies outputs without publishing fabricated or coarse-clock times.
    // The distinct record kind is deliberately rejected by timing readers.
    if output_only() {
        let first_return_value = black_box(operation());
        println!(
            "{{\"record_kind\":\"probe-output\",\"case\":{},\"semantics\":{},\"first_return_value\":{first_return_value},\"output\":{}}}",
            quote(id),
            quote(semantics),
            quote(output)
        );
        return;
    }
    let config = Config::environment();
    assert!(
        iterations > 0 && iterations <= config.max_n,
        "iterations must be positive and bounded"
    );
    let start = Instant::now();
    let first_return_value = black_box(operation());
    let cold = start.elapsed().as_nanos();
    for _ in 0..100 {
        black_box(operation());
    }
    let starting = iterations;
    let (iterations, history) = if config.target_ns == 0 {
        (iterations, Vec::new())
    } else {
        calibrate(iterations, config, |count| {
            timed_batch(count, &mut operation)
        })
        .unwrap_or_else(|error| {
            panic!("{}: {}; observations={:?}", id, error.reason, error.history)
        })
    };
    let samples = retained_samples(|| timed_batch(iterations, &mut operation))
        .unwrap_or_else(|index| panic!("{id}: nonpositive sample at index={index}, iterations={iterations}, calibration={history:?}; no samples retried"));
    let pass = AllocationPass::start();
    black_box(operation());
    drop(pass);
    let allocations = COUNT.load(Ordering::Relaxed);
    let allocated_bytes = BYTES.load(Ordering::Relaxed);
    let calibration = if config.target_ns == 0 {
        String::new()
    } else {
        let batches = history
            .iter()
            .map(|batch| {
                let [elapsed, a, b] = batch.observations_ns;
                format!(
                    "{{\"iterations\":{},\"elapsed_ns\":{elapsed},\"confirmations_ns\":[{a},{b}]}}",
                    batch.iterations
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            ",\"calibration\":{{\"target_ns\":{},\"start_n\":{starting},\"max_n\":{},\"batches\":[{batches}]}}",
            config.target_ns, config.max_n
        )
    };
    println!(
        "{{\"case\":{},\"semantics\":{},\"cold_ns\":{cold},\"first_return_value\":{first_return_value},\"iterations_per_sample\":{iterations},\"samples_ns\":[{}],\"allocations\":{allocations},\"allocated_bytes\":{allocated_bytes},\"output\":{}{calibration}}}",
        quote(id),
        quote(semantics),
        samples
            .iter()
            .map(|&elapsed| (elapsed as f64 / iterations as f64).to_string())
            .collect::<Vec<_>>()
            .join(","),
        quote(output)
    );
}

#[cfg(test)]
#[allow(dead_code, unused_imports)] // Also included by custom, non-libtest benches.
mod tests {
    use super::*;

    #[test]
    fn configuration_is_bounded_and_fixed_mode_is_explicit() {
        assert_eq!(Config::parse(None).unwrap().target_ns, 20_000_000);
        for target in ["0", "1", "1000000000"] {
            assert_eq!(
                Config::parse(Some(target)).unwrap().target_ns,
                target.parse::<u128>().unwrap()
            );
        }
        for target in [
            "",
            "-1",
            "nan",
            "1000000001",
            "340282366920938463463374607431768211456",
        ] {
            assert!(Config::parse(Some(target)).is_err());
        }
        assert_eq!(parse_iterations(None, 300), Ok(300));
        assert_eq!(parse_iterations(Some("6710886"), 300), Ok(6_710_886));
        for count in ["", "0", "-1", "no", "6710887", "184467440737095516160"] {
            assert!(parse_iterations(Some(count), 300).is_err());
        }
        assert!(parse_iterations(None, 0).is_err());
    }

    #[test]
    fn calibration_keeps_all_observations_and_requires_three_confirmations() {
        let mut counts = Vec::new();
        let mut durations = [1000, 0, 0, 10, 11, 12].into_iter();
        let (actual, history) = calibrate(
            3,
            Config {
                target_ns: 10,
                max_n: 10,
            },
            |n| {
                counts.push(n);
                durations.next().unwrap()
            },
        )
        .unwrap();
        assert_eq!(counts, [3, 3, 3, 6, 6, 6]);
        assert_eq!(actual, 6);
        assert_eq!(
            history,
            [
                CalibrationBatch {
                    iterations: 3,
                    observations_ns: [1000, 0, 0]
                },
                CalibrationBatch {
                    iterations: 6,
                    observations_ns: [10, 11, 12]
                },
            ]
        );
    }

    #[test]
    fn calibration_failure_preserves_coarse_clock_history_at_the_cap() {
        let error = calibrate(
            3,
            Config {
                target_ns: 10,
                max_n: 10,
            },
            |_| 0,
        )
        .unwrap_err();
        assert_eq!(error.reason, "sample target not reached at iteration limit");
        assert_eq!(
            error
                .history
                .iter()
                .map(|v| v.iterations)
                .collect::<Vec<_>>(),
            [3, 6, 10]
        );
        assert!(error.history.iter().all(|v| v.observations_ns == [0; 3]));
    }

    #[test]
    fn invalid_calibration_performs_no_operations() {
        for (start, target, maximum) in [(0, 10, 10), (11, 10, 10), (1, 0, 10), (1, 10, 0)] {
            let mut calls = 0;
            let error = calibrate(
                start,
                Config {
                    target_ns: target,
                    max_n: maximum,
                },
                |_| {
                    calls += 1;
                    10
                },
            )
            .unwrap_err();
            assert_eq!(calls, 0);
            assert!(error.history.is_empty());
        }
    }

    #[test]
    fn calibration_growth_never_overflows_usize() {
        let start = usize::MAX / 2 + 1;
        let (actual, history) = calibrate(
            start,
            Config {
                target_ns: 1,
                max_n: usize::MAX,
            },
            |n| u128::from(n == usize::MAX),
        )
        .unwrap();
        assert_eq!(actual, usize::MAX);
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].iterations, start);
        assert_eq!(history[1].iterations, usize::MAX);
    }

    #[test]
    fn retained_samples_keep_every_interval_and_never_retry_a_zero() {
        let mut calls = 0;
        let samples = retained_samples(|| {
            calls += 1;
            calls
        })
        .unwrap();
        assert_eq!(samples, std::array::from_fn(|index| index as u128 + 1));
        assert_eq!(calls, 31);
        let mut calls = 0;
        assert_eq!(
            retained_samples(|| {
                calls += 1;
                if calls == 8 { 0 } else { 10 }
            }),
            Err(7)
        );
        assert_eq!(calls, 8);
    }

    #[test]
    fn shared_batch_runs_exactly_the_requested_stateful_operations() {
        let mut state = 0;
        let mut operation = || {
            state += 1;
            state
        };
        timed_batch(3, &mut operation);
        timed_batch(5, &mut operation);
        assert_eq!(state, 8);
    }

    #[test]
    fn allocation_tracking_is_released_on_unwind_and_rejects_nesting() {
        assert!(!TRACK.load(Ordering::Relaxed));
        let failure = std::panic::catch_unwind(|| {
            let _pass = AllocationPass::start();
            assert!(TRACK.load(Ordering::Relaxed));
            assert!(std::panic::catch_unwind(AllocationPass::start).is_err());
            assert!(TRACK.load(Ordering::Relaxed));
            panic!("operation failure");
        });
        assert!(failure.is_err());
        assert!(!TRACK.load(Ordering::Relaxed));
        let pass = AllocationPass::start();
        drop(pass);
        assert!(!TRACK.load(Ordering::Relaxed));
    }
}

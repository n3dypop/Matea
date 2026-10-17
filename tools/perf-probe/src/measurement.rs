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
    let start = Instant::now();
    let first_return_value = black_box(operation());
    let cold = start.elapsed().as_nanos();
    for _ in 0..100 {
        black_box(operation());
    }
    let mut samples = Vec::with_capacity(31);
    for _ in 0..31 {
        let start = Instant::now();
        for _ in 0..iterations {
            black_box(operation());
        }
        samples.push(start.elapsed().as_nanos() as f64 / iterations as f64);
    }
    COUNT.store(0, Ordering::Relaxed);
    BYTES.store(0, Ordering::Relaxed);
    TRACK.store(true, Ordering::Relaxed);
    black_box(operation());
    TRACK.store(false, Ordering::Relaxed);
    println!(
        "{{\"case\":{},\"semantics\":{},\"cold_ns\":{cold},\"first_return_value\":{first_return_value},\"iterations_per_sample\":{iterations},\"samples_ns\":[{}],\"allocations\":{},\"allocated_bytes\":{},\"output\":{}}}",
        quote(id),
        quote(semantics),
        samples
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        COUNT.load(Ordering::Relaxed),
        BYTES.load(Ordering::Relaxed),
        quote(output)
    );
}

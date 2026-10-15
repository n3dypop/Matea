//! Matched Foundation micro-workloads. No image or GPU processing.
//! Allocation instrumentation is enabled only during the separate allocation pass.
use matea::{
    prelude::*,
    terminal::{Capabilities, FrameEncoder},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};
use unicode_width::UnicodeWidthStr;
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
fn quote(s: &str) -> String {
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
fn trace() {
    let caps = Capabilities {
        color: ColorLevel::None,
        unicode: true,
        reduced_motion: true,
    };
    let mut encoder = FrameEncoder::default();
    let mut previous = None;
    let mut sequences = Vec::new();
    for c in ['a', 'b', 'a'] {
        let frame = View::text(format!("{c}{}", "a".repeat(79)))
            .render(Size::new(80, 24))
            .unwrap();
        let mut bytes = Vec::new();
        encoder
            .write(&mut bytes, &frame, previous.as_ref(), caps)
            .unwrap();
        sequences.push(quote(&String::from_utf8(bytes).unwrap()));
        previous = Some(frame);
    }
    println!(
        "{{\"width\":80,\"height\":24,\"sequences\":[{}]}}",
        sequences.join(",")
    );
}
fn measure(
    id: &str,
    iterations: usize,
    mut operation: impl FnMut() -> u64,
    output: &str,
    semantics: &str,
) {
    let start = Instant::now();
    black_box(operation());
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
        "{{\"case\":{},\"semantics\":{},\"cold_ns\":{cold},\"iterations_per_sample\":{iterations},\"samples_ns\":[{}],\"allocations\":{},\"allocated_bytes\":{},\"output\":{}}}",
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
fn main() {
    if std::env::args().any(|arg| arg == "--trace") {
        trace();
        return;
    }
    let iterations = std::env::var("MATEA_PERF_ITERATIONS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(300);
    input_bindings(iterations);
    projectiles(iterations);
    pagination(iterations);
    let size = Size::new(80, 24);
    for (name, text) in [
        ("empty", String::new()),
        (
            "ascii",
            (0..20)
                .map(|_| "Matea tea studio: identical ASCII row")
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        (
            "unicode",
            (0..20)
                .map(|_| "抹茶 e\u{301} 👩‍💻 tea")
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    ] {
        let view = View::text(&text);
        let output = view.render(size).unwrap().plain();
        measure(
            &format!("render/{name}_80x24"),
            iterations,
            || {
                let s = black_box(view.render(size).unwrap());
                black_box(s.row(0));
                u64::from(s.size().width)
            },
            &output,
            "fresh clipped grapheme cell surface, default style, non-CJK ambiguous width",
        );
    }
    let text = "抹茶 e\u{301} 👩‍💻 ".repeat(16);
    let width = text.width();
    measure(
        "unicode/width",
        iterations * 10,
        || black_box(text.as_str()).width() as u64,
        &width.to_string(),
        "grapheme display width, no control characters, non-CJK ambiguous width",
    );
    let frame = View::text("a".repeat(80)).render(size).unwrap();
    let caps = Capabilities {
        color: ColorLevel::None,
        unicode: true,
        reduced_motion: true,
    };
    let mut sink = Vec::with_capacity(8192);
    let mut encoder = FrameEncoder::default();
    encoder.write(&mut sink, &frame, None, caps).unwrap();
    measure(
        "terminal/unchanged_80x24",
        iterations,
        || {
            sink.clear();
            encoder
                .write(&mut sink, &frame, Some(&frame), caps)
                .unwrap();
            sink.len() as u64
        },
        "",
        "encode unchanged existing frame to reused memory sink; no terminal I/O",
    );
    let next = View::text(format!("b{}", "a".repeat(79)))
        .render(size)
        .unwrap();
    let mut flip = false;
    measure(
        "terminal/one_cell_80x24",
        iterations,
        || {
            sink.clear();
            let (a, b) = if flip {
                (&frame, &next)
            } else {
                (&next, &frame)
            };
            encoder.write(&mut sink, a, Some(b), caps).unwrap();
            flip = !flip;
            sink.len() as u64
        },
        "one ASCII cell toggled; independently replay output before claiming equivalence",
        "toggle one existing ASCII cell, encode to reused memory sink; output encoding may differ",
    );
    for damping in [0.2, 1.0, 2.0] {
        use matea::motion::{Motion, Spring};
        let make = || {
            Spring::new(
                std::time::Duration::from_millis(16),
                black_box(12.0),
                black_box(damping),
            )
            .unwrap()
        };
        let spring = make();
        let state = Motion {
            position: 2.0,
            velocity: 0.0,
        };
        let output = spring.step(state, 1.0).unwrap();
        let expected = format!("{:.12},{:.12}", output.position, output.velocity);
        measure(
            &format!("motion/spring_update_{damping}"),
            (iterations * 10).max(30000),
            || {
                let m = spring.sample(black_box(state), black_box(1.0));
                m.position.to_bits() ^ m.velocity.to_bits()
            },
            &expected,
            "cached spring update dt=16ms omega=12 state=(2,0) target=1; finite inputs",
        );
        measure(
            &format!("motion/spring_checked_update_{damping}"),
            (iterations * 10).max(30000),
            || {
                let m = spring.step(black_box(state), black_box(1.0)).unwrap();
                m.position.to_bits() ^ m.velocity.to_bits()
            },
            &expected,
            "cached spring update with finite input/result checks dt=16ms omega=12 state=(2,0) target=1",
        );
        measure(
            &format!("motion/spring_construct_{damping}"),
            (iterations * 10).max(30000),
            || {
                let m = black_box(make())
                    .step(black_box(state), black_box(1.0))
                    .unwrap();
                m.position.to_bits() ^ m.velocity.to_bits()
            },
            &expected,
            "construct spring coefficients and update once dt=16ms omega=12 state=(2,0) target=1; finite inputs",
        );
    }
}
fn projectiles(iterations: usize) {
    use matea::motion::{Point3, Projectile, TERMINAL_GRAVITY, Vector3, frame_duration};
    use std::{num::NonZeroU32, time::Duration};
    let fps = NonZeroU32::new(60).unwrap();
    measure(
        "motion/frame_duration",
        (iterations * 10).max(300000),
        || frame_duration(black_box(fps)).as_secs_f64().to_bits(),
        &format!("{:.12}", frame_duration(fps).as_secs_f64()),
        "positive fps=60; integer nanosecond period converted to seconds",
    );
    let make = || {
        Projectile::new(
            Duration::from_millis(16),
            black_box(Point3 {
                x: 1.0,
                y: 10.0,
                z: -2.0,
            }),
            black_box(Vector3 {
                x: 2.0,
                y: -3.0,
                z: 4.0,
            }),
            black_box(TERMINAL_GRAVITY),
        )
        .unwrap()
    };
    let initial = make();
    let result = initial.sample();
    let (p, v) = (result.position(), result.velocity());
    let expected = format!(
        "{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}",
        p.x, p.y, p.z, v.x, v.y, v.z
    );
    for checked in [false, true] {
        measure(
            if checked {
                "motion/projectile_checked_update"
            } else {
                "motion/projectile_update"
            },
            (iterations * 10).max(300000),
            || {
                let mut next = black_box(initial);
                if checked {
                    next.step().unwrap();
                } else {
                    next = next.sample();
                }
                let (p, v, a) = (next.position(), next.velocity(), next.acceleration());
                p.x.to_bits()
                    ^ p.y.to_bits()
                    ^ p.z.to_bits()
                    ^ v.x.to_bits()
                    ^ v.y.to_bits()
                    ^ v.z.to_bits()
                    ^ a.y.to_bits()
            },
            &expected,
            if checked {
                "copy initial projectile; forward Euler and six finite result checks; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0); read all getters"
            } else {
                "copy initial projectile; raw forward Euler; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0); read all getters"
            },
        );
    }
    measure(
        "motion/projectile_construct",
        (iterations * 10).max(300000),
        || {
            let mut next = black_box(make());
            let p = next.step().unwrap();
            let v = next.velocity();
            p.y.to_bits() ^ v.y.to_bits()
        },
        &expected,
        "construct with nine finite input checks then checked Euler; dt=16ms p=(1,10,-2) v=(2,-3,4) a=(0,9.81,0)",
    );
}
fn pagination(iterations: usize) {
    use matea::widgets::{Pagination, Paginator};
    use std::num::NonZeroUsize;
    let mut pages = Pagination::new(101, NonZeroUsize::new(10).unwrap());
    pages.set_page(10);
    assert_eq!(pages.range(), 100..101);
    measure(
        "pagination/range",
        (iterations * 10).max(300000),
        || {
            let range = black_box(pages).range();
            (range.start ^ range.end) as u64
        },
        "100,101",
        "collection length=101 per_page=10 page=10; slice bounds only",
    );
    pages.set_page(4);
    measure(
        "pagination/navigation",
        (iterations * 10).max(300000),
        || {
            let mut p = black_box(pages);
            p.next();
            p.previous();
            p.page() as u64
        },
        "4",
        "next then previous from page=4 total_pages=11 per_page=10; state helpers only",
    );
    let pager = Paginator::new(pages);
    assert_eq!(pager.label(), "5/11");
    measure(
        "pagination/arabic",
        iterations.max(1000),
        || black_box(&pager).label().len() as u64,
        "5/11",
        "Arabic label only current=5 total=11; no view/render/I/O",
    );
    for count in [32, 256] {
        let mut dots = Pagination::from_pages(count, NonZeroUsize::new(1).unwrap());
        dots.set_page(10);
        let expected = dots.dots("•", "○", count);
        measure(
            &format!("pagination/dots_{count}"),
            iterations.max(1000),
            || black_box(dots).dots("•", "○", black_box(count)).len() as u64,
            &expected,
            "dot label only all pages current=10 zero-based; active=• inactive=○; no view/render/I/O",
        );
    }
    let mut pager = Paginator::new(pages);
    let right = Input::Key(Key::new(KeyCode::Right));
    let left = Input::Key(Key::new(KeyCode::Left));
    measure(
        "pagination/key_dispatch",
        (iterations * 10).max(30000),
        || {
            black_box(black_box(&mut pager).event(black_box(&right), true));
            black_box(black_box(&mut pager).event(black_box(&left), true));
            pager.pages.page() as u64
        },
        "4",
        "next/right then previous/left key dispatch; initial page=4 total_pages=11; no view/render/I/O",
    );
    assert_eq!(pager.pages.page(), 4);
}
fn input_bindings(iterations: usize) {
    use matea::widgets::key::Binding;
    let ctrl_x = Key::new(KeyCode::Char('x')).with_modifiers(Modifiers::CONTROL);
    for (name, event, enabled, expected) in [
        ("ctrl_x", ctrl_x, true, 1),
        ("f12", Key::new(KeyCode::F(12)), true, 1),
        ("miss", Key::new(KeyCode::Char('y')), true, 0),
        ("disabled", ctrl_x, false, 0),
        ("repeat", ctrl_x.with_kind(KeyKind::Repeat), true, 1),
        ("release", ctrl_x.with_kind(KeyKind::Release), true, 0),
    ] {
        let mut binding = Binding::new([ctrl_x, Key::new(KeyCode::F(12))], "ctrl+x/F12", "inspect");
        binding.enabled = enabled;
        assert_eq!(u64::from(binding.matches(event)), expected);
        measure(
            &format!("input/binding_{name}"),
            (iterations * 10).max(30000),
            || u64::from(black_box(&binding).matches(black_box(event))),
            &expected.to_string(),
            "two typed/string shortcuts ctrl+x,F12; down-event policy; matching only; no construction or I/O",
        );
    }
}

//! Reproducible cell-rendering probe. Numbers describe this host, not a performance guarantee.
use matea::prelude::*;
use matea::terminal::{Capabilities, write_frame};
use std::time::Instant;
fn main() {
    let view = View::column((0..20).map(|n| View::text(format!("Row {n}: Matea foundation"))));
    let size = Size::new(80, 24);
    let start = Instant::now();
    for _ in 0..1000 {
        std::hint::black_box(view.render(size).unwrap());
    }
    let render_us = start.elapsed().as_micros();
    let frame = view.render(size).unwrap();
    let mut output = Vec::new();
    let caps = Capabilities {
        color: ColorLevel::None,
        unicode: true,
        reduced_motion: true,
    };
    write_frame(&mut output, &frame, None, caps).unwrap();
    let first = output.len();
    output.clear();
    write_frame(&mut output, &frame, Some(&frame), caps).unwrap();
    println!(
        "{{\"iterations\":1000,\"size\":\"80x24\",\"render_total_us\":{},\"first_frame_bytes\":{},\"unchanged_frame_bytes\":{}}}",
        render_us,
        first,
        output.len()
    );
    assert_eq!(output.len(), 0);
}

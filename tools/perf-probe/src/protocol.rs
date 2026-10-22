//! Pure encoders consume already prepared borrowed bytes. No processing/decoding.
use super::{black_box, measure};
use matea::terminal::protocol::{
    IndexedPixels, Limits, PixelFormat, RawPixels, kitty_delete, kitty_place, kitty_query,
    kitty_query_response, kitty_transfer, sixel,
};
use std::num::NonZeroU32;

pub(super) fn costs(iterations: usize) {
    let id = NonZeroU32::new(7).unwrap();
    let limits = Limits::default();
    for (format_name, format, channels) in
        [("rgb", PixelFormat::Rgb, 3), ("rgba", PixelFormat::Rgba, 4)]
    {
        for (width, height) in [(1, 1), (1024, 1), (1025, 1), (64, 32)] {
            let bytes: Vec<u8> = (0..width * height * channels)
                .map(|i| (i * 17 + 3) as u8)
                .collect();
            let pixels = RawPixels {
                width,
                height,
                format,
                bytes: &bytes,
            };
            for display in [false, true] {
                let output = kitty_transfer(pixels, id, display, limits).unwrap();
                measure(
                    &format!("protocol/kitty_{format_name}_{width}x{height}_display_{display}"),
                    (iterations / 20).max(30),
                    || {
                        black_box(
                            kitty_transfer(
                                black_box(pixels),
                                black_box(id),
                                black_box(display),
                                black_box(limits),
                            )
                            .unwrap(),
                        )
                        .len() as u64
                    },
                    std::str::from_utf8(&output).unwrap(),
                    "fresh bounded Kitty packet from borrowed prepared RGB/RGBA; include validation, headers, chunking, base64, allocation and drop; no conversion, terminal I/O or support inference",
                );
            }
        }
    }
    let palette: Vec<[u8; 3]> = (0..16).map(|i| [i * 17, 255 - i * 17, i * 11]).collect();
    for (width, height) in [(1, 1), (64, 7), (80, 24), (257, 6)] {
        for (pattern, transparent) in [("solid", None), ("varied", None), ("varied", Some(3))] {
            let indices: Vec<u8> = (0..width * height)
                .map(|i| {
                    if pattern == "solid" {
                        0
                    } else {
                        (i % 16) as u8
                    }
                })
                .collect();
            let pixels = IndexedPixels {
                width,
                height,
                indices: &indices,
                palette: if pattern == "solid" {
                    &palette[..1]
                } else {
                    &palette
                },
                transparent,
            };
            let output = sixel(pixels, limits).unwrap();
            measure(
                &format!("protocol/sixel_{width}x{height}_{pattern}_transparent_{transparent:?}"),
                (iterations / 20).max(30),
                || black_box(sixel(black_box(pixels), black_box(limits)).unwrap()).len() as u64,
                std::str::from_utf8(&output).unwrap(),
                "fresh bounded Sixel packet from borrowed already indexed bytes and palette; include validation, palette headers, masks, run encoding, allocation and drop; no quantization, conversion or terminal I/O",
            );
        }
    }
    for value in [1, u32::MAX] {
        let id = NonZeroU32::new(value).unwrap();
        for name in ["query", "place", "delete_keep", "delete_free"] {
            let operation = || match name {
                "query" => kitty_query(black_box(id)),
                "place" => kitty_place(
                    black_box(id),
                    black_box(id),
                    black_box(id),
                    black_box(id),
                    black_box(i32::MIN),
                ),
                "delete_keep" => kitty_delete(black_box(id), black_box(false)),
                _ => kitty_delete(black_box(id), black_box(true)),
            };
            let output = operation();
            measure(
                &format!("protocol/kitty_{name}_id_{value}"),
                iterations.max(1000),
                || black_box(operation()).len() as u64,
                std::str::from_utf8(&output).unwrap(),
                "fresh owned Kitty control bytes from typed runtime IDs, i32::MIN z for placement; include decimal formatting, allocation and drop; no terminal I/O or support inference",
            );
        }
    }
    for (name, bytes) in [
        ("ok", b"\x1b_Gi=7;OK\x1b\\".to_vec()),
        ("unsupported", b"\x1b_Gi=7;ENOENT\x1b\\".to_vec()),
        ("wrong_id", b"\x1b_Gi=8;OK\x1b\\".to_vec()),
        ("control", b"\x1b_Gi=7;bad\x1b[31m\x1b\\".to_vec()),
        ("oversize", vec![b'x'; 257]),
    ] {
        let output = kitty_query_response(id, &bytes);
        measure(
            &format!("protocol/kitty_reply_{name}"),
            iterations.max(1000),
            || {
                u64::from(
                    black_box(kitty_query_response(black_box(id), black_box(&bytes))).is_some(),
                )
            },
            &format!("{output:?}"),
            "bounded complete Kitty reply classification for expected ID 7, prebuilt borrowed bytes, no accumulation, terminal read or allocation",
        );
    }
}

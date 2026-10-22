//! Test-only protocol interpreters for the encoder's emitted subset. No terminal
//! support or general image decoder is inferred from an in-memory round trip.
use matea_terminal::protocol::{
    EncodeError, IndexedPixels, Limits, PixelFormat, RawPixels, kitty_transfer, sixel,
};
use std::num::NonZeroU32;

fn unbase64(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut bits = 0u32;
    let mut count = 0;
    for &c in s {
        if c == b'=' {
            break;
        }
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => panic!("invalid base64 byte"),
        };
        bits = (bits << 6) | u32::from(value);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
        }
    }
    out
}

#[test]
fn kitty_transfers_round_trip_formats_chunk_edges_and_exact_limits() {
    for format in [PixelFormat::Rgb, PixelFormat::Rgba] {
        for width in [1, 2, 3, 767, 768, 769, 1023, 1024, 1025, 3073] {
            let channels = if matches!(format, PixelFormat::Rgb) {
                3
            } else {
                4
            };
            let bytes: Vec<_> = (0..width * channels).map(|i| (i * 37) as u8).collect();
            let pixels = RawPixels {
                width,
                height: 1,
                format,
                bytes: &bytes,
            };
            let encoded = kitty_transfer(
                pixels,
                NonZeroU32::new(42).unwrap(),
                false,
                Limits::default(),
            )
            .unwrap();
            let chunks: Vec<_> = encoded
                .split(|&b| b == 0x1b)
                .filter(|s| s.starts_with(b"_G"))
                .collect();
            let mut decoded = Vec::new();
            for (i, chunk) in chunks.iter().enumerate() {
                let split = chunk.iter().position(|&b| b == b';').unwrap();
                let header = std::str::from_utf8(&chunk[..split]).unwrap();
                assert!(header.ends_with(if i + 1 < chunks.len() { "m=1" } else { "m=0" }));
                let payload = &chunk[split + 1..];
                assert!(payload.len() <= 4096 && payload.len() % 4 == 0);
                decoded.extend(unbase64(payload));
            }
            assert_eq!(decoded, bytes);
            for (limit, success) in [(encoded.len(), true), (encoded.len() - 1, false)] {
                let result = kitty_transfer(
                    pixels,
                    NonZeroU32::new(42).unwrap(),
                    false,
                    Limits {
                        max_encoded_bytes: limit,
                        ..Limits::default()
                    },
                );
                assert_eq!(result.is_ok(), success);
                if !success {
                    assert_eq!(result, Err(EncodeError::Limit));
                }
            }
        }
    }
}

fn number(bytes: &[u8], i: &mut usize) -> usize {
    let start = *i;
    while bytes.get(*i).is_some_and(u8::is_ascii_digit) {
        *i += 1;
    }
    assert!(*i > start);
    std::str::from_utf8(&bytes[start..*i])
        .unwrap()
        .parse()
        .unwrap()
}

fn sixel_marks(encoded: &[u8], width: usize, height: usize) -> Vec<Option<u8>> {
    let q = encoded.iter().position(|&c| c == b'q').unwrap();
    let bytes = &encoded[q + 1..encoded.len() - 2];
    assert_eq!(bytes[0], b'"');
    let mut i = 1;
    let mut raster = Vec::new();
    for n in 0..4 {
        raster.push(number(bytes, &mut i));
        if n < 3 {
            assert_eq!(bytes[i], b';');
            i += 1;
        }
    }
    assert_eq!(raster, [1, 1, width, height]);
    let (mut x, mut y, mut color) = (0, 0, 0u8);
    let mut marks = vec![None; width * height];
    while i < bytes.len() {
        let c = bytes[i];
        i += 1;
        match c {
            b'#' => {
                color = number(bytes, &mut i) as u8;
                if bytes.get(i) == Some(&b';') {
                    for _ in 0..4 {
                        i += 1;
                        number(bytes, &mut i);
                    }
                }
            }
            b'$' => x = 0,
            b'-' => {
                x = 0;
                y += 6;
            }
            b'!' | b'?'..=b'~' => {
                let (repeat, mask) = if c == b'!' {
                    let n = number(bytes, &mut i);
                    let mask = bytes[i];
                    i += 1;
                    (n, mask)
                } else {
                    (1, c)
                };
                assert!((b'?'..=b'~').contains(&mask));
                for _ in 0..repeat {
                    assert!(x < width);
                    for dy in 0..6 {
                        if (mask - b'?') & (1 << dy) != 0 {
                            assert!(y + dy < height);
                            marks[(y + dy) * width + x] = Some(color);
                        }
                    }
                    x += 1;
                }
            }
            _ => panic!("unexpected Sixel command {c}"),
        }
    }
    marks
}

#[test]
fn sixel_band_rewinds_runs_and_transparency_preserve_every_pixel() {
    let palette = [
        [0, 0, 0],
        [255, 255, 255],
        [24, 125, 247],
        [80, 90, 100],
        [255, 0, 0],
    ];
    for width in [1usize, 4, 17] {
        for height in [1usize, 5, 6, 7, 12, 13] {
            for transparent in [None, Some(0), Some(4)] {
                let indices: Vec<_> = (0..width * height)
                    .map(|i| ((i / 5 + i / width) % palette.len()) as u8)
                    .collect();
                let pixels = IndexedPixels {
                    width: width as u32,
                    height: height as u32,
                    indices: &indices,
                    palette: &palette,
                    transparent,
                };
                let encoded = sixel(pixels, Limits::default()).unwrap();
                let expected: Vec<_> = indices
                    .iter()
                    .map(|&n| {
                        if Some(n) == transparent {
                            None
                        } else {
                            Some(n)
                        }
                    })
                    .collect();
                assert_eq!(sixel_marks(&encoded, width, height), expected);
                assert_eq!(
                    sixel(
                        pixels,
                        Limits {
                            max_encoded_bytes: encoded.len(),
                            ..Limits::default()
                        }
                    )
                    .unwrap(),
                    encoded
                );
                assert_eq!(
                    sixel(
                        pixels,
                        Limits {
                            max_encoded_bytes: encoded.len() - 1,
                            ..Limits::default()
                        }
                    ),
                    Err(EncodeError::Limit)
                );
            }
        }
    }
}

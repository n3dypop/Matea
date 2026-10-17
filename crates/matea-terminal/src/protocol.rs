//! Pure terminal graphics encoding: no image decoding, conversion or terminal I/O.
//! Specifications and limitations: ADR 0009. Encoding never establishes support.
use std::num::NonZeroU32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    Kitty,
    Sixel,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Support {
    #[default]
    Unknown,
    Supported,
    Unsupported,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphicsSupport {
    pub kitty: Support,
    pub sixel: Support,
}
impl GraphicsSupport {
    /// Unknown environment hints are not confirmation. Callers choose a text
    /// fallback on error or explicitly override the appropriate support field.
    pub fn require(self, protocol: Protocol) -> Result<(), EncodeError> {
        match match protocol {
            Protocol::Kitty => self.kitty,
            Protocol::Sixel => self.sixel,
        } {
            Support::Supported => Ok(()),
            _ => Err(EncodeError::Unsupported),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_dimension: u32,
    pub max_pixels: usize,
    pub max_encoded_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_dimension: 8192,
            max_pixels: 16_777_216,
            max_encoded_bytes: 64 * 1024 * 1024,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    Dimensions,
    PixelLength,
    Palette,
    Limit,
    Unsupported,
}
impl std::fmt::Display for EncodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Dimensions => "pixel dimensions must be positive and within limits",
            Self::PixelLength => "pixel length does not match dimensions and format",
            Self::Palette => "palette/index/transparent color is invalid",
            Self::Limit => "encoded payload exceeds configured limit",
            Self::Unsupported => "terminal protocol support is unknown or unavailable",
        })
    }
}
impl std::error::Error for EncodeError {}
fn area(width: u32, height: u32, limits: Limits) -> Result<usize, EncodeError> {
    if width == 0 || height == 0 || width > limits.max_dimension || height > limits.max_dimension {
        return Err(EncodeError::Dimensions);
    }
    let pixels = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .ok_or(EncodeError::Dimensions)?;
    if pixels > limits.max_pixels {
        return Err(EncodeError::Limit);
    }
    Ok(pixels)
}
struct Packet {
    bytes: Vec<u8>,
    limit: usize,
}

// All numeric protocol headers below are shorter than 128 ASCII bytes even at
// u32/usize maxima. Keep formatting temporary storage bounded and on the stack;
// copy each complete header once so Vec growth does not follow fmt fragments.
struct Header {
    bytes: [u8; 128],
    len: usize,
}
impl std::fmt::Write for Header {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        let end = self.len.checked_add(text.len()).ok_or(std::fmt::Error)?;
        self.bytes
            .get_mut(self.len..end)
            .ok_or(std::fmt::Error)?
            .copy_from_slice(text.as_bytes());
        self.len = end;
        Ok(())
    }
}
impl Packet {
    fn new(limits: Limits) -> Self {
        Self {
            bytes: Vec::new(),
            limit: limits.max_encoded_bytes,
        }
    }
    fn push(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(EncodeError::Limit);
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn formatted(&mut self, arguments: std::fmt::Arguments<'_>) -> Result<(), EncodeError> {
        let mut header = Header {
            bytes: [0; 128],
            len: 0,
        };
        std::fmt::write(&mut header, arguments).map_err(|_| EncodeError::Limit)?;
        self.push(&header.bytes[..header.len])
    }
}
#[derive(Clone, Copy, Debug)]
pub enum PixelFormat {
    Rgb,
    Rgba,
}
impl PixelFormat {
    fn channels(self) -> usize {
        match self {
            Self::Rgb => 3,
            Self::Rgba => 4,
        }
    }
}
/// Borrowed tightly packed row-major sRGB data. RGBA alpha is straight alpha.
#[derive(Clone, Copy, Debug)]
pub struct RawPixels<'a> {
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub bytes: &'a [u8],
}
/// Direct uncompressed Kitty transfer, at most 4096 base64 bytes per chunk.
/// Failure returns no partial packet. Transport must serialize the whole packet.
pub fn kitty_transfer(
    pixels: RawPixels<'_>,
    id: NonZeroU32,
    display: bool,
    limits: Limits,
) -> Result<Vec<u8>, EncodeError> {
    let count = area(pixels.width, pixels.height, limits)?;
    if count.checked_mul(pixels.format.channels()) != Some(pixels.bytes.len()) {
        return Err(EncodeError::PixelLength);
    }
    let mut packet = Packet::new(limits);
    let chunks = pixels.bytes.chunks(3072);
    let total = chunks.len();
    for (index, chunk) in chunks.enumerate() {
        let more = u8::from(index + 1 < total);
        if index == 0 {
            packet.formatted(format_args!(
                "\x1b_Ga={},f={},s={},v={},i={},q=2,m={more};",
                if display { 'T' } else { 't' },
                pixels.format.channels() * 8,
                pixels.width,
                pixels.height,
                id
            ))?;
        } else {
            packet.formatted(format_args!("\x1b_Gm={more};"))?;
        }
        base64(&mut packet, chunk)?;
        packet.push(b"\x1b\\")?;
    }
    Ok(packet.bytes)
}
fn base64(packet: &mut Packet, bytes: &[u8]) -> Result<(), EncodeError> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buffer = [b'='; 4096];
    let mut end = 0;
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        buffer[end] = ALPHABET[usize::from(a >> 2)];
        buffer[end + 1] = ALPHABET[usize::from((a & 3) << 4 | b >> 4)];
        if chunk.len() > 1 {
            buffer[end + 2] = ALPHABET[usize::from((b & 15) << 2 | c >> 6)];
        }
        if chunk.len() > 2 {
            buffer[end + 3] = ALPHABET[usize::from(c & 63)];
        }
        end += 4;
    }
    packet.push(&buffer[..end])
}
/// Query a dummy pixel without storing/displaying it. Transport owns timeout and
/// input routing; a positive matching response confirms Kitty support only.
pub fn kitty_query(id: NonZeroU32) -> Vec<u8> {
    format!("\x1b_Gi={id},s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c").into_bytes()
}
/// Bounded complete reply for this ID. Error text is never re-emitted.
pub fn kitty_query_response(expected: NonZeroU32, bytes: &[u8]) -> Option<Support> {
    if bytes.len() > 256 {
        return None;
    }
    let body = bytes.strip_prefix(b"\x1b_Gi=")?.strip_suffix(b"\x1b\\")?;
    let separator = body.iter().position(|&b| b == b';')?;
    let id = std::str::from_utf8(&body[..separator])
        .ok()?
        .parse::<NonZeroU32>()
        .ok()?;
    if id != expected {
        return None;
    }
    let response = &body[separator + 1..];
    if !response.iter().all(|b| (32..=126).contains(b)) {
        return None;
    }
    Some(if response == b"OK" {
        Support::Supported
    } else {
        Support::Unsupported
    })
}
/// Place at the current cursor, C=1. Reusing the image/placement pair replaces it.
pub fn kitty_place(
    id: NonZeroU32,
    placement: NonZeroU32,
    columns: NonZeroU32,
    rows: NonZeroU32,
    z: i32,
) -> Vec<u8> {
    format!("\x1b_Ga=p,i={id},p={placement},c={columns},r={rows},z={z},C=1,q=2;\x1b\\").into_bytes()
}
/// Scoped deletion, optionally freeing pixels. Never deletes other image IDs.
pub fn kitty_delete(id: NonZeroU32, free_data: bool) -> Vec<u8> {
    format!(
        "\x1b_Ga=d,d={},i={id},q=2;\x1b\\",
        if free_data { 'I' } else { 'i' }
    )
    .into_bytes()
}
/// Already indexed pixels: no quantization or blending. Transparent index leaves
/// existing terminal pixels untouched (Sixel P2=1).
#[derive(Clone, Copy, Debug)]
pub struct IndexedPixels<'a> {
    pub width: u32,
    pub height: u32,
    pub indices: &'a [u8],
    pub palette: &'a [[u8; 3]],
    pub transparent: Option<u8>,
}
pub fn sixel(pixels: IndexedPixels<'_>, limits: Limits) -> Result<Vec<u8>, EncodeError> {
    let count = area(pixels.width, pixels.height, limits)?;
    if count != pixels.indices.len() {
        return Err(EncodeError::PixelLength);
    }
    let colors = pixels.palette.len();
    if colors == 0
        || colors > 256
        || pixels.indices.iter().any(|&i| usize::from(i) >= colors)
        || pixels.transparent.is_some_and(|i| usize::from(i) >= colors)
    {
        return Err(EncodeError::Palette);
    }
    let mut packet = Packet::new(limits);
    packet.formatted(format_args!(
        "\x1bP0;{};0q\"1;1;{};{}",
        u8::from(pixels.transparent.is_some()),
        pixels.width,
        pixels.height
    ))?;
    for (index, rgb) in pixels.palette.iter().enumerate() {
        if pixels.transparent == Some(index as u8) {
            continue;
        }
        let percent = |c: u8| (u32::from(c) * 100 + 127) / 255;
        packet.formatted(format_args!(
            "#{index};2;{};{};{}",
            percent(rgb[0]),
            percent(rgb[1]),
            percent(rgb[2])
        ))?;
    }
    let width = pixels.width as usize;
    let mask_len = width.checked_mul(colors).ok_or(EncodeError::Limit)?;
    if mask_len > limits.max_pixels {
        return Err(EncodeError::Limit);
    }
    let mut masks = vec![0u8; mask_len];
    for band in (0..pixels.height as usize).step_by(6) {
        masks.fill(0);
        for dy in 0..6.min(pixels.height as usize - band) {
            for x in 0..width {
                let index = pixels.indices[(band + dy) * width + x];
                if Some(index) != pixels.transparent {
                    masks[usize::from(index) * width + x] |= 1 << dy;
                }
            }
        }
        let mut painted = false;
        for (index, mask) in masks.chunks(width).enumerate() {
            let Some(last) = mask.iter().rposition(|&m| m != 0) else {
                continue;
            };
            if painted {
                packet.push(b"$")?;
            }
            painted = true;
            packet.formatted(format_args!("#{index}"))?;
            let mut x = 0;
            while x <= last {
                let value = mask[x];
                let start = x;
                while x <= last && mask[x] == value {
                    x += 1;
                }
                let count = x - start;
                if count >= 4 {
                    packet.formatted(format_args!("!{count}"))?;
                    packet.push(&[value + 63])?;
                } else {
                    for _ in 0..count {
                        packet.push(&[value + 63])?;
                    }
                }
            }
        }
        if band + 6 < pixels.height as usize {
            packet.push(b"-")?;
        }
    }
    packet.push(b"\x1b\\")?;
    Ok(packet.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id() -> NonZeroU32 {
        NonZeroU32::new(7).unwrap()
    }
    #[test]
    fn header_formatting_is_bounded_and_failed_headers_are_not_appended() {
        let mut packet = Packet::new(Limits::default());
        let maximum_header = format!(
            "\x1b_Ga=T,f=32,s={},v={},i={},q=2,m=1;",
            u32::MAX,
            u32::MAX,
            u32::MAX
        );
        packet
            .formatted(format_args!(
                "\x1b_Ga=T,f=32,s={},v={},i={},q=2,m=1;",
                u32::MAX,
                u32::MAX,
                u32::MAX
            ))
            .unwrap();
        assert_eq!(packet.bytes, maximum_header.as_bytes());
        packet.formatted(format_args!("!{}", usize::MAX)).unwrap();
        let previous = packet.bytes.clone();
        assert_eq!(
            packet.formatted(format_args!("{}", "x".repeat(129))),
            Err(EncodeError::Limit)
        );
        assert_eq!(packet.bytes, previous);
        packet
            .formatted(format_args!("{}", "x".repeat(128)))
            .unwrap();
        let mut limited = Packet::new(Limits {
            max_encoded_bytes: 2,
            ..Limits::default()
        });
        assert_eq!(
            limited.formatted(format_args!("123")),
            Err(EncodeError::Limit)
        );
        assert!(limited.bytes.is_empty());
    }
    #[test]
    fn kitty_byte_snapshots_and_lifecycle() {
        assert_eq!(
            kitty_transfer(
                RawPixels {
                    width: 1,
                    height: 1,
                    format: PixelFormat::Rgb,
                    bytes: &[255, 0, 1]
                },
                id(),
                false,
                Limits::default()
            )
            .unwrap(),
            b"\x1b_Ga=t,f=24,s=1,v=1,i=7,q=2,m=0;/wAB\x1b\\"
        );
        assert_eq!(kitty_delete(id(), true), b"\x1b_Ga=d,d=I,i=7,q=2;\x1b\\");
        assert_eq!(
            kitty_place(id(), id(), id(), id(), -1),
            b"\x1b_Ga=p,i=7,p=7,c=7,r=7,z=-1,C=1,q=2;\x1b\\"
        );
        assert_eq!(
            kitty_query_response(id(), b"\x1b_Gi=7;OK\x1b\\"),
            Some(Support::Supported)
        );
        assert_eq!(kitty_query_response(id(), b"\x1b_Gi=8;OK\x1b\\"), None);
        assert_eq!(
            kitty_query_response(id(), b"\x1b_Gi=7;bad\x1b[31m\x1b\\"),
            None
        );
        assert!(GraphicsSupport::default().require(Protocol::Kitty).is_err());
    }
    #[test]
    fn kitty_chunk_boundaries_padding_and_limits() {
        let data = vec![0; 4096];
        let pixels = RawPixels {
            width: 1024,
            height: 1,
            format: PixelFormat::Rgba,
            bytes: &data,
        };
        let encoded = kitty_transfer(pixels, id(), true, Limits::default()).unwrap();
        let text = std::str::from_utf8(&encoded).unwrap();
        let chunks = text
            .split("\x1b\\")
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].split_once(';').unwrap().1.len(), 4096);
        assert!(chunks[0].contains("m=1;"));
        assert!(chunks[1].starts_with("\x1b_Gm=0;"));
        assert!(chunks[1].ends_with("=="));
        assert_eq!(
            kitty_transfer(
                pixels,
                id(),
                true,
                Limits {
                    max_encoded_bytes: 20,
                    ..Limits::default()
                }
            ),
            Err(EncodeError::Limit)
        );
    }
    #[test]
    fn sixel_bit_order_bands_transparency_and_runs() {
        let palette = [[255, 0, 0], [0, 255, 0]];
        assert_eq!(
            sixel(
                IndexedPixels {
                    width: 1,
                    height: 7,
                    indices: &[0, 1, 0, 1, 0, 1, 0],
                    palette: &palette,
                    transparent: Some(1)
                },
                Limits::default()
            )
            .unwrap(),
            b"\x1bP0;1;0q\"1;1;1;7#0;2;100;0;0#0T-#0@\x1b\\"
        );
        assert_eq!(
            sixel(
                IndexedPixels {
                    width: 4,
                    height: 1,
                    indices: &[0; 4],
                    palette: &palette[..1],
                    transparent: None
                },
                Limits::default()
            )
            .unwrap(),
            b"\x1bP0;0;0q\"1;1;4;1#0;2;100;0;0#0!4@\x1b\\"
        );
        assert_eq!(
            sixel(
                IndexedPixels {
                    width: 1,
                    height: 1,
                    indices: &[2],
                    palette: &palette,
                    transparent: None
                },
                Limits::default()
            ),
            Err(EncodeError::Palette)
        );
    }
    #[test]
    fn invalid_dimensions_and_lengths_are_rejected() {
        for (width, height, error) in [
            (0, 1, EncodeError::Dimensions),
            (1, 1, EncodeError::PixelLength),
            (u32::MAX, u32::MAX, EncodeError::Dimensions),
        ] {
            assert_eq!(
                kitty_transfer(
                    RawPixels {
                        width,
                        height,
                        format: PixelFormat::Rgb,
                        bytes: &[]
                    },
                    id(),
                    false,
                    Limits::default()
                ),
                Err(error)
            );
        }
    }
}

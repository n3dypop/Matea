//! Typed Kitty wire controls and bounded prepared-payload packets.
//!
//! No image processing, codecs, filesystem access or terminal I/O. See ADR 0075.
//! Fields can have different meanings for different actions; this layer checks
//! wire representation, not image/animation semantics or terminal support.
use super::{EncodeError, Limits, Packet, base64};
use std::{fmt, fmt::Write as _, iter::FusedIterator, num::NonZeroU32, str::FromStr};

const RAW_CHUNK: usize = 3072;
const MAX_HEADER: usize = 512;

macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident { $first:ident = $value:expr, $($variant:ident = $wire:expr),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum $name { #[default] $first = $value, $($variant = $wire),* }
        impl $name {
            fn parse(value: &[u8]) -> Option<Self> {
                match value { [$value] => Some(Self::$first), $([$wire] => Some(Self::$variant),)* _ => None }
            }
        }
    };
}
wire_enum! {
    /// Graphics action; frame composition is performed by the receiving terminal.
    Action { Transmit = b't', TransmitDisplay = b'T', Query = b'q', Put = b'p',
        Delete = b'd', Frame = b'f', Animate = b'a', Compose = b'c' }
}
wire_enum! {
    /// Prepared payload format. Png does not introduce a PNG encoder/decoder.
    Format { Rgba = 32, Rgb = 24, Png = 100 }
}
wire_enum! {
    /// Data location. Encoding a name does not open or validate its OS object.
    Transmission { Direct = b'd', File = b'f', TemporaryFile = b't', SharedMemory = b's' }
}
wire_enum! {
    /// Response policy corresponding to q=0, q=1 and q=2.
    Responses { All = 0, Errors = 1, Suppressed = 2 }
}
wire_enum! {
    /// Selection for a deletion command; freeing data is a separate flag.
    DeleteTarget { AllVisible = b'a', Image = b'i', Number = b'n', Cursor = b'c',
        Frames = b'f', Cell = b'p', CellZ = b'q', Range = b'r', Column = b'x', Row = b'y', Z = b'z' }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Compression {
    #[default]
    None,
    /// Payload is already zlib-compressed; this module does not compress it.
    Zlib,
}

/// Immutable-by-convention wire value; serializing it never installs defaults by
/// mutating the caller. Zero integer fields use the protocol's implicit value.
/// Use the existing raw-pixel helpers for validated pixel dimensions/lengths.
///
/// ```
/// use matea_terminal::protocol::{Limits, kitty::{Action, Control}};
/// let control = Control {
///     action: Action::Put,
///     image_id: std::num::NonZeroU32::new(7),
///     columns: 40,
///     rows: 12,
///     no_cursor_movement: true,
///     ..Control::default()
/// };
/// let bytes = control.encode(&[], Limits::default()).unwrap();
/// assert_eq!(bytes, b"\x1b_Gi=7,C=1,c=40,r=12,a=p;\x1b\\");
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Control {
    pub action: Action,
    pub responses: Responses,
    pub image_id: Option<NonZeroU32>,
    pub placement_id: Option<NonZeroU32>,
    pub image_number: Option<NonZeroU32>,
    pub format: Format,
    pub image_width: u32,
    pub image_height: u32,
    pub compression: Compression,
    pub transmission: Transmission,
    pub size: u32,
    pub offset: u32,
    /// Explicit state of one packet; None allows automatic direct chunking.
    pub more: Option<bool>,
    pub x: u32,
    pub y: u32,
    /// Signed stacking order, or signed frame gap for animation actions.
    pub z: i32,
    pub width: u32,
    pub height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub columns: u32,
    pub rows: u32,
    pub virtual_placement: bool,
    /// C=1: preserve cursor on placement; select overwrite on frame composition.
    pub no_cursor_movement: bool,
    pub parent_id: Option<NonZeroU32>,
    pub parent_placement_id: Option<NonZeroU32>,
    pub delete_target: DeleteTarget,
    pub delete_resources: bool,
    /// Additional primary-spec controls not present in pinned x/ansi Options.
    pub usage_hints: u32,
    pub relative_x: i32,
    pub relative_y: i32,
}

/// Parsing is bounded and transactional. Errors never echo untrusted input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    TooLong,
    Syntax,
    UnknownKey(u8),
    DuplicateKey(u8),
    InvalidValue(u8),
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => f.write_str("Kitty control header exceeds 512 bytes"),
            Self::Syntax => f.write_str("malformed Kitty control header"),
            Self::UnknownKey(key) => write!(f, "unknown Kitty key 0x{key:02x}"),
            Self::DuplicateKey(key) => write!(f, "duplicate Kitty key 0x{key:02x}"),
            Self::InvalidValue(key) => write!(f, "invalid value for Kitty key 0x{key:02x}"),
        }
    }
}
impl std::error::Error for ParseError {}

impl Control {
    /// Parse comma-separated ASCII controls from str/String or byte storage.
    /// Empty input means defaults. Unknown/duplicate keys and invalid values are
    /// errors; no partially updated receiver or truncated integer is returned.
    pub fn parse(input: impl AsRef<[u8]>) -> Result<Self, ParseError> {
        let input = input.as_ref();
        if input.len() > MAX_HEADER {
            return Err(ParseError::TooLong);
        }
        if !input.is_ascii() {
            return Err(ParseError::Syntax);
        }
        let mut result = Self::default();
        if input.is_empty() {
            return Ok(result);
        }
        let mut seen = 0u64;
        for pair in input.split(|&byte| byte == b',') {
            if pair.len() < 3 || pair[1] != b'=' {
                return Err(ParseError::Syntax);
            }
            let key = pair[0];
            let value = &pair[2..];
            let index = match key {
                b'a' => 0,
                b'q' => 1,
                b'i' => 2,
                b'p' => 3,
                b'I' => 4,
                b'f' => 5,
                b's' => 6,
                b'v' => 7,
                b'o' => 8,
                b't' => 9,
                b'S' => 10,
                b'O' => 11,
                b'm' => 12,
                b'x' => 13,
                b'y' => 14,
                b'z' => 15,
                b'w' => 16,
                b'h' => 17,
                b'X' => 18,
                b'Y' => 19,
                b'c' => 20,
                b'r' => 21,
                b'U' => 22,
                b'C' => 23,
                b'P' => 24,
                b'Q' => 25,
                b'd' => 26,
                b'N' => 27,
                b'H' => 28,
                b'V' => 29,
                _ => return Err(ParseError::UnknownKey(key)),
            };
            if seen & (1 << index) != 0 {
                return Err(ParseError::DuplicateKey(key));
            }
            seen |= 1 << index;
            let bad = || ParseError::InvalidValue(key);
            let unsigned = || {
                std::str::from_utf8(value)
                    .map_err(|_| bad())?
                    .parse::<u32>()
                    .map_err(|_| bad())
            };
            let signed = || {
                std::str::from_utf8(value)
                    .map_err(|_| bad())?
                    .parse::<i32>()
                    .map_err(|_| bad())
            };
            let boolean = || match unsigned()? {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(bad()),
            };
            match key {
                b'a' => result.action = Action::parse(value).ok_or_else(bad)?,
                b'q' => {
                    let value = u8::try_from(unsigned()?).map_err(|_| bad())?;
                    result.responses = Responses::parse(&[value]).ok_or_else(bad)?;
                }
                b'i' => result.image_id = NonZeroU32::new(unsigned()?),
                b'p' => result.placement_id = NonZeroU32::new(unsigned()?),
                b'I' => result.image_number = NonZeroU32::new(unsigned()?),
                b'f' => {
                    let value = u8::try_from(unsigned()?).map_err(|_| bad())?;
                    result.format = Format::parse(&[value]).ok_or_else(bad)?;
                }
                b's' => result.image_width = unsigned()?,
                b'v' => result.image_height = unsigned()?,
                b'o' => {
                    if value != b"z" {
                        return Err(bad());
                    }
                    result.compression = Compression::Zlib;
                }
                b't' => result.transmission = Transmission::parse(value).ok_or_else(bad)?,
                b'S' => result.size = unsigned()?,
                b'O' => result.offset = unsigned()?,
                b'm' => result.more = Some(boolean()?),
                b'x' => result.x = unsigned()?,
                b'y' => result.y = unsigned()?,
                b'z' => result.z = signed()?,
                b'w' => result.width = unsigned()?,
                b'h' => result.height = unsigned()?,
                b'X' => result.offset_x = unsigned()?,
                b'Y' => result.offset_y = unsigned()?,
                b'c' => result.columns = unsigned()?,
                b'r' => result.rows = unsigned()?,
                b'U' => result.virtual_placement = boolean()?,
                b'C' => result.no_cursor_movement = boolean()?,
                b'P' => result.parent_id = NonZeroU32::new(unsigned()?),
                b'Q' => result.parent_placement_id = NonZeroU32::new(unsigned()?),
                b'd' => {
                    if value.len() != 1 {
                        return Err(bad());
                    }
                    result.delete_resources = value[0].is_ascii_uppercase();
                    result.delete_target =
                        DeleteTarget::parse(&[value[0].to_ascii_lowercase()]).ok_or_else(bad)?;
                }
                b'N' => result.usage_hints = unsigned()?,
                b'H' => result.relative_x = signed()?,
                b'V' => result.relative_y = signed()?,
                _ => unreachable!("key was checked before matching"),
            }
        }
        Ok(result)
    }

    /// Encode prepared bytes. With more=None, direct payloads are automatically
    /// chunked; an explicit more value describes exactly one packet. Named media
    /// use one complete nonempty, NUL-free name and never access that OS object.
    /// Byte limits and declared data-action dimensions are checked here. Payload
    /// format/length coherence and undeclared geometry (e.g. prepared PNG) belong
    /// to higher-level producers; no decoder inspects the prepared bytes.
    pub fn encode(&self, payload: &[u8], limits: Limits) -> Result<Vec<u8>, CommandError> {
        self.encode_reserved(payload, limits, reserve_packet)
    }
    fn encode_reserved(
        &self,
        payload: &[u8],
        limits: Limits,
        reserve: impl FnOnce(usize) -> Result<Packet, CommandError>,
    ) -> Result<Vec<u8>, CommandError> {
        let plan = Plan::new(*self, payload, limits)?;
        let mut packet = reserve(plan.total)?;
        for index in 0..plan.count {
            emit(&mut packet, plan.control(index), plan.chunk(payload, index))?;
        }
        debug_assert_eq!(packet.bytes.len(), plan.total);
        Ok(packet.bytes)
    }

    /// Preflight the whole byte budget, then yield individual owned packets.
    /// Iterator adapters can wrap each packet (e.g. transport envelopes). An
    /// allocation failure can follow earlier successful packets; after its one
    /// error this iterator is fused. Encoding alone never writes those packets.
    pub fn chunks<'a>(
        &self,
        payload: &'a [u8],
        limits: Limits,
    ) -> Result<Packets<'a>, CommandError> {
        Ok(Packets {
            plan: Plan::new(*self, payload, limits)?,
            payload,
            next: 0,
            failed: false,
        })
    }
}
impl FromStr for Control {
    type Err = ParseError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
impl fmt::Display for Control {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut separator = "";
        macro_rules! field {
            ($key:literal, $value:expr) => {{
                write!(f, "{separator}{}={}", $key, $value)?;
                separator = ",";
            }};
        }
        macro_rules! number {
            ($key:literal, $value:expr) => {
                if $value != 0 {
                    field!($key, $value);
                }
            };
        }
        macro_rules! id {
            ($key:literal, $value:expr) => {
                if let Some(value) = $value {
                    field!($key, value);
                }
            };
        }
        if self.format != Format::Rgba {
            field!("f", self.format as u8);
        }
        number!("q", self.responses as u8);
        id!("i", self.image_id);
        id!("p", self.placement_id);
        id!("I", self.image_number);
        number!("s", self.image_width);
        number!("v", self.image_height);
        if self.transmission != Transmission::Direct {
            field!("t", self.transmission as u8 as char);
        }
        number!("S", self.size);
        number!("O", self.offset);
        if self.compression == Compression::Zlib {
            field!("o", 'z');
        }
        if self.virtual_placement {
            field!("U", 1);
        }
        if self.no_cursor_movement {
            field!("C", 1);
        }
        id!("P", self.parent_id);
        id!("Q", self.parent_placement_id);
        number!("x", self.x);
        number!("y", self.y);
        number!("z", self.z);
        number!("w", self.width);
        number!("h", self.height);
        number!("X", self.offset_x);
        number!("Y", self.offset_y);
        number!("c", self.columns);
        number!("r", self.rows);
        if self.delete_target != DeleteTarget::AllVisible || self.delete_resources {
            let value = self.delete_target as u8;
            field!(
                "d",
                if self.delete_resources {
                    value.to_ascii_uppercase()
                } else {
                    value
                } as char
            );
        }
        if self.action != Action::Transmit {
            field!("a", self.action as u8 as char);
        }
        if let Some(more) = self.more {
            field!("m", u8::from(more));
        }
        number!("N", self.usage_hints);
        number!("H", self.relative_x);
        number!("V", self.relative_y);
        let _ = separator;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandError {
    Limit,
    Name,
    Allocation,
    Encoding(EncodeError),
}
impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Self::Encoding(error) = self {
            return error.fmt(f);
        }
        f.write_str(match self {
            Self::Limit => "Kitty packet or sequence exceeds its byte limit",
            Self::Name => "named Kitty transmission needs a nonempty NUL-free name",
            Self::Allocation => "unable to reserve Kitty output storage",
            Self::Encoding(_) => unreachable!("handled above"),
        })
    }
}
impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encoding(error) => Some(error),
            _ => None,
        }
    }
}
impl From<EncodeError> for CommandError {
    fn from(error: EncodeError) -> Self {
        Self::Encoding(error)
    }
}

struct Header {
    bytes: [u8; MAX_HEADER],
    len: usize,
}
impl fmt::Write for Header {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let end = self.len.checked_add(value.len()).ok_or(fmt::Error)?;
        self.bytes
            .get_mut(self.len..end)
            .ok_or(fmt::Error)?
            .copy_from_slice(value.as_bytes());
        self.len = end;
        Ok(())
    }
}
fn header(control: Control) -> Result<Header, CommandError> {
    let mut output = Header {
        bytes: [0; MAX_HEADER],
        len: 0,
    };
    write!(&mut output, "{control}").map_err(|_| CommandError::Limit)?;
    Ok(output)
}
fn emit(packet: &mut Packet, control: Control, payload: &[u8]) -> Result<(), CommandError> {
    let header = header(control)?;
    packet.push(b"\x1b_G")?;
    packet.push(&header.bytes[..header.len])?;
    packet.push(b";")?;
    base64(packet, payload)?;
    packet.push(b"\x1b\\")?;
    Ok(())
}
fn reserve_packet(size: usize) -> Result<Packet, CommandError> {
    let mut packet = Packet::new(Limits {
        max_encoded_bytes: size,
        ..Limits::default()
    });
    packet
        .bytes
        .try_reserve_exact(size)
        .map_err(|_| CommandError::Allocation)?;
    Ok(packet)
}

#[derive(Debug)]
struct Plan {
    control: Control,
    count: usize,
    total: usize,
}
impl Plan {
    fn new(control: Control, payload: &[u8], limits: Limits) -> Result<Self, CommandError> {
        if matches!(
            control.action,
            Action::Transmit | Action::TransmitDisplay | Action::Query | Action::Frame
        ) {
            if control.image_width > limits.max_dimension
                || control.image_height > limits.max_dimension
            {
                return Err(CommandError::Encoding(EncodeError::Dimensions));
            }
            if control.image_width != 0 && control.image_height != 0 {
                super::area(control.image_width, control.image_height, limits)?;
            }
        }
        let named = control.transmission != Transmission::Direct;
        if named && (payload.is_empty() || payload.contains(&0)) {
            return Err(CommandError::Name);
        }
        let count = if control.more.is_some() || named {
            if payload.len() > RAW_CHUNK {
                return Err(CommandError::Limit);
            }
            1
        } else {
            payload.len().div_ceil(RAW_CHUNK).max(1)
        };
        let mut plan = Self {
            control,
            count,
            total: 0,
        };
        let encoded = payload
            .len()
            .checked_add(2)
            .and_then(|n| n.checked_div(3))
            .and_then(|n| n.checked_mul(4))
            .ok_or(CommandError::Limit)?;
        let first = header(plan.control(0))?.len + 6;
        let rest = if count > 1 {
            header(plan.control(1))?.len + 6
        } else {
            0
        };
        plan.total = (count - 1)
            .checked_mul(rest)
            .and_then(|n| n.checked_add(first))
            .and_then(|n| n.checked_add(encoded))
            .ok_or(CommandError::Limit)?;
        if plan.total > limits.max_encoded_bytes {
            return Err(CommandError::Limit);
        }
        Ok(plan)
    }
    fn control(&self, index: usize) -> Control {
        if self.count == 1 {
            return self.control;
        }
        let mut result = if index == 0 {
            self.control
        } else {
            Control {
                action: if self.control.action == Action::Frame {
                    Action::Frame
                } else {
                    Action::Transmit
                },
                responses: self.control.responses,
                ..Control::default()
            }
        };
        result.more = Some(index + 1 < self.count);
        result
    }
    fn chunk<'a>(&self, payload: &'a [u8], index: usize) -> &'a [u8] {
        let start = index * RAW_CHUNK;
        &payload[start..payload.len().min(start.saturating_add(RAW_CHUNK))]
    }
}

/// Incremental, bounded packet ownership; no terminal or OS resource is owned.
#[derive(Debug)]
pub struct Packets<'a> {
    plan: Plan,
    payload: &'a [u8],
    next: usize,
    failed: bool,
}
impl Packets<'_> {
    fn next_reserved(
        &mut self,
        reserve: impl FnOnce(usize) -> Result<Packet, CommandError>,
    ) -> Option<Result<Vec<u8>, CommandError>> {
        if self.failed || self.next == self.plan.count {
            return None;
        }
        let index = self.next;
        self.next += 1;
        let result = (|| {
            let control = self.plan.control(index);
            let payload = self.plan.chunk(self.payload, index);
            let size = header(control)?.len + 6 + payload.len().div_ceil(3) * 4;
            let mut packet = reserve(size)?;
            emit(&mut packet, control, payload)?;
            Ok(packet.bytes)
        })();
        if result.is_err() {
            self.failed = true;
        }
        Some(result)
    }
}
impl Iterator for Packets<'_> {
    type Item = Result<Vec<u8>, CommandError>;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_reserved(reserve_packet)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = if self.failed {
            0
        } else {
            self.plan.count - self.next
        };
        (usize::from(remaining > 0), Some(remaining))
    }
}
impl FusedIterator for Packets<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reservation_failure_is_atomic_or_fuses_after_the_successful_prefix() {
        let control = Control::default();
        assert_eq!(
            control.encode_reserved(b"data", Limits::default(), |_| Err(
                CommandError::Allocation
            )),
            Err(CommandError::Allocation)
        );
        assert_eq!(
            control.encode_reserved(
                b"data",
                Limits {
                    max_encoded_bytes: 0,
                    ..Limits::default()
                },
                |_| panic!("budget must fail before allocation")
            ),
            Err(CommandError::Limit)
        );
        let data = [0; RAW_CHUNK + 1];
        let mut packets = control.chunks(&data, Limits::default()).unwrap();
        assert!(packets.next().unwrap().is_ok());
        assert_eq!(
            packets.next_reserved(|_| Err(CommandError::Allocation)),
            Some(Err(CommandError::Allocation))
        );
        assert_eq!(packets.size_hint(), (0, Some(0)));
        assert!(
            packets
                .next_reserved(|_| panic!("failed iterator must stay fused"))
                .is_none()
        );
        assert!(packets.next().is_none());
    }
}

use matea_terminal::protocol::{
    EncodeError, Limits,
    kitty::{
        Action, CommandError, Compression, Control, DeleteTarget, Format, ParseError, Responses,
        Transmission,
    },
};
use std::{iter::FusedIterator, num::NonZeroU32};

fn decode(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::new();
    let (mut value, mut bits) = (0u32, 0);
    for &byte in bytes {
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => break,
            _ => panic!("non-base64 payload"),
        };
        value = (value << 6) | u32::from(digit);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((value >> bits) as u8);
            value &= (1 << bits) - 1;
        }
    }
    result
}
fn split(packet: &[u8]) -> (Control, Vec<u8>) {
    let body = packet
        .strip_prefix(b"\x1b_G")
        .unwrap()
        .strip_suffix(b"\x1b\\")
        .unwrap();
    let separator = body.iter().position(|&b| b == b';').unwrap();
    let payload = &body[separator + 1..];
    assert!(payload.len() <= 4096 && payload.len().is_multiple_of(4));
    (Control::parse(&body[..separator]).unwrap(), decode(payload))
}

#[test]
fn every_control_role_round_trips_the_independent_literal_header() {
    let header = "f=24,q=2,i=1,p=2,I=3,s=4,v=5,t=f,S=6,O=7,o=z,U=1,C=1,P=8,Q=9,x=10,y=11,z=-12,w=13,h=14,X=15,Y=16,c=17,r=18,d=I,a=T,m=0,N=19,H=-20,V=21";
    let expected = Control {
        action: Action::TransmitDisplay,
        responses: Responses::Suppressed,
        image_id: NonZeroU32::new(1),
        placement_id: NonZeroU32::new(2),
        image_number: NonZeroU32::new(3),
        format: Format::Rgb,
        image_width: 4,
        image_height: 5,
        transmission: Transmission::File,
        size: 6,
        offset: 7,
        compression: Compression::Zlib,
        virtual_placement: true,
        no_cursor_movement: true,
        parent_id: NonZeroU32::new(8),
        parent_placement_id: NonZeroU32::new(9),
        x: 10,
        y: 11,
        z: -12,
        width: 13,
        height: 14,
        offset_x: 15,
        offset_y: 16,
        columns: 17,
        rows: 18,
        delete_target: DeleteTarget::Image,
        delete_resources: true,
        more: Some(false),
        usage_hints: 19,
        relative_x: -20,
        relative_y: 21,
    };
    assert_eq!(Control::parse(header).unwrap(), expected);
    assert_eq!(header.parse::<Control>().unwrap(), expected);
    assert_eq!(Control::parse(header.as_bytes()).unwrap(), expected);
    assert_eq!(expected.to_string(), header);
    assert_eq!(Control::default().to_string(), "");
    assert_eq!(Control::parse(b"").unwrap(), Control::default());
    assert_eq!(
        Control::default().encode(&[], Limits::default()).unwrap(),
        b"\x1b_G;\x1b\\"
    );
}

#[test]
fn cursor_policy_survives_text_conversion_unlike_the_fixed_reference() {
    // Actual pinned x/ansi MarshalText -> UnmarshalText drops C=1. Preserve the
    // independent literal placement packet and both explicit Boolean values.
    let original = "i=7,C=1,c=40,r=12,a=p";
    let control = Control::parse(original).unwrap();
    assert!(control.no_cursor_movement);
    assert_eq!(control.to_string(), original);
    assert_eq!(
        control.encode(&[], Limits::default()).unwrap(),
        b"\x1b_Gi=7,C=1,c=40,r=12,a=p;\x1b\\"
    );
    assert!(!Control::parse("C=0").unwrap().no_cursor_movement);
    assert_eq!(Control::parse("m=0").unwrap().more, Some(false));
    assert_eq!(Control::parse("m=1").unwrap().more, Some(true));
    assert_eq!(Control::parse("z=-1").unwrap().to_string(), "z=-1");
}

#[test]
fn integer_domains_and_all_wire_byte_choices_are_bounded() {
    for key in [
        "i", "p", "I", "s", "v", "S", "O", "x", "y", "w", "h", "X", "Y", "c", "r", "P", "Q", "N",
    ] {
        for value in [0, 1, u32::MAX] {
            let header = format!("{key}={value}");
            let control = Control::parse(&header).unwrap();
            assert_eq!(
                control.to_string(),
                if value == 0 { String::new() } else { header }
            );
        }
        for value in ["-1", "4294967296", "18446744073709551615"] {
            assert!(matches!(
                Control::parse(format!("{key}={value}")),
                Err(ParseError::InvalidValue(_))
            ));
        }
    }
    for key in ["z", "H", "V"] {
        for value in [i32::MIN, -1, 0, 1, i32::MAX] {
            let header = format!("{key}={value}");
            let control = Control::parse(&header).unwrap();
            assert_eq!(
                control.to_string(),
                if value == 0 { String::new() } else { header }
            );
        }
        for value in ["-2147483649", "2147483648"] {
            assert!(Control::parse(format!("{key}={value}")).is_err());
        }
    }
    for byte in 0..=255 {
        for (key, valid) in [
            (b'a', b"tTqpdfac".as_slice()),
            (b't', b"dfts"),
            (b'd', b"aicnf pqrxyzAICNFPQRXYZ"),
            (b'o', b"z"),
        ] {
            // The space in this independent readable selection is not a token.
            let accepted = byte != b' ' && valid.contains(&byte);
            assert_eq!(
                Control::parse([key, b'=', byte]).is_ok(),
                accepted,
                "{key:02x}={byte:02x}"
            );
        }
    }
    for (key, valid) in [
        ("q", &[0, 1, 2][..]),
        ("f", &[24, 32, 100][..]),
        ("m", &[0, 1][..]),
        ("C", &[0, 1][..]),
        ("U", &[0, 1][..]),
    ] {
        for value in 0..=256 {
            assert_eq!(
                Control::parse(format!("{key}={value}")).is_ok(),
                valid.contains(&value)
            );
        }
    }
}

#[test]
fn malformed_headers_fail_without_partial_state_or_control_echo() {
    for header in [
        "a=", ",i=1", "i=1,", "i=1,,p=2", "ii=1", "i=1=2", "a=tt", "U=2", "C=-1", "q=257", "q=-1",
        "z=1.5", "i= 1",
    ] {
        assert!(Control::parse(header).is_err(), "{header:?}");
    }
    assert_eq!(
        Control::parse("i=1,i=2"),
        Err(ParseError::DuplicateKey(b'i'))
    );
    assert_eq!(Control::parse("i=1,j=2"), Err(ParseError::UnknownKey(b'j')));
    assert_eq!(Control::parse(b"a=\xff"), Err(ParseError::Syntax));
    assert_eq!(Control::parse(vec![b'0'; 513]), Err(ParseError::TooLong));
    for key in 0..=255 {
        for error in [
            ParseError::UnknownKey(key),
            ParseError::DuplicateKey(key),
            ParseError::InvalidValue(key),
        ] {
            assert!(error.to_string().chars().all(|c| !c.is_control()));
        }
    }
    // Every maximal field still fits the encoder's independently stated header
    // budget, and signed extrema must survive parsing without truncation.
    let header = "f=100,q=2,i=4294967295,p=4294967295,I=4294967295,s=4294967295,v=4294967295,t=s,S=4294967295,O=4294967295,o=z,U=1,C=1,P=4294967295,Q=4294967295,x=4294967295,y=4294967295,z=-2147483648,w=4294967295,h=4294967295,X=4294967295,Y=4294967295,c=4294967295,r=4294967295,d=Z,a=T,m=0,N=4294967295,H=-2147483648,V=2147483647";
    assert!(header.len() <= 512);
    assert_eq!(Control::parse(header).unwrap().to_string(), header);
}

#[test]
fn packet_boundaries_budgets_and_frame_continuations_match_the_wire_contract() {
    for len in [0usize, 1, 2, 3, 3071, 3072, 3073, 6144, 6145] {
        let payload: Vec<_> = (0..len).map(|n| (n % 256) as u8).collect();
        let control = Control {
            action: Action::Frame,
            responses: Responses::Suppressed,
            image_id: NonZeroU32::new(7),
            image_width: 4,
            image_height: 5,
            ..Control::default()
        };
        let combined = control.encode(&payload, Limits::default()).unwrap();
        let mut iterator = control.chunks(&payload, Limits::default()).unwrap();
        fn fused(_: &impl FusedIterator) {}
        fused(&iterator);
        let count = len.div_ceil(3072).max(1);
        assert_eq!(iterator.size_hint(), (1, Some(count)));
        let mut rebuilt = Vec::new();
        let mut frames = Vec::new();
        let mut decoded = Vec::new();
        for packet in iterator.by_ref() {
            let packet = packet.unwrap();
            let (header, bytes) = split(&packet);
            frames.push(header);
            decoded.extend(bytes);
            rebuilt.extend(packet);
        }
        assert_eq!(rebuilt, combined);
        assert_eq!(decoded, payload);
        assert_eq!(frames.len(), count);
        for (index, header) in frames.iter().enumerate() {
            assert_eq!(header.action, Action::Frame);
            assert_eq!(header.responses, Responses::Suppressed);
            assert_eq!(
                header.more,
                if count == 1 {
                    None
                } else {
                    Some(index + 1 < count)
                }
            );
            if index == 0 {
                assert_eq!(header.image_id, NonZeroU32::new(7));
            } else {
                assert_eq!(header.image_id, None);
                assert_eq!(header.image_width, 0);
                assert_eq!(header.image_height, 0);
            }
        }
        assert_eq!(iterator.size_hint(), (0, Some(0)));
        assert!(iterator.next().is_none());
        assert!(iterator.next().is_none());
        let exact = Limits {
            max_encoded_bytes: combined.len(),
            ..Limits::default()
        };
        assert_eq!(control.encode(&payload, exact).unwrap(), combined);
        let small = Limits {
            max_encoded_bytes: combined.len() - 1,
            ..exact
        };
        assert_eq!(control.encode(&payload, small), Err(CommandError::Limit));
        assert!(matches!(
            control.chunks(&payload, small),
            Err(CommandError::Limit)
        ));
    }
}

#[test]
fn explicit_chunks_named_payloads_and_declared_geometry_are_checked() {
    let more = Control {
        more: Some(false),
        ..Control::default()
    };
    assert_eq!(
        more.encode(&[0, 1, 2, 3], Limits::default()).unwrap(),
        b"\x1b_Gm=0;AAECAw==\x1b\\"
    );
    assert_eq!(
        more.encode(&vec![0; 3073], Limits::default()),
        Err(CommandError::Limit)
    );
    for transmission in [
        Transmission::File,
        Transmission::TemporaryFile,
        Transmission::SharedMemory,
    ] {
        let control = Control {
            transmission,
            format: Format::Png,
            compression: Compression::Zlib,
            size: 64,
            ..Control::default()
        };
        let packet = control
            .encode(b"/prepared-name", Limits::default())
            .unwrap();
        let (decoded, bytes) = split(&packet);
        assert_eq!(decoded, control);
        assert_eq!(bytes, b"/prepared-name");
        assert_eq!(
            control.encode(b"", Limits::default()),
            Err(CommandError::Name)
        );
        assert_eq!(
            control.encode(b"name\0suffix", Limits::default()),
            Err(CommandError::Name)
        );
        assert_eq!(
            control.encode(&vec![b'x'; 3073], Limits::default()),
            Err(CommandError::Limit)
        );
    }
    let control = Control {
        image_width: 10,
        image_height: 20,
        ..Control::default()
    };
    assert_eq!(
        control.encode(
            b"opaque",
            Limits {
                max_dimension: 9,
                ..Limits::default()
            }
        ),
        Err(CommandError::Encoding(EncodeError::Dimensions))
    );
    assert_eq!(
        control.encode(
            b"opaque",
            Limits {
                max_pixels: 199,
                ..Limits::default()
            }
        ),
        Err(CommandError::Encoding(EncodeError::Limit))
    );
    // s/v have animation meanings; a generic pixel-area limit must not be
    // incorrectly imposed on an animation-control command.
    let animate = Control {
        action: Action::Animate,
        image_width: 3,
        image_height: u32::MAX,
        ..Control::default()
    };
    assert!(animate.encode(&[], Limits::default()).is_ok());
}

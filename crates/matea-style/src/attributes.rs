//! Resolved SGR requests; terminal font/decoration support is not universal.
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextAttribute {
    Bold,
    Dim,
    Italic,
    Reverse,
    Hidden,
    Strikethrough,
    Fraktur,
    Overline,
}
impl TextAttribute {
    pub const ALL: [Self; 8] = [
        Self::Bold,
        Self::Dim,
        Self::Italic,
        Self::Reverse,
        Self::Hidden,
        Self::Strikethrough,
        Self::Fraktur,
        Self::Overline,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}
impl UnderlineStyle {
    pub const ALL: [Self; 6] = [
        Self::None,
        Self::Single,
        Self::Double,
        Self::Curly,
        Self::Dotted,
        Self::Dashed,
    ];
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BlinkStyle {
    #[default]
    None,
    Slow,
    Rapid,
}
impl BlinkStyle {
    pub const ALL: [Self; 3] = [Self::None, Self::Slow, Self::Rapid];
}
/// Font-level framing, independent of View panels/layout borders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextFrame {
    #[default]
    None,
    Framed,
    Encircled,
}
impl TextFrame {
    pub const ALL: [Self; 3] = [Self::None, Self::Framed, Self::Encircled];
}

/// An owned, resolved attribute set. Numeric invalid choice combinations have
/// no safe constructor. Storage is private and is not a public ABI contract.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct TextAttributes {
    bits: u16,
}
impl TextAttributes {
    #[inline]
    pub const fn new() -> Self {
        Self { bits: 0 }
    }
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.bits == 0
    }
    #[inline]
    pub const fn contains(self, attribute: TextAttribute) -> bool {
        self.bits & (1 << attribute as u8) != 0
    }
    #[inline]
    pub const fn set(mut self, attribute: TextAttribute, enabled: bool) -> Self {
        let bit = 1 << attribute as u8;
        if enabled {
            self.bits |= bit;
        } else {
            self.bits &= !bit;
        }
        self
    }
    #[inline]
    pub const fn with(self, attribute: TextAttribute) -> Self {
        self.set(attribute, true)
    }
    #[inline]
    pub const fn without(self, attribute: TextAttribute) -> Self {
        self.set(attribute, false)
    }
    #[inline]
    pub const fn with_underline(mut self, style: UnderlineStyle) -> Self {
        self.bits = (self.bits & !(7 << 8)) | ((style as u16) << 8);
        self
    }
    #[inline]
    pub const fn with_blink(mut self, style: BlinkStyle) -> Self {
        self.bits = (self.bits & !(3 << 11)) | ((style as u16) << 11);
        self
    }
    #[inline]
    pub const fn with_frame(mut self, style: TextFrame) -> Self {
        self.bits = (self.bits & !(3 << 13)) | ((style as u16) << 13);
        self
    }
    #[inline]
    pub const fn underline(self) -> UnderlineStyle {
        match (self.bits >> 8) & 7 {
            0 => UnderlineStyle::None,
            1 => UnderlineStyle::Single,
            2 => UnderlineStyle::Double,
            3 => UnderlineStyle::Curly,
            4 => UnderlineStyle::Dotted,
            5 => UnderlineStyle::Dashed,
            _ => panic!("private underline invariant"),
        }
    }
    #[inline]
    pub const fn blink(self) -> BlinkStyle {
        match (self.bits >> 11) & 3 {
            0 => BlinkStyle::None,
            1 => BlinkStyle::Slow,
            2 => BlinkStyle::Rapid,
            _ => panic!("private blink invariant"),
        }
    }
    #[inline]
    pub const fn frame(self) -> TextFrame {
        match (self.bits >> 13) & 3 {
            0 => TextFrame::None,
            1 => TextFrame::Framed,
            2 => TextFrame::Encircled,
            _ => panic!("private frame invariant"),
        }
    }
}
impl fmt::Debug for TextAttributes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Flags(TextAttributes);
        impl fmt::Debug for Flags {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_set()
                    .entries(
                        TextAttribute::ALL
                            .into_iter()
                            .filter(|flag| self.0.contains(*flag)),
                    )
                    .finish()
            }
        }
        f.debug_struct("TextAttributes")
            .field("flags", &Flags(*self))
            .field("underline", &self.underline())
            .field("blink", &self.blink())
            .field("frame", &self.frame())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_typed_states_preserve_independent_choices_and_clear_back_to_default() {
        let masks: Vec<u16> = if cfg!(miri) {
            vec![0, 1, 85, 170, 255]
        } else {
            (0..=255).collect()
        };
        for mask in masks {
            for underline in UnderlineStyle::ALL {
                for blink in BlinkStyle::ALL {
                    for frame in TextFrame::ALL {
                        let flags: Vec<_> = TextAttribute::ALL
                            .into_iter()
                            .enumerate()
                            .filter(|(index, _)| mask & (1 << index) != 0)
                            .map(|(_, flag)| flag)
                            .collect();
                        let mut attributes = TextAttributes::new();
                        for flag in &flags {
                            attributes = attributes.with(*flag);
                        }
                        let a = attributes
                            .with_underline(underline)
                            .with_blink(blink)
                            .with_frame(frame);
                        let b = TextAttributes::new()
                            .with_frame(frame)
                            .with_underline(underline)
                            .with_blink(blink);
                        let b = flags.iter().fold(b, |a, flag| a.with(*flag));
                        assert_eq!(a, b);
                        assert_eq!(
                            (a.underline(), a.blink(), a.frame()),
                            (underline, blink, frame)
                        );
                        for flag in TextAttribute::ALL {
                            assert_eq!(a.contains(flag), flags.contains(&flag));
                        }
                        let mut cleared = a
                            .with_underline(UnderlineStyle::None)
                            .with_blink(BlinkStyle::None)
                            .with_frame(TextFrame::None);
                        for flag in TextAttribute::ALL {
                            cleared = cleared.without(flag);
                        }
                        assert_eq!(cleared, TextAttributes::default());
                        assert!(cleared.is_empty());
                        assert_eq!(
                            a.with_frame(frame)
                                .with_blink(blink)
                                .with_underline(underline),
                            a
                        );
                    }
                }
            }
        }
    }
}

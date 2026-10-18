use crate::{is_safe_text, write_sanitized};
use compact_str::CompactString;
use std::{
    borrow::{Borrow, Cow},
    fmt::{self, Write},
    ops::Deref,
};

/// Owned, immutable display text. Tabs become four spaces and C0/C1 controls
/// become U+FFFD; newlines remain structural. Short values can be stored inline.
/// Formatting Arguments are consumed immediately, retaining no caller borrow.
/// As with format!, a failing user Display implementation panics.
///
/// ```compile_fail
/// let mut text = matea_render::Text::from("tea");
/// text.push_str("\u{1b}"); // normalized content has no mutable string accessor
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Text(CompactString);
impl Text {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_string(self) -> String {
        self.0.into()
    }
    // Only builders that already ran the shared sanitizer can use this path.
    // This is a display invariant, not a memory-safety precondition.
    pub(crate) fn from_normalized(text: CompactString) -> Self {
        debug_assert!(is_safe_text(&text));
        Self(text)
    }
    fn normalized(raw: &str) -> Self {
        let mut text = CompactString::default();
        write_sanitized(raw, &mut text).expect("writing normalized text cannot fail");
        Self(text)
    }
}
impl Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl Borrow<str> for Text {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}
impl From<&str> for Text {
    fn from(raw: &str) -> Self {
        if is_safe_text(raw) {
            Self(CompactString::from(raw))
        } else {
            Self::normalized(raw)
        }
    }
}
impl From<String> for Text {
    fn from(raw: String) -> Self {
        if is_safe_text(&raw) {
            Self(CompactString::from(raw))
        } else {
            Self::normalized(&raw)
        }
    }
}
impl From<CompactString> for Text {
    fn from(raw: CompactString) -> Self {
        if is_safe_text(&raw) {
            Self(raw)
        } else {
            Self::normalized(&raw)
        }
    }
}
impl From<fmt::Arguments<'_>> for Text {
    fn from(args: fmt::Arguments<'_>) -> Self {
        let mut raw = CompactString::default();
        raw.write_fmt(args).expect("formatting Text failed");
        Self::from(raw)
    }
}
impl From<char> for Text {
    fn from(value: char) -> Self {
        let mut raw = CompactString::default();
        raw.push(value);
        Self::from(raw)
    }
}
impl From<&String> for Text {
    fn from(raw: &String) -> Self {
        Self::from(raw.as_str())
    }
}
impl From<&mut String> for Text {
    fn from(raw: &mut String) -> Self {
        Self::from(raw.as_str())
    }
}
impl From<&mut str> for Text {
    fn from(raw: &mut str) -> Self {
        Self::from(&*raw)
    }
}
impl From<&CompactString> for Text {
    fn from(raw: &CompactString) -> Self {
        Self::from(raw.as_str())
    }
}
impl From<&Text> for Text {
    fn from(text: &Text) -> Self {
        text.clone()
    }
}
impl From<Box<str>> for Text {
    fn from(raw: Box<str>) -> Self {
        Self::from(raw.into_string())
    }
}
impl From<Cow<'_, str>> for Text {
    fn from(raw: Cow<'_, str>) -> Self {
        match raw {
            Cow::Borrowed(s) => Self::from(s),
            Cow::Owned(s) => Self::from(s),
        }
    }
}
impl From<Text> for String {
    fn from(text: Text) -> Self {
        text.into_string()
    }
}

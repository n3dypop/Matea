//! Source declaration inventory; macro/cfg resolution is deliberately not claimed.
use quote::ToTokens;
use std::fmt::Write as _;
use std::{
    fmt, fs,
    io::{self, Write},
    path::Path,
};
use syn::{Fields, ImplItem, Item, TraitItem, Visibility, spanned::Spanned};

struct Json<'a>(&'a str);
impl fmt::Display for Json<'_> {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str("\"")?;
        let mut start = 0;
        for (index, c) in self.0.char_indices() {
            let escaped = match c {
                '"' => "\\\"",
                '\\' => "\\\\",
                '\n' => "\\n",
                '\r' => "\\r",
                '\t' => "\\t",
                c if c.is_control() => "",
                _ => continue,
            };
            out.write_str(&self.0[start..index])?;
            if escaped.is_empty() {
                write!(out, "\\u{:04x}", c as u32)?;
            } else {
                out.write_str(escaped)?;
            }
            start = index + c.len_utf8();
        }
        out.write_str(&self.0[start..])?;
        out.write_str("\"")
    }
}
fn emit(
    out: &mut impl Write,
    file: &str,
    line: usize,
    kind: &str,
    symbol: &str,
    cfg: &str,
) -> io::Result<()> {
    writeln!(
        out,
        "{{\"file\":{},\"line\":{line},\"kind\":{},\"symbol\":{},\"cfg\":{}}}",
        Json(file),
        Json(kind),
        Json(symbol),
        Json(cfg)
    )
}
fn public(v: &Visibility) -> bool {
    matches!(v, Visibility::Public(_))
}
fn attrs(attrs: &[syn::Attribute]) -> String {
    let mut output = String::new();
    for attr in attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
    {
        if !output.is_empty() {
            output.push(' ');
        }
        if let syn::Meta::List(list) = &attr.meta
            && matches!(list.delimiter, syn::MacroDelimiter::Paren(_))
        {
            // Preserve the pinned TokenStream spelling while borrowing the
            // existing list tokens. Reconstructing #/[...]/(...) duplicates the
            // complete group and then allocates another Vec and joined String.
            write!(
                output,
                "# {}[{} ({})]",
                if matches!(attr.style, syn::AttrStyle::Inner(_)) {
                    "! "
                } else {
                    ""
                },
                list.path.to_token_stream(),
                list.tokens
            )
            .expect("String formatting cannot fail");
        } else {
            // Lexical inventory also accepts syntactically valid path/value
            // attributes even when cfg evaluation would reject their meaning.
            write!(output, "{}", attr.to_token_stream()).expect("String formatting cannot fail");
        }
    }
    output
}
fn conditions(parent: &str, own: &[syn::Attribute]) -> String {
    let own = attrs(own);
    if own.is_empty() {
        parent.to_owned()
    } else {
        format!("{parent} {own}")
    }
}
fn fields(
    out: &mut impl Write,
    file: &str,
    name: &str,
    fields: &Fields,
    cfg: &str,
    all: bool,
) -> io::Result<()> {
    for (i, f) in fields.iter().enumerate() {
        if all || public(&f.vis) {
            let n = f
                .ident
                .as_ref()
                .map_or_else(|| i.to_string(), ToString::to_string);
            emit(
                out,
                file,
                f.span().start().line,
                "field",
                &format!("{name}.{n}"),
                &conditions(cfg, &f.attrs),
            )?;
        }
    }
    Ok(())
}
fn items(
    out: &mut impl Write,
    file: &str,
    declarations: &[Item],
    prefix: &str,
    parent_cfg: &str,
) -> io::Result<()> {
    for item in declarations {
        let line = item.span().start().line;
        match item {
            Item::Mod(v) => {
                let cfg = format!("{parent_cfg} {}", attrs(&v.attrs));
                if v.ident == "tests" || cfg.contains("cfg (test)") {
                    continue;
                }
                let name = format!("{prefix}{}", v.ident);
                if public(&v.vis) {
                    emit(out, file, line, "module", &name, &cfg)?;
                }
                if let Some((_, inner)) = &v.content {
                    items(out, file, inner, &format!("{name}::"), &cfg)?;
                }
            }
            Item::Impl(v) => {
                let cfg = format!("{parent_cfg} {}", attrs(&v.attrs));
                let ty = v.self_ty.to_token_stream().to_string();
                let tr = v
                    .trait_
                    .as_ref()
                    .map(|(_, p, _)| format!(" as {}", p.to_token_stream()));
                for it in &v.items {
                    let (vis, name, kind, member_attrs) = match it {
                        ImplItem::Fn(m) => (&m.vis, m.sig.ident.to_string(), "method", &m.attrs),
                        ImplItem::Const(c) => {
                            (&c.vis, c.ident.to_string(), "associated_const", &c.attrs)
                        }
                        ImplItem::Type(t) => {
                            (&t.vis, t.ident.to_string(), "associated_type", &t.attrs)
                        }
                        ImplItem::Macro(m) => {
                            emit(
                                out,
                                file,
                                m.span().start().line,
                                "impl_macro",
                                &m.mac.path.to_token_stream().to_string(),
                                &conditions(&cfg, &m.attrs),
                            )?;
                            continue;
                        }
                        _ => continue,
                    };
                    if public(vis) || tr.is_some() {
                        emit(
                            out,
                            file,
                            it.span().start().line,
                            kind,
                            &format!("{prefix}<{ty}{}>::{name}", tr.as_deref().unwrap_or("")),
                            &conditions(&cfg, member_attrs),
                        )?;
                    }
                }
            }
            Item::Struct(v) if public(&v.vis) => {
                let cfg = format!("{parent_cfg} {}", attrs(&v.attrs));
                let name = format!("{prefix}{}", v.ident);
                emit(out, file, line, "struct", &name, &cfg)?;
                fields(out, file, &name, &v.fields, &cfg, false)?;
            }
            Item::Enum(v) if public(&v.vis) => {
                let cfg = format!("{parent_cfg} {}", attrs(&v.attrs));
                let name = format!("{prefix}{}", v.ident);
                emit(out, file, line, "enum", &name, &cfg)?;
                for variant in &v.variants {
                    let cfg = conditions(&cfg, &variant.attrs);
                    let n = format!("{name}::{}", variant.ident);
                    emit(out, file, variant.span().start().line, "variant", &n, &cfg)?;
                    fields(out, file, &n, &variant.fields, &cfg, true)?;
                }
            }
            Item::Trait(v) if public(&v.vis) => {
                let cfg = format!("{parent_cfg} {}", attrs(&v.attrs));
                let name = format!("{prefix}{}", v.ident);
                emit(out, file, line, "trait", &name, &cfg)?;
                for it in &v.items {
                    let (id, member_attrs) = match it {
                        TraitItem::Fn(f) => (f.sig.ident.to_string(), &f.attrs),
                        TraitItem::Type(t) => (t.ident.to_string(), &t.attrs),
                        TraitItem::Const(c) => (c.ident.to_string(), &c.attrs),
                        _ => continue,
                    };
                    emit(
                        out,
                        file,
                        it.span().start().line,
                        "trait_member",
                        &format!("{name}::{id}"),
                        &conditions(&cfg, member_attrs),
                    )?;
                }
            }
            Item::Fn(v) if public(&v.vis) => emit(
                out,
                file,
                line,
                "function",
                &format!("{prefix}{}", v.sig.ident),
                &format!("{parent_cfg} {}", attrs(&v.attrs)),
            )?,
            Item::Const(v) if public(&v.vis) => emit(
                out,
                file,
                line,
                "const",
                &format!("{prefix}{}", v.ident),
                &conditions(parent_cfg, &v.attrs),
            )?,
            Item::Static(v) if public(&v.vis) => emit(
                out,
                file,
                line,
                "static",
                &format!("{prefix}{}", v.ident),
                &conditions(parent_cfg, &v.attrs),
            )?,
            Item::Type(v) if public(&v.vis) => emit(
                out,
                file,
                line,
                "type",
                &format!("{prefix}{}", v.ident),
                &conditions(parent_cfg, &v.attrs),
            )?,
            Item::Use(v) if public(&v.vis) => emit(
                out,
                file,
                line,
                "reexport",
                &v.tree.to_token_stream().to_string(),
                &conditions(parent_cfg, &v.attrs),
            )?,
            Item::Macro(v) => {
                let name = v.ident.as_ref().map_or_else(
                    || v.mac.path.to_token_stream().to_string(),
                    ToString::to_string,
                );
                emit(
                    out,
                    file,
                    line,
                    "macro_or_invocation",
                    &name,
                    &format!("{parent_cfg} {}", attrs(&v.attrs)),
                )?;
            }
            _ => {}
        }
    }
    Ok(())
}
/// Parse and stream one UTF-8 source file. The caller owns buffering/flush.
/// Macro expansion and evaluated cfg/public export resolution are not provided.
pub fn audit_source(
    out: &mut impl Write,
    file: &str,
    source: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use syn::parse::Parser;
    let clean = source.strip_prefix('\u{feff}').unwrap_or(source);
    // Keep syn's complete shebang/comment disambiguation for uncommon prefixes.
    // Ordinary files and compact inner attributes use the bounded AST path.
    if clean.starts_with("#!") && !clean.starts_with("#![") {
        let parsed = syn::parse_file(source)?;
        items(out, file, &parsed.items, "", &attrs(&parsed.attrs))?;
        return Ok(());
    }
    let mut bytes = Vec::new();
    (|input: syn::parse::ParseStream<'_>| {
        let inner = input.call(syn::Attribute::parse_inner)?;
        stream_items(input, &mut bytes, file, "", &attrs(&inner), false)
    })
    .parse_str(clean)?;
    // A later parse error must not publish an earlier declaration from this
    // file. Buffer bytes rather than retaining its complete AST.
    out.write_all(&bytes)?;
    Ok(())
}

fn module_ahead(input: syn::parse::ParseStream<'_>) -> bool {
    use proc_macro2::Delimiter;
    let mut cursor = input.cursor();
    while let Some((punct, after)) = cursor.punct() {
        if punct.as_char() != '#' {
            break;
        }
        let Some((_, _, rest)) = after.group(Delimiter::Bracket) else {
            return false;
        };
        cursor = rest;
    }
    let Some((mut word, mut rest)) = cursor.ident() else {
        return false;
    };
    if word == "pub" {
        if let Some((_, _, after)) = rest.group(Delimiter::Parenthesis) {
            rest = after;
        }
        let Some((next, after)) = rest.ident() else {
            return false;
        };
        word = next;
        rest = after;
    }
    if word == "unsafe" {
        let Some((next, _)) = rest.ident() else {
            return false;
        };
        word = next;
    }
    word == "mod"
}

fn stream_items(
    input: syn::parse::ParseStream<'_>,
    out: &mut Vec<u8>,
    file: &str,
    prefix: &str,
    parent: &str,
    ignored: bool,
) -> syn::Result<()> {
    while !input.is_empty() {
        if !module_ahead(input) {
            let item: Item = input.parse()?;
            if !ignored {
                items(out, file, std::slice::from_ref(&item), prefix, parent)
                    .expect("Vec output cannot fail");
            }
            continue;
        }
        let line = input.span().start().line;
        let mut attributes = input.call(syn::Attribute::parse_outer)?;
        let visibility: Visibility = input.parse()?;
        let _: Option<syn::Token![unsafe]> = input.parse()?;
        let _: syn::Token![mod] = input.parse()?;
        let ident: syn::Ident = input.parse()?;
        let name = format!("{prefix}{ident}");
        if input.peek(syn::Token![;]) {
            let _: syn::Token![;] = input.parse()?;
            let cfg = format!("{parent} {}", attrs(&attributes));
            if !ignored && ident != "tests" && !cfg.contains("cfg (test)") && public(&visibility) {
                emit(out, file, line, "module", &name, &cfg).expect("Vec output cannot fail");
            }
        } else {
            let content;
            syn::braced!(content in input);
            attributes.extend(content.call(syn::Attribute::parse_inner)?);
            let cfg = format!("{parent} {}", attrs(&attributes));
            let ignored = ignored || ident == "tests" || cfg.contains("cfg (test)");
            if !ignored && public(&visibility) {
                emit(out, file, line, "module", &name, &cfg).expect("Vec output cannot fail");
            }
            stream_items(&content, out, file, &format!("{name}::"), &cfg, ignored)?;
        }
    }
    Ok(())
}
fn walk(
    out: &mut impl Write,
    root: &Path,
    kind: fs::FileType,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending = vec![(root.to_owned(), kind)];
    while let Some((path, kind)) = pending.pop() {
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            let mut entries = Vec::new();
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                let path = entry.path();
                if [
                    ".git", "target", "examples", "tests", "benches", "testdata", "xtask",
                    ".github",
                ]
                .contains(&path.file_name().and_then(|n| n.to_str()).unwrap_or(""))
                {
                    continue;
                }
                // file_type never follows links and commonly uses cached
                // directory-entry data. Keep no DirEntry/open-directory handle
                // in the DFS stack; the fixed-tree contract still applies.
                entries.push((path, entry.file_type()?));
            }
            // A directory cannot contain duplicate entry names, so unstable
            // sorting retains deterministic path order without scratch storage.
            entries.sort_unstable_by(|a, b| a.0.cmp(&b.0));
            pending.extend(entries.into_iter().rev());
        } else if kind.is_file() && path.extension().is_some_and(|e| e == "rs") {
            let relative = path.strip_prefix(root)?;
            let mut file = String::with_capacity(relative.as_os_str().len());
            for part in relative.components() {
                if !file.is_empty() {
                    file.push('/');
                }
                file.push_str(part.as_os_str().to_str().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "source path is not UTF-8")
                })?);
            }
            if file.split('/').any(|p| p == "tests.rs") {
                continue;
            }
            audit_source(out, &file, &fs::read_to_string(path)?)?;
        }
    }
    Ok(())
}
/// Inventory a fixed directory tree in deterministic filename order. Directory
/// traversal is iterative; symlink entries are skipped and a linked/non-directory
/// root is rejected. This is not an adversarial concurrent filesystem sandbox.
pub fn audit(out: &mut impl Write, root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "source root must be a real directory",
        )
        .into());
    }
    walk(out, root, metadata.file_type())?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incremental_items_match_whole_file_parser_including_module_boundaries() {
        let fixtures = [
            "",
            "\u{feff}pub const BOM: u8 = 0;",
            "#!/usr/bin/env rust-script\npub const SCRIPT: u8 = 0;",
            "#! /* inner attribute, not a shebang */ [allow(unused)]\npub struct Tea;",
            r#"#![cfg(any(unix, windows))]
                /// A module doc comment.
                #[cfg(feature = "tea")]
                pub mod outer {
                    #![cfg_attr(unix, allow(unused))]
                    pub(in crate::outer) mod private { pub const V: u8 = 0; }
                    pub(crate) mod restricted;
                    pub unsafe mod external;
                    mod hidden { pub struct S { pub field: u8 } }
                    pub mod empty {}
                    pub mod r#type { pub enum E { V { value: u8 } } }
                    pub mod tests { pub const NOT_EMITTED: u8 = 0; }
                    #[cfg(test)] pub mod also_ignored { pub fn hidden() {} }
                    #[cfg(any(test, unix))] pub mod retained { pub trait T { fn run(); } }
                    pub mod inner { #![cfg(test)] pub const HIDDEN: u8 = 0; }
                }
                pub fn f() { mod local { pub const LOCAL: u8 = 0; } }
                macro_rules! r#mod { () => {} }
                r#mod!();
                pub use outer::*;
                impl outer::hidden::S { pub fn call(&self) {} }
            "#,
            "#[cfg[unix]] pub mod m { #[cfg{unix}] pub const C: u8 = 0; }",
        ];
        for source in fixtures {
            let parsed = syn::parse_file(source).unwrap();
            let mut expected = Vec::new();
            items(
                &mut expected,
                "抹茶.rs",
                &parsed.items,
                "",
                &attrs(&parsed.attrs),
            )
            .unwrap();
            let mut actual = Vec::new();
            audit_source(&mut actual, "抹茶.rs", source).unwrap();
            assert_eq!(actual, expected, "{source}");
        }
    }

    #[test]
    fn later_syntax_errors_never_publish_partial_file_declarations() {
        for source in [
            "pub const GOOD: u8 = 0; pub struct",
            "pub mod m { pub const GOOD: u8 = 0; pub struct }",
            "pub mod tests { pub struct }",
            "#[cfg(test)] pub mod ignored { pub struct }",
            "pub mod m { pub const GOOD: u8 = 0; } trailing tokens",
            "#!/bin/rust\npub const GOOD: u8 = 0; pub struct",
        ] {
            assert!(syn::parse_file(source).is_err());
            let mut output = b"earlier file\n".to_vec();
            assert!(audit_source(&mut output, "bad.rs", source).is_err());
            assert_eq!(output, b"earlier file\n", "{source}");
        }
    }

    #[test]
    fn public_source_path_propagates_short_writes_and_failures() {
        struct ShortWriter {
            remaining: usize,
            bytes: Vec<u8>,
        }
        impl Write for ShortWriter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.remaining == 0 {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "source fixture"));
                }
                let length = bytes.len().min(self.remaining).min(7);
                self.bytes.extend_from_slice(&bytes[..length]);
                self.remaining -= length;
                Ok(length)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        for source in [
            "pub mod m { pub const C: u8 = 0; }",
            "#!/bin/rust\npub const C:u8=0;",
        ] {
            let mut expected = Vec::new();
            audit_source(&mut expected, "fixture.rs", source).unwrap();
            for limit in 0..=expected.len() {
                let mut writer = ShortWriter {
                    remaining: limit,
                    bytes: Vec::new(),
                };
                let result = audit_source(&mut writer, "fixture.rs", source);
                assert_eq!(writer.bytes, expected[..limit]);
                if limit < expected.len() {
                    assert_eq!(
                        result
                            .unwrap_err()
                            .downcast_ref::<io::Error>()
                            .unwrap()
                            .kind(),
                        io::ErrorKind::BrokenPipe
                    );
                } else {
                    result.unwrap();
                }
            }
        }
    }

    #[test]
    fn borrowed_json_preserves_unicode_and_escapes_without_changing_bytes() {
        assert_eq!(
            Json("抹茶\"\\\n\r\t\0\x7f\u{85}").to_string(),
            "\"抹茶\\\"\\\\\\n\\r\\t\\u0000\\u007f\\u0085\""
        );
    }

    #[test]
    fn output_failure_at_every_byte_is_reported_without_panicking() {
        struct FailsAfter(usize);
        impl Write for FailsAfter {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                if self.0 == 0 {
                    return Err(io::Error::new(io::ErrorKind::BrokenPipe, "fixture"));
                }
                let written = bytes.len().min(self.0);
                self.0 -= written;
                Ok(written)
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let parsed = syn::parse_file(
            r#"pub mod nested {
                pub struct S { pub field: u8 }
                pub enum E { A { value: u8 } }
                pub trait T { fn call(); }
                impl S { pub fn run() {} }
                pub const C: u8 = 0;
                pub static V: u8 = 0;
                pub type A = u8;
                pub use self::S as Other;
            }"#,
        )
        .unwrap();
        let mut expected = Vec::new();
        items(&mut expected, "抹茶.rs", &parsed.items, "", "").unwrap();
        for limit in 0..expected.len() {
            let error =
                items(&mut FailsAfter(limit), "抹茶.rs", &parsed.items, "", "").unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
            assert_eq!(error.to_string(), "fixture");
        }
        items(
            &mut FailsAfter(expected.len()),
            "抹茶.rs",
            &parsed.items,
            "",
            "",
        )
        .unwrap();
    }

    #[test]
    fn final_flush_failure_cannot_report_a_successful_inventory() {
        #[derive(Default)]
        struct FlushFails(usize);
        impl Write for FlushFails {
            fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
                self.0 += bytes.len();
                Ok(bytes.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush fixture"))
            }
        }
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut out = FlushFails::default();
        let error = audit(&mut out, &source).unwrap_err();
        assert_eq!(
            error.downcast_ref::<io::Error>().unwrap().kind(),
            io::ErrorKind::BrokenPipe
        );
    }
}

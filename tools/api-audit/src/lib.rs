//! Source declaration inventory; macro/cfg resolution is deliberately not claimed.
use quote::ToTokens;
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
    attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg") || a.path().is_ident("cfg_attr"))
        .map(|a| a.to_token_stream().to_string())
        .collect::<Vec<_>>()
        .join(" ")
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
    let parsed = syn::parse_file(source)?;
    items(out, file, &parsed.items, "", &attrs(&parsed.attrs))?;
    Ok(())
}
fn walk(out: &mut impl Write, root: &Path, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut pending = vec![path.to_owned()];
    while let Some(path) = pending.pop() {
        let kind = fs::symlink_metadata(&path)?.file_type();
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            let mut entries = fs::read_dir(&path)?
                .map(|e| e.map(|e| e.path()))
                .collect::<Result<Vec<_>, _>>()?;
            entries.sort();
            pending.extend(entries.into_iter().rev().filter(|p| {
                ![
                    ".git", "target", "examples", "tests", "benches", "testdata", "xtask",
                    ".github",
                ]
                .contains(&p.file_name().and_then(|n| n.to_str()).unwrap_or(""))
            }));
        } else if kind.is_file() && path.extension().is_some_and(|e| e == "rs") {
            let file = path
                .strip_prefix(root)?
                .to_str()
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "source path is not UTF-8")
                })?
                .replace('\\', "/");
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
    walk(out, root, root)?;
    out.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

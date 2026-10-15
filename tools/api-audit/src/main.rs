//! Source declaration inventory; macro/cfg resolution is deliberately not claimed.
use quote::ToTokens;
use std::{env, fs, path::Path};
use syn::{Fields, ImplItem, Item, TraitItem, Visibility, spanned::Spanned};

fn json(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
fn emit(file: &str, line: usize, kind: &str, symbol: &str, cfg: &str) {
    println!(
        "{{\"file\":{},\"line\":{line},\"kind\":{},\"symbol\":{},\"cfg\":{}}}",
        json(file),
        json(kind),
        json(symbol),
        json(cfg)
    );
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
fn fields(file: &str, name: &str, fields: &Fields, cfg: &str, all: bool) {
    for (i, f) in fields.iter().enumerate() {
        if all || public(&f.vis) {
            let n = f
                .ident
                .as_ref()
                .map_or_else(|| i.to_string(), ToString::to_string);
            emit(
                file,
                f.span().start().line,
                "field",
                &format!("{name}.{n}"),
                cfg,
            );
        }
    }
}
fn items(file: &str, declarations: &[Item], prefix: &str, parent_cfg: &str) {
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
                    emit(file, line, "module", &name, &cfg);
                }
                if let Some((_, inner)) = &v.content {
                    items(file, inner, &format!("{name}::"), &cfg);
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
                    let (vis, name, kind) = match it {
                        ImplItem::Fn(m) => (&m.vis, m.sig.ident.to_string(), "method"),
                        ImplItem::Const(c) => (&c.vis, c.ident.to_string(), "associated_const"),
                        ImplItem::Type(t) => (&t.vis, t.ident.to_string(), "associated_type"),
                        ImplItem::Macro(m) => {
                            emit(
                                file,
                                m.span().start().line,
                                "impl_macro",
                                &m.mac.path.to_token_stream().to_string(),
                                &cfg,
                            );
                            continue;
                        }
                        _ => continue,
                    };
                    if public(vis) || tr.is_some() {
                        emit(
                            file,
                            it.span().start().line,
                            kind,
                            &format!("{prefix}<{ty}{}>::{name}", tr.as_deref().unwrap_or("")),
                            &cfg,
                        );
                    }
                }
            }
            Item::Struct(v) if public(&v.vis) => {
                let cfg = format!("{parent_cfg} {}", attrs(&v.attrs));
                let name = format!("{prefix}{}", v.ident);
                emit(file, line, "struct", &name, &cfg);
                fields(file, &name, &v.fields, &cfg, false);
            }
            Item::Enum(v) if public(&v.vis) => {
                let cfg = format!("{parent_cfg} {}", attrs(&v.attrs));
                let name = format!("{prefix}{}", v.ident);
                emit(file, line, "enum", &name, &cfg);
                for variant in &v.variants {
                    let n = format!("{name}::{}", variant.ident);
                    emit(file, variant.span().start().line, "variant", &n, &cfg);
                    fields(file, &n, &variant.fields, &cfg, true);
                }
            }
            Item::Trait(v) if public(&v.vis) => {
                let cfg = format!("{parent_cfg} {}", attrs(&v.attrs));
                let name = format!("{prefix}{}", v.ident);
                emit(file, line, "trait", &name, &cfg);
                for it in &v.items {
                    let id = match it {
                        TraitItem::Fn(f) => f.sig.ident.to_string(),
                        TraitItem::Type(t) => t.ident.to_string(),
                        TraitItem::Const(c) => c.ident.to_string(),
                        _ => continue,
                    };
                    emit(
                        file,
                        it.span().start().line,
                        "trait_member",
                        &format!("{name}::{id}"),
                        &cfg,
                    );
                }
            }
            Item::Fn(v) if public(&v.vis) => emit(
                file,
                line,
                "function",
                &format!("{prefix}{}", v.sig.ident),
                &format!("{parent_cfg} {}", attrs(&v.attrs)),
            ),
            Item::Const(v) if public(&v.vis) => emit(
                file,
                line,
                "const",
                &format!("{prefix}{}", v.ident),
                parent_cfg,
            ),
            Item::Static(v) if public(&v.vis) => emit(
                file,
                line,
                "static",
                &format!("{prefix}{}", v.ident),
                parent_cfg,
            ),
            Item::Type(v) if public(&v.vis) => emit(
                file,
                line,
                "type",
                &format!("{prefix}{}", v.ident),
                parent_cfg,
            ),
            Item::Use(v) if public(&v.vis) => emit(
                file,
                line,
                "reexport",
                &v.tree.to_token_stream().to_string(),
                parent_cfg,
            ),
            Item::Macro(v) => {
                let name = v.ident.as_ref().map_or_else(
                    || v.mac.path.to_token_stream().to_string(),
                    ToString::to_string,
                );
                emit(
                    file,
                    line,
                    "macro_or_invocation",
                    &name,
                    &format!("{parent_cfg} {}", attrs(&v.attrs)),
                );
            }
            _ => {}
        }
    }
}
fn walk(root: &Path, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.is_dir() {
        let mut entries = fs::read_dir(path)?
            .map(|e| e.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()?;
        entries.sort();
        for p in entries {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if [
                ".git", "target", "examples", "tests", "benches", "testdata", "xtask", ".github",
            ]
            .contains(&n)
            {
                continue;
            }
            walk(root, &p)?;
        }
    } else if path.extension().is_some_and(|e| e == "rs") {
        let file = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if file.split('/').any(|p| p == "tests.rs") {
            return Ok(());
        }
        let parsed = syn::parse_file(&fs::read_to_string(path)?)?;
        items(&file, &parsed.items, "", &attrs(&parsed.attrs));
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = env::args()
        .nth(1)
        .ok_or("usage: matea-api-audit SOURCE_ROOT")?;
    walk(Path::new(&root), Path::new(&root))
}

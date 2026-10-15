# ADR 0005 — Design tokens, components and crate boundaries

Status: Implementation decision; provisional names/API, review pending (2026-10-16).

## Decision
Implement six workspace crates at the repository root: style → render → core; widgets uses core/render/style; terminal uses core/render/style and Crossterm; the facade reexports a concise prelude. Core and widgets have no terminal transport dependency. A separate non-image extension crate depends only on public contracts and demonstrates independent composition.

Theme is an explicit value: text, muted, accent, surface, border, error, focus styles plus spacing, borders and reduced motion. Fixed/fill lengths use deterministic integer allocation. Focus uses ordered stable IDs, Tab and BackTab; parents route events once. A Component receives an input event and focus status, returns consumed/action, and builds a View with a Theme. State remains in each component. Forms own child state, validation and traversal. Selection uses a visible marker and focus uses reverse/underline, so color is never the only signal.

## Alternatives and consequences
Unlike Lip Gloss, styles are values rather than ANSI strings. Unlike Huh's broad Field interface, components cannot independently start terminal sessions. A function validator avoids a dynamic validation DSL. Markdown parsing and spring animation remain optional future libraries; viewport and timer-driven spinner validate their required boundaries now. No image implementation or API freeze. Rust source crate composition is supported; no stable binary ABI. Internal crate names and `matea` registry/trademark availability remain unverified; all packages are unpublished.

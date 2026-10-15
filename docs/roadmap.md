# Matea roadmap

| Milestone | Scope | Acceptance evidence |
| --- | --- | --- |
| F0 | Source audit of Bubble Tea, Charm and Rust alternatives at pinned revisions | Audit documents and upstream-lock.json |
| F1 | Runtime, effects, rendering, styling, component and terminal boundaries | ADRs 0003–0006 and executable APIs |
| F2 | Typed state/effects, subscriptions, cancellation, resize and terminal lifecycle | Core and lifecycle tests |
| F3 | Unicode cell buffer, clipping, deterministic diffs, layout and theme | Renderer/style tests and dashboard golden |
| F4 | Input, list, table, viewport, spinner, form and extension contracts | Widget tests and independent component package |
| F5 | Runnable examples, documentation, cross-platform stabilization and owner review | Linux/Windows/macOS tests, fmt/clippy, smoke/PTY evidence and explicit owner approval |

The current implementation is a Foundation candidate. Its public API is provisional. The gate report records actual results and open limitations. Green CI alone does not authorize advancing beyond F5.

Only after Foundation approval, plan an independent image-rendering extension with CPU fallback and conditional CUDA evaluation. Image conversion, protocol encoding and terminal transport will have separate ownership. No image crate, codec, graphics-protocol encoder, CUDA prototype or image benchmark is included in Foundation.

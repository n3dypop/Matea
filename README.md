# Matea (抹茶)

**A delightful TUI framework for Rust.**

Matea is an experimental framework inspired by Charm's approachable developer experience, polished terminal interfaces and composable ecosystem. It uses typed state/messages/effects, structured views, explicit themes and reusable components. APIs and internal crate names remain provisional; crate publication is disabled.

```sh
cargo run -p matea --example counter
cargo run -p matea --example dashboard
cargo run -p matea --example cursor
cargo run -p matea --example attributes
cargo run -p matea --example gradients
cargo run -p matea --example progress
# Plain non-TTY output:
cargo run -p matea --example form -- --snapshot
```

See the [example guide](docs/example-apps.md), [architecture](docs/architecture.md), [Bubble Tea audit](docs/bubbletea-audit.md), [Charm audit](docs/charm-ecosystem-audit.md), and [Foundation gate report](docs/foundation-gate-report.md).

```sh
python3 scripts/verify_foundation.py
```

The workspace contains seven framework crates plus an independent example component package. The earlier limited implementation passed Linux/Windows/macOS CI and MSRV 1.88. Foundation development has reopened for [complete reference capability coverage and performance validation](docs/coverage-plan.md); those earlier results do not establish parity. Pure terminal graphics encoders are included by explicit owner clarification; the image processing engine, decoding and CUDA remain deferred until that gate. See the [roadmap](docs/roadmap.md), [capability ledger](docs/coverage/capabilities.json), [performance evidence](docs/performance.md) and [upstream issue analysis](docs/upstream-defects.md).

The [quality evaluation](docs/quality-evaluation.md) inventories every implemented program and keeps Rust design, safety, behavior, reference parity, performance and portability separate. `python3 scripts/check_quality.py --gate` fails while applicable scores remain below the documented threshold; ordinary green CI does not imply every assessment is high.

The current scope is 138 programs in 42 units, with 49 assessments still below
the high threshold. The [last fully bound seven-job CI](https://github.com/n3dypop/Matea/actions/runs/39178730764)
validates the 135-program StylePatch checkpoint. The 138-program viewport highlight
checkpoint passes 39 native acceptance commands, eight targeted Miri tests and
83-path LLVM execution; its [three-OS CI](https://github.com/n3dypop/Matea/actions/runs/39196946306)
is in progress. [Recent work and remaining limitations](docs/remaining-optimization.md)
include immutable styled text, explicit inherited/set/clear style intent,
whole-grapheme viewport wrapping and validated search highlights. Measurements
retain complete outputs, all raw distributions and rejected trials. The native
Windows console oracle verifies 36 ASCII/BMP frames and restoration; modern
terminal/font/IME appearance remains outside that verdict. Component reference
outputs retain their distinct source bindings. Stage A, Stage B and F5 are not
complete; the image engine and CUDA stay gated.

This repository contains source, tests, examples, CI and engineering documentation. External source checkouts and agent instructions are not part of it. Upstream revisions and declared dependency licenses are recorded in docs; Matea's distribution license and public naming availability remain owner review items.

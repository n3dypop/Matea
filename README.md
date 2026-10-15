# Matea (抹茶)

**A delightful TUI framework for Rust.**

Matea is an experimental framework inspired by Charm's approachable developer experience, polished terminal interfaces and composable ecosystem. It uses typed state/messages/effects, structured views, explicit themes and reusable components. APIs and internal crate names remain provisional; crate publication is disabled.

```sh
cargo run -p matea --example counter
cargo run -p matea --example dashboard
# Plain non-TTY output:
cargo run -p matea --example form -- --snapshot
```

See the [example guide](docs/example-apps.md), [architecture](docs/architecture.md), [Bubble Tea audit](docs/bubbletea-audit.md), [Charm audit](docs/charm-ecosystem-audit.md), and [Foundation gate report](docs/foundation-gate-report.md).

```sh
python3 scripts/verify_foundation.py
```

The workspace contains seven framework crates plus an independent example component package. The earlier limited implementation passed Linux/Windows/macOS CI and MSRV 1.88. Foundation development has reopened for [complete reference capability coverage and performance validation](docs/coverage-plan.md); those earlier results do not establish parity. Pure terminal graphics encoders are included by explicit owner clarification; the image processing engine, decoding and CUDA remain deferred until that gate. See the [roadmap](docs/roadmap.md), [capability ledger](docs/coverage/capabilities.json), [performance evidence](docs/performance.md) and [upstream issue analysis](docs/upstream-defects.md).

This repository contains source, tests, examples, CI and engineering documentation. External source checkouts and agent instructions are not part of it. Upstream revisions and declared dependency licenses are recorded in docs; Matea's distribution license and public naming availability remain owner review items.

# ADR 0007 — Full public capability coverage before F5

Status: accepted owner scope direction, implementation in progress (2026-10-16).

The owner expanded Foundation acceptance to every public user-visible feature of the six Charm references and the referenced base libraries, and requires module-level performance comparison, upstream issue analysis and Rust-focused optimization. The earlier green CI validates only a limited prototype. Reopen F0–F4 and withdraw the prior F5 approval request.

Maintain an exhaustive source/API inventory plus a capability ledger. Exported names are evidence anchors, not a requirement to translate Go signatures literally. Platform/cfg and macro-generated APIs must be explicit. Reuse of Rust libraries is allowed behind integrated, tested contracts; unsupported or unmeasured functionality stays pending.

All implementation stays in the isolated Matea Git root. External references and agent instructions remain outside. The owner subsequently permitted terminal image protocol encoders in Foundation. Image processing, codecs and CUDA remain gated. See coverage-plan.md for benchmark comparability and acceptance rules.

# Foundation risk register

| Risk | Current mitigation / acceptance requirement |
| --- | --- |
| Three-OS execution unverified | CI workflow supplied; actual remote runs required before gate acceptance |
| Provisional public API / names | Owner API review, registry/trademark checks before publication; publish=false |
| Distribution license not selected | Owner must select project license before distribution; upstream license facts captured separately |
| Non-cooperative user tasks can outlive Runtime | Explicit tokens/deadlines, per-runtime active limit, stale-result discard; no forced-kill promise |
| Narrow custom renderer maintenance | Row clear + explicit positioning; Unicode/diff/clip tests; revisit Ratatui before broader renderer scope |
| Emoji/ambiguous widths differ by terminal | Non-CJK width policy, atomic clusters; real-emulator/SSH/tmux validation outstanding |
| Real Windows console restoration | Portable mock tests and workflow; real Windows console still unverified |
| Accessibility and motion | Textual focus/error cues, NO_COLOR, reduced motion, plain snapshots; interactive screen-reader prompt mode outstanding |
| Surface memory / effect flood | Surface allocation cap, 8 workers/32 envelopes/64 subscriptions; component/document content still app-owned |
| Signal, abort or failed output cleanup | SIGINT/SIGTERM flags and panic unwind cleanup; SIGKILL/abort/I/O failure cannot be guaranteed |
| Interactive feature coverage | Inline, mouse, function keys, enhanced keyboard, clipboard, suspend/resume deferred and explicitly unsupported |
| Publication/source provenance | No Go code/assets copied; root licenses audited; Rust dependency license inventory is not a legal certification |

Future image/CUDA work is deferred as described in roadmap.md; it requires explicit Foundation approval.

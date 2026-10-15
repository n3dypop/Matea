# Extension contracts

## Executable public boundary

`examples/external-widget` is a separate package. Its Badge implements only public Component, Input, Response and View/Theme contracts; no runtime change or private import is needed. The parent maps its boolean action and owns application focus. Run `cargo run -p matea-example-external-widget` or append `-- --snapshot`.

A Component owns state; `event(input, focused)` returns consumed and optional typed action; `view(theme, focused)` is read-only. Child actions do not directly control the terminal. A parent forwards an event once, routes only to the focused target and maps actions to its own Message. Components may expose domain-specific constructors, sizing and methods while preserving these shared contracts. Effects requested by a parent use the normal runtime cancellation path. Each app instance owns its component state; dropping it releases those values.

A third-party library can depend only on matea-core/render/style; the example uses the facade for discoverability. The Backend trait supports alternate terminal transport implementations and mock testing. A transport must obey bounded polling timeouts and return errors rather than panic during restoration. This is ordinary Rust source composition; no binary ABI, dynamic plugin compatibility or stable 1.0 API is promised.

## Future graphics boundary — design only

After explicit Foundation approval, an independently usable image library may produce cells or a pixel/graphics artifact. Image conversion must not write stdout, emit terminal commands, own raw mode, or require the runtime. CPU/GPU processing, protocol encoding, layout and terminal transport stay separate.

The future compositor would own placement, clipping and layer order; terminal session/backend would own negotiated protocol support, unique resource/placement IDs, upload/delete, cache invalidation and cleanup. A negotiated unsupported capability would choose a cell/text fallback or an explicit unsupported error. Resize/scroll and session shutdown must invalidate or dispose resources. IDs cannot survive a session accidentally. OS/TERM alone never prove Kitty/Sixel support.

No image-specific structs, image node, protocol encoder, codec, GPU prototype, image crate or image benchmark is implemented. The shape above is deliberately not a frozen public API. Existing supplied image source references are research inputs only. A separate ADR and tests will be required after gate approval.

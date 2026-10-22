# Third-party implementation notices

## Native terminal devices

The altered Rust native-device flag transitions and byte input behavior in
matea-terminal are informed by Charmbracelet x/term at
4542a189d0fbeb156bb1df8730d253f2ef443c00. Its MIT notice is Copyright (c) 2023
Charmbracelet, Inc.; the full MIT permission and disclaimer below apply to these
adapted sections as well. The source was audited outside this repository.
Matea adds borrowed-handle lifetimes, owned retryable restoration, explicit input
limits/partial errors and zero-read termination. It does not expose an unchecked
descriptor API or reproduce the source's zero-read loop.

## Terminal color gradients

Matea's gradient color conversions are an altered Rust implementation informed
by go-colorful v1.4.1 (colors.go). Segmented/grid sampling also follows Lip Gloss
blending.go at 19f011f1. The common MIT notice for these adapted sections:

```
Copyright (c) 2013 Lucas Beyer
Copyright (c) 2021-2026 Charmbracelet, Inc.
Copyright (c) 2020-2026 Charmbracelet, Inc. (Bubbles progress and viewport behavioral references)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies
of the Software, and to permit persons to whom the Software is furnished to
do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
THE SOFTWARE.
```

Progress in matea-widgets is an altered Rust implementation informed by Bubbles
progress/progress.go at c3bde0d166c2ca607d9864493453fd77bdcae7e4. The MIT terms
above also apply to those adapted sections; its original notice is Copyright
(c) 2020-2026 Charmbracelet, Inc. Matea uses independently owned state, typed
clock generations, semantic theme defaults, validated geometry and corrected
absolute-velocity rest detection. See ADR 0046 for deliberate differences.

Viewport content/vertical-control behavior and public fixtures are informed by
Bubbles viewport/viewport.go at the same revision. The independent Rust state
owns content, validates index/resize invariants and saturates movement; CRLF and
overflow differences are recorded in ADR 0047 and the viewport source audit.

## Terminal style color inputs

The altered Rust color-input parsing and RGB8 adjustment behavior in
`matea-style/src/color_input.rs` is informed by Lip Gloss `color.go` at
19f011f1cc573e5d2bf9d7a9c4f14d429647759e. Its MIT notice is Copyright (c)
2021-2026 Charmbracelet, Inc.; the full MIT permission and disclaimer above apply
to these adapted sections. Matea adds explicit errors, unknown default colors,
canonical depth conversion, validated premultiplied types and explicit known
background composition. The original upstream code is not vendored here.

## Motion implementation attribution

Matea's Rust spring implementation is an altered implementation informed by
Charmbracelet Harmonica and Ryan Juckett's damped-spring equations. It is not the
original C++ or Go software. Harmonica's repository license is MIT. Its spring
source additionally preserves this notice:

```
Copyright (c) 2008-2012 Ryan Juckett
http://www.ryanjuckett.com/

This software is provided 'as-is', without any express or implied
warranty. In no event will the authors be held liable for any damages
arising from the use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not
   claim that you wrote the original software. If you use this software
   in a product, an acknowledgment in the product documentation would be
   appreciated but is not required.

2. Altered source versions must be plainly marked as such, and must not be
   misrepresented as being the original software.

3. This notice may not be removed or altered from any source
   distribution.
```

## Structured block decoration

`matea-style/src/decoration.rs` and `matea-render/src/block.rs` are altered Rust
implementations informed by Lip Gloss style/set/get/unset/borders.go at
19f011f1cc573e5d2bf9d7a9c4f14d429647759e and x/ansi wrap.go at
faa4adf95555331dfb439b716cc83767ecdadeb4. The MIT copyright/permission/disclaimer
above apply to the adapted sections (Charmbracelet, Inc.). Matea separates
optional layout intent from cell values, owns shared immutable text/patterns,
uses whole graphemes and checked cell geometry, and rejects non-progressing or
control-bearing border patterns. The source is audited outside this repository.

# Third-party implementation notices

## Terminal color gradients

Matea's gradient color conversions are an altered Rust implementation informed
by go-colorful v1.4.1 (colors.go). Segmented/grid sampling also follows Lip Gloss
blending.go at 19f011f1. The common MIT notice for these adapted sections:

```
Copyright (c) 2013 Lucas Beyer
Copyright (c) 2021-2026 Charmbracelet, Inc.
Copyright (c) 2020-2026 Charmbracelet, Inc. (Bubbles progress)

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

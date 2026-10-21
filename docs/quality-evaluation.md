# Implementation quality evaluation

This is a reproducible review and test plan for **all implemented programs**, not
an assertion that Foundation or the referenced public APIs are complete.
[The explicit inventory](quality-plan.json) maps every Rust, Go, Python and CI
program file to a review unit. New unlisted files fail the inventory check.
[The generated report](evidence/quality/report.json) records each file's source
hash, assessment and available native execution coverage. No average hides a
failing dimension; `python scripts/check_quality.py --gate` fails until every
applicable assessment is at least 75.

## Scoring contract

Scores are ordinal evidence levels, **not percentages of correctness**:

| Score | Required interpretation |
| --- | --- |
| 0 | No executed evidence for this dimension/workload. |
| 25 | Known defect or review only; behavior/performance is not validated. |
| 50 | Partial evidence, missing behavior/measurements, or unresolved regressions. |
| 75 | High **within the explicitly bounded reviewed/tested scope**; independent fixtures and executed checks support it. |
| 100 | Exhaustive evidence for an explicitly finite domain; never universal safety or maximum speed. |
| N/A | The dimension has no counterpart (e.g. Rust ownership for a Python utility); a reason is mandatory. Unmeasured performance is 0, never N/A. |

The assessments are conservative reviewer judgments, with limitations in
each row. Passing a native test does not establish reference parity. Passing an
inventory check does not establish a passing quality gate. Tests/configuration
have no production latency acceptance criterion, so performance is N/A there;
auditors, report programs and examples need their own performance evidence;
entirely unmeasured units receive 0 and partially measured units remain at 50.

## Evaluation and test items

| Dimension | Review questions | Executed checks / required follow-up |
| --- | --- | --- |
| Rust design | Are state and resources owned once? Can invalid states be represented? Are borrows/lifetimes simple, cancellation and errors explicit, and extension contracts independent? | All-target Clippy with warnings denied, documentation, compile-fail immutability, standalone widget; targeted runtime identity/deadline tests. Public API/registry review remains open. |
| Memory safety | Is unsafe isolated and justified? Are arithmetic, indexing, UTF-8 slices, aliasing, mutation, cleanup and thread ownership valid? | Safe-code crate policies, bounded paint/layout models, exhaustive Unicode scalar policy, targeted Miri. The Unix sigaction boundary and performance-only System allocator wrapper are explicitly unsafe; transitive dependencies are not asserted unsafe-free. |
| Behavior | Are boundaries, empty input, control characters, wide glyph overlap, resize, stale messages, failures/panics and recovery correct? | Debug/all-target/release tests, snapshots, independent terminal trace interpreters, 20 Unix PTY cases, graphics protocol round trips, before/fix regression logs. These do not validate every physical emulator. |
| Reference equivalence | Are semantics/outputs equal at the pinned SHAs? Are deliberate improvements documented and unavailable features visible? | Finite Harmonica/paginator/key groups, clock/spinner fixtures and matched performance output checks. Revised whole-reference coverage remains incomplete; see the separate declaration/compiler ledger. |
| Performance | Are constructor and complete pipeline costs both measured? Are p50/p95, allocations, output bytes and regressions retained? | Original 84 reference-matched finite cases plus seven virtual-cursor and six fresh RGB8 gradient cases; six separate gradient streaming controls, 5 subscription and 20 scene Rust-only cases, 12 Viewport before/after cases; long clock repeats overlap the 84. No instrumented timing is performance evidence. Missing resident memory, scheduling/application latency and module workloads remain pending. |
| Portability | Does this exact implementation run on Linux, Windows, macOS and MSRV? Are terminal/FFI limits explicit? | Foundation CI plus MSRV. Native host coverage/Miri is additional evidence, not another OS. Windows PTY and physical emulator behavior remain unverified. |

## Redevelopment performed

1. **Viewport behavior: 25 → 75 for the tested slice.** The first three added tests
   failed before the fix: trailing blank lines disappeared, CRLF became a visible
   replacement character, and a grown viewport exposed an obsolete offset before
   navigation. [The known-negative log](evidence/viewport-before.log) is retained.
   Immutable content plus UTF-8 line offsets preserves trailing lines, normalizes
   CRLF and clamps the observed position before navigation. The independent line
   model covers zero heights, controls, Unicode and navigation. A fourth test
   prevents ignored input from committing a resize position change; its
   [failing intermediate case](evidence/quality/viewport-ignored-before.log) is retained. The containing
   basic-widget module remains partial; this does not award List/Table/Form parity.
2. **Viewport allocation/performance: 50 → 75 for 12 finite workloads.** ASCII
   24-row View generation changes from 26 allocations / 1,559 requested bytes to
   1 / 503; p50 1,096.21 → 364.90 ns. Complete build/render changes from 27 to 2
   allocations and p50 11,916.92 → 11,058.44 ns. The one-row ASCII complete
   workload has a p95 regression (759.71 → 853.68 ns). All samples, output
   equivalence and workload exclusions remain in [the first report](evidence/quality/viewport-first-report.json); the final rerun is in
   [the current report](evidence/quality/viewport-report.json).
   This excludes initial parsing and resident storage, so the whole Viewport
   performance dimension remains 50 until those and matched upstream work exist.
3. **Source auditor conditions: 25 → 75 for the new nested-cfg fixture.** Rust
   field, enum variant/payload, impl/trait member, const/static/type/reexport
   conditions were omitted. [The before failure](evidence/quality/auditors-before.log)
   reproduces the lost `unix` constraint. Regenerated inventories correct 80 Crossterm and
   42 Ratatui condition records while retaining all declaration IDs/counts. Child conditions now combine with the
   parent conditions without evaluating them. Go fixtures also verify build tags,
   embedded types, exports and benchmark filtering. The complete compiler-export
   audit and scanner failure/scale cases remain separate work.

4. **Evidence validation: 25 → 75 for optimization-mode regression.** Python
   `-O` removed `assert` statements, including a complete replay call, allowing an
   invalid trace to print “exact states”. The [before failure](evidence/quality/optimized-validation-before.log)
   is retained. Replace 138 removable checks in 14 production scripts with an
   explicit `require` function; lazy messages preserve failure-only evaluation.
   Tests reject invalid traces in normal/optimized modes and prevent new removable
   checks in production scripts. All 16 Python regression cases run in both modes.

## Reproduction and limits

Run `python scripts/verify_foundation.py --offline` for the native test, lint,
release, documentation, example, Python syntax, auditor/oracle and inventory suite.
Run `python scripts/measure_rust_coverage.py` after installing matching official
`llvm-tools-preview` to collect instrumented native tests, example/PTY/CLI runs.
Its line/function totals include embedded `cfg(test)` code; they are not branch
coverage or production-only percentages. Compiler/profile raw files stay in
ignored `artifacts/quality-coverage`. Failed syscalls, process creation and rare
OS error branches still need fault-injection evidence.

The targeted Miri commands were `MIRIFLAGS=-Zmiri-seed=17 cargo +nightly
miri test --locked --offline -p matea-render --test invariants` and
`MIRIFLAGS=-Zmiri-seed=29 cargo +nightly miri test --locked --offline -p matea-core
--test subscription_atomicity --test subscription_identity`. The paint model uses
32 steps under Miri and 4,096 natively. Four additional Text ownership/formatting/
clipping tests ran with seed 41, excluding the million-scalar native test. Nine
targeted Miri tests passed in total; no full-workspace Miri claim is made.
[Miri's own documentation](https://github.com/rust-lang/miri#readme) explains why
a successful execution cannot prove soundness or explore all thread schedules.

Do not loosen thresholds or drop adverse cases to make the report green.
Address the generated below-threshold queue with independent regression tests
and fresh measurements, then update only assessments supported by that evidence.
The initial four-job CI succeeded but skipped one Go fixture on macOS. The
[initial excerpt](evidence/quality/initial-source-auditor-ci-excerpt.log) preserves
that gap. Pin official setup-go v6 to commit
`924ae3a1cded613372ab5595356fb5720e22ba16`, install Go 1.26.0 and make a missing Go
CLI fail the fixture. Require fresh CI with all two source-auditor cases and all
16 optimized Python cases executed on every OS. Windows PTY and the optional
reference recollection remain separate exclusions.

F5 and the image-engine/CUDA gate remain closed.

Final implementation `9ce5564ad525d047594c5e1debecabd7c3bb4e37` passed [Linux, Windows, macOS and MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39043211231). The generated report retains 56 below-threshold assessments: performance 28, reference equivalence 18, behavior 5 and external-tool portability 5. These are incomplete evidence/behavior scopes, not 56 confirmed defects. The quality gate still fails.

The individual optimization follow-up adds 127 measured fixtures (124 Rust primitive/packet fixtures plus three complete auditor processes). [Results and retained regressions](individual-optimization.md) include exact all-RGB ANSI256 checking, adaptive owned focus indexing, bounded stack protocol headers, fallback style restoration and streaming source inventories. Eleven additional distinct targeted Miri tests passed: [commands, seeds, target hashes and logs](evidence/optimization/miri-context.json). The current native suite runs 129 Rust cases plus two doctests, one explicit ignored exhaustive case, 24 Python regression cases in normal/optimized mode, 12 snapshots and 17 Unix PTY cases. Six performance assessments move from 0 to 50; no dimension is promoted to high by these partial measurements. Of 86 program files in 40 units, 56 assessments remain below 75 (performance 28, reference 18, behavior 5, portability 5). Fully unmeasured performance units decrease from 18 to 12. The whole-framework optimization stage has not started.

Implementation `203ba5d7c56f3588a2dadb14410e3aafe3ead59f` passed [all three OSes plus MSRV 1.88](https://github.com/n3dypop/Matea/actions/runs/39049417573). [Step/log evidence](evidence/optimization/ci-run.json) confirms two Go/Rust auditor fixtures, 24 optimized Python cases and 46/37/41 current module records on each OS. This is ordinary Foundation CI; reference recollection and Windows PTY remain separate exclusions. The 56 below-high assessments and both gates remain unchanged.


## 未測定12単位への追補

[76件の追加測定と修正](remaining-optimization.md)により完全未測定の性能単位は0です。
現在のfacade再公開に追加のruntime処理がないことを型・関数別名と3測定経路で確認し、
この単位の性能を75へ更新しました。List/Table/Formは96+30+6境界構成と4panic位置の
独立試験を追加し、再現した無効選択を修正したため、実装済み動作を75へ更新しました。
その他11単位の新しい性能測定は部分証拠の50で、元の範囲を縮めて高評価とはしません。
100プログラム・40単位で51軸が未達です。LLVM report schema 2はcanonical sourceごとに
全compiler instanceを保持し、合計・平均・最大値を作りません。
compiler抽出2件を加え、追補は78固有workloadとなりました。独立26 compiler監査fixtureと
全20 profileのgraph一致、3 OSの実source/API経路を確認し、compiler監査の動作・移植性と
coverage/issuesの移植性を75へ更新しました。内訳は性能27・参照18・動作3・移植性3です。
出力専用CI recordには計時値を作らず、性能record readerはこの型を拒否します。
[最新CI・native検証・途中失敗と残件](remaining-optimization.md)。

## 仮想カーソル追加後の評価

21公開宣言を対応付けたVirtualCursor、文字入力との合成、inline表示修正を追加しました。
現在の範囲は103プログラム・41単位、未達53軸（性能28・参照19・動作3・移植性3）です。
51→53は新しい独立単位の性能／参照一致が部分評価であるためです。閾値や既存範囲は変更していません。
新規6統合試験、Miri、13 snapshot、18 Unix PTYと
[3 OS・MSRV・参照照合7ジョブ](https://github.com/n3dypop/Matea/actions/runs/39075023243)に成功しました。
制御の改善、遅いGo比、p95悪化、全分布は[追補](remaining-optimization.md)に保持しています。
F5と横断最適化の条件は未達で、画像エンジン／CUDAは着手していません。

## 検証器の再開後の評価

現在は107プログラム・41単位、未達49軸（性能28・参照一致19・動作1・移植性1）です。
棚卸し／課題処理の動作、Go計測器の移植性、受入／LLVM実行器の動作と移植性の4軸を、
独立故障試験と[7必須CIジョブ](https://github.com/n3dypop/Matea/actions/runs/39079660329)に基づき75へ更新しました。
[ソース対応・3 OS別の実成果物](evidence/measurement-drivers/ci-source-bindings.json)。
Goの短バッチ失敗2件を保持し、20ms校正でも31の計時標本をそのまま検査します。
103 Python／5 Go試験、Unix 33／Windows 32受入チェックと全3 OSのLLVM収集を実行済みです。
品質reportの公開には、完走したnative journalと現在のsource/build/check入力・ログのhash一致を要求します。
全参照機能・性能最大化・F5の完了を意味しません。残件は[追補](remaining-optimization.md)で継続します。

期限対応の追加では、短周期購読を遅らせる16ms待機と、入力前の待機時間を新規購読へ
加算する不具合を修正しました。[最終3 OS CI](https://github.com/n3dypop/Matea/actions/runs/39088967599)、
ローカルRust 148＋2 doctest、Python 103、33受入チェックとLLVM収集、
5固有テストのMiri 9実行が成功しています。
[全分布・CPU対照の悪化・source/binaryの対応](evidence/deadlines/report.json)を保持し、
性能の部分評価50と全49軸の未達を維持します。これはStage Bへの移行ではありません。

## Compact resolved text attributes

Source 4d1a397 adds typed attributes, six underline styles/colors and reduced-motion
blink suppression, with 13,824 native state/SGR combinations and six targeted Miri
tests. Native validation passes 33 checks, 155 Rust tests plus two doctests, 105
Python tests, 14 snapshots and 19 Unix PTY cases. LLVM records 62 source files/67
separate instances. The inventory now covers 111 programs/41 units/49 low axes.

The bounded color cache no longer keys on unrelated attributes. Matched repeats
retain output bytes and zero repaint allocations; 16-variant ANSI16 median drops
from 4.207–4.211 µs to 2.670–2.688 µs. Theme/style/frame regressions and a 20-byte
short-panel allocation increase remain open. Exactly two legacy Debug outputs
require an explicit default-field transform; raw records are untouched.
[Audit, domains and all limits](text-attributes-audit.md),
[all controls](evidence/attributes/report.json),
[balanced repeats](evidence/attributes/repeat-summary.json).

The Style API is provisional; bool-field migration is documented. A cell still
occupies 40 bytes on the measured host. This does not establish complete style
parity, native-terminal appearance, resident memory or universal maximum speed.
The performance/reference scores and both strict gates remain unchanged.

## Owned panel payload follow-up

At 29ffeea, a panel snapshots only the three consumed style tokens and stores
them with its child in one owned allocation. Short construction changes from
2 allocations/160 requested bytes to 1/96; the token-only intermediate is
retained. All 20 outputs match, while complete-render timings remain mixed.
Six-run balanced repeats preserve every distribution. Native validation passes
33 checks, 156 Rust tests plus two doctests, 105 Python tests and targeted Miri.
[Three-OS/MSRV/reference CI](https://github.com/n3dypop/Matea/actions/runs/39093118277)
passes seven jobs. All hosts observe View 48/Cell 40 bytes and match all 14
extended-attribute outputs. [Evidence and limits](panel-performance.md).

The default-underline guard experiment is not adopted: early and repeated
results mix. Its data and expanded Default/RGB correctness fixture remain.
A malformed-record follow-up reproduces five failures (duplicate fields,
correctness records masquerading as timings, incomplete calibration and a
negative cold counter). These are recorded pending fixes; ordinary green CI
does not erase the behavioral gap. The 111-program/41-unit inventory still has
49 low assessments. No threshold, scope or gate is relaxed.


2026-10-22の[全48軸の原因分析](unmet-axes-analysis.md)と[監査器の修正・測定](oracle-performance-audit.md)により、
維持プログラム158／44単位の性能1軸を75へ更新し、未達は47（参照21・性能26）です。
校正した短制御列の追加検証コスト1.3–2.5µsと全adverseを明記し、平均や範囲縮小で点数を上げていません。
3 OS＋MSRVの4必須CI成功と200入力を照合し、評価更新後47受入、Rust285＋14 doctest、Python151（skip 2）、LLVM97/104を確認しました。
最初のinventory失敗を保存し、同一ソースの46成功を保持して1件のみ再試験。Rust変更がないためMiriを追加実行していません。
Stage Aは継続、Stage BとF5は閉じています。

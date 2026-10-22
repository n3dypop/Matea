# 未達48軸の原因と達成条件

逐次実行へ変更：現在は `matea-style/lib` の参照軸だけを処理する。
[実行順・小項目・軸の達成条件](sequential-axis-plan.md)。色入力の小項目で
33宣言を新たに対応付けたが、軸全体は未完なので50を維持し、未達は47軸。
現在のraw未リンクは6,650。既存の未対応範囲を色入力能力として明示したため、
部分／未測定能力の行数は22となる。軸の追加・分割・点数の引上げは行わない。

追補：検証器の性能1軸を、修正・全操作測定・native/3 OS qualificationに基づき50→75へ更新しました。現在は47軸未達（参照21・性能26）です。[達成の証拠と追加検証コスト](oracle-performance-audit.md)、[範囲を維持した評価差分](evidence/oracle-scale/assessment-review.json)。以下の48軸表は分析時点の元の理由と範囲を保持します。

2026-10-22。基準コミット `ce3d2038c6f788bce87af9e165019045e6e7d761`。点数・閾値・範囲を変更せず、21参照軸と27性能軸を全件分析した。

未達の主因は、機能/公開条件の未完、測定範囲の不足、非同等な比較経路、実測の adverse、試験設計の不足である。48は確認済みバグの件数ではない。詳細な元理由・ファイル・証拠・各軸の達成条件は [機械可読表](evidence/oracle-scale/unmet-axes.json) に保持した。

停滞への対応は、算術microbenchmarkの反復から、未実装契約と未測定の全操作を埋める順序へ変えること。時計4候補は可変入力を含めて不採用を維持する。既存CIやnative試験の再実行だけでreference/performanceを昇格しない。

| 単位 / 軸 | 未達の具体的理由 | 次の修正・検証 |
| --- | --- | --- |
| `matea-core/input` / reference | 入力型の対応付けは部分的。Crossterm の変換実装があっても、全イベント・修飾・プラットフォーム条件の公開契約が閉じていない。 | Bubble Tea / Crossterm / Ultraviolet のイベント宣言を型・変換・焦点の責務へ振り分け、未対応の入力表現を実装し、条件別の正負 fixture を結ぶ。 |
| `matea-core/input` / performance | 37 Rust-only primitive では source と同意味の入力経路・focus 変更/lookup 分布・保持メモリがない。 | 型変換/focus の cold/build/warm/mutate、件数0/1/64/多、重複/無効入力、native event burst を分けて source と完全出力を比較し、保持後/解放後のメモリを測る。 |
| `matea-core/lib` / reference | 37入力 primitive という古い評価理由は runtime 全体を説明しない。購読/期限の既存改善はあるが、効果・外部プロセス・終了等の全公開契約の対応付けがない。 | Program/options/commands/subscriptions の公開項目を runtime と terminal に割り当てる。取消・終了・panic・worker の保証範囲を独立モデルと照合し、未実装契約を縦に実装する。 |
| `matea-core/lib` / performance | 古い理由は入力37件だけを記すが、購読5件や期限待機の既存測定もある。実source同等のactive timer/effect/queue/cancel/worker通知・保持コストは未完。 | 同じ生成/通知/処理/取消を行う runtime workload を用意する。購読維持/差替え/終了、遅延/多量メッセージ、期限誤差、p50/p95、CPU/保持メモリを測る。 |
| `matea-render/lib` / reference | 所有 scene、rich text、snapshot は実装済みの一部。Ultraviolet/Ratatui の全描画・buffer・cursor 契約、出力条件が閉じていない。 | scene/compositor と transport の境界を維持し、原点・clipping・wide cell・resize・dirty/diff・cursor の公開契約を全件分類し、独立セル/端末モデルへ結ぶ。 |
| `matea-render/lib` / performance | scene20件等は部分。構築/正規化/装飾/再描画で adverse 分布があり、大規模/変更密度/保持の比較がない。 | constructor と build→compose、share/consume、full/diff を分離する。短長ASCII/Unicode/control、変更多寡、resize を保持し、候補は現APIで全出力/分布/メモリを再比較する。 |
| `matea-render/text` / reference | 安全な Text と owned RichText は ANSI/Unicode 全ユーティリティの代替を証明しない。source の escape-aware 処理と Matea の正規化は同じ意味ではない。 | x/ansi と Ratatui text の公開操作を保全する rich parser / 正規化する plain text へ分類し、wrap/truncate/width/controls/hyperlink 等の不足を入力と出力の契約ごとに実装する。 |
| `matea-render/text` / performance | Text 正規化の構築コストと source の意味差がある。時計 profile も UTF8/正規化を含む構築コストを示すが原因の寄与率は未確定。 | 安全性を維持する所有/検証境界の候補を実APIで試す。正規化済/制御混在/多言語/小長/clone/owned formatting と retained storage を分け、source と意味が一致する操作だけ比率を出す。 |
| `matea-style/lib` / reference | resolved attributes・RGB8 gradient・patch は部分 capability。Lip Gloss の装飾/geometry、全色入力、条件付き API の同等性が未完。 | padding/margin/border/alignment/wrapping/transform と色入力の公開項目を分離して実装する。既定値だけでなく組合せ、max/min size、空白と継承をセル fixture で検証する。 |
| `matea-style/lib` / performance | x/ansi の色距離と Matea の色距離が違い、既存46 primitive を全 style の速度比へ使えない。 | 同じ色距離/alpha/depth で比較可能なケースを作る。style patch/装飾/gradient の cold/warm/varied/大規模と保持コストを測り、既存 adverse を残して最適化する。 |
| `matea-terminal/ansi` / reference | 26 private fixtures は任意 Crossterm Command や x/ansi 全公開制御列を網羅しない。 | 公開 command family を列挙し、typed encoding・能力照会・生制御の所有者を定める。未対応 command と platform branch を literal bytes と独立 interpreter で検証する。 |
| `matea-terminal/ansi` / performance | 26 private fixtures は一般 Command と native OS 出力のコストを含まない。 | 実装済 command 全族の生成/encode/バッチ/色fallback/失敗を測る。source と同 byte 列・同 flush 条件の比較と非同等ケースを分け、output bytes/alloc/RSS を記録する。 |
| `matea-terminal/frame` / reference | 現在の FrameEncoder は全 Ultraviolet renderer/buffer/diff 契約の対応表がない。 | diff/cursor/resize/style reset/color fallback/invalidated frame の全公開契約を frame と render に分配し、復旧フレームを含む連続出力を独立再生する。 |
| `matea-terminal/frame` / performance | 属性/テーマ/色fallback/フレームの adverse が残り、差分密度やnative transportを網羅しない。 | 空/疎/全面/resize/cursor-only/失敗復旧を同じ frame schedule と能力で比較する。bytes/write/flush を保持し、state reuse と出力 batching を実測して適用する。 |
| `matea-terminal/input` / reference | Crossterm event→typed Input は実装済みだが byte parsing、モード依存・OS 条件を含む source 全体の対応がない。 | parser の再利用境界と Matea の変換を明示し、release/repeat/IME/paste/mouse/focus/resize と unsupported 表現の契約を条件ごとに検証する。 |
| `matea-terminal/input` / performance | 37 primitive は byte→event→Input の native parser、burst、resident を含まない。 | parser単体/変換単体/実入力配送を分ける。ASCII/Unicode/paste/mouse/key-kind と境界分割、取消、OS条件を同入力で測り、source を二重parseした不公平比較を避ける。 |
| `matea-terminal/lib` / reference | mock の正常/失敗 event loop は native session・実プロセス/effect・全 Program API の代替証拠ではない。 | core の runtime 項目と重複しない native lifecycle/exec/suspend/error/cleanup の責任を対応付け、各失敗点と復旧を実行する。 |
| `matea-terminal/lib` / performance | 12 mock loops は実I/O、effect・scheduling・保持メモリを含まない。 | PTY/native console と mock を分け、同イベント列の入力→update→render→flush、終了/失敗と保持を測る。CPU/待機/通知遅延を別記し、source と同じ lifecycle を比較する。 |
| `matea-terminal/modes` / reference | lease の局所実装は公開 terminal mode/capability 全体の同等性を閉じていない。 | query/enable/disable/restore の source 条件を列挙し、共有 lease・初期状態・途中失敗・Windows native 分岐ごとに復旧を検証する。 |
| `matea-terminal/modes` / performance | 6 lease/Unix cycle の logical writes から physical I/O・失敗・Windows の速度は導けない。 | 既存状態からの共有/排他 lease、途中失敗、復旧、native Windows を同じ mode set/flush で測る。transport 往復は encoder と別分布にする。 |
| `matea-terminal/protocol` / reference | Kitty 82 raw 宣言中64は bounded実装、2部分、16 pending。Sixel/能力/lifecycle も部分。codec 等は承認待ちの別 scope。 | File/ChunkFormatter と非codec protocol operations の不足を埋める。画像処理/codec を実装せず、placeholder/diacritic、transport、Sixel 制御、照会/取消を固定 source と検証する。 |
| `matea-terminal/protocol` / performance | 14 eligible Kitty cases は全writer比較ではなく public primitive composition。空delimiter差、strict rejection、他transport/Sixel/throughput は別。 | 既存14条件を保持し、非codec writer/chunkiterator/transport/large payload/control parse/Sixel の完全出力とcold/warm/retained/RSSを測る。delimiter差や image writer と packet を同一比率へ混ぜない。 |
| `matea-terminal/signals_unix` / reference | 安全な所有/復旧の局所試験は全 signal/OS 条件の公開契約対応を示さない。 | Bubble Tea/Ultraviolet の signal lifecycle を列挙し、再入・既存 handler・部分設置失敗・終了・OS 条件を設計に対応させ、実行可能な failure injection と実 signal を分けて検証する。 |
| `matea-terminal/signals_unix` / performance | 31 install/restore と4 synthetic tests に実signal配送・syscall failure・OS別性能がない。 | 自プロセス/子の signal install→receive→cleanup と既存handler復旧を測る。合成失敗の処理時間と実OS配送遅延を分離し、Linux/macOS の native 分布と保持を記録する。 |
| `matea-widgets/clocks` / performance | 算術4候補は全て adverse があり不採用。追加468recordsで可変durationは測定済みだが source差、active timer/cancel/notification/retained は残る。 | ADR0076 に従い算術だけの再試行を止め、安全な Text 構築を実APIで比較する。同時に actual source timer 作成/通知/取消を含む同意味 workload、変化するclock状態と保持を測る。 |
| `matea-widgets/help` / reference | 27宣言はリンク済みだが、7 style role の arbitrary Lip Gloss decorators が部分。文字列の幅修正や高速化では padding/border/wrap の不足を埋められない。 | 共通 decorator の geometry/空白/背景/transform 契約を先に実装し、各 help role の装飾前後の列幅/ellipsis を source fixture と比較する。 |
| `matea-widgets/lib` / reference | Viewport の wrap/highlight 等は部分、基本 List/Table/Form は Bubbles/Huh の全機能ではない。 | List delegate/filter/keymap、Table selection/style、Form/field lifecycle と Viewport gutter/highlight/wrap を独立 capability に分解し、全 source 宣言を所有者と試験へ結ぶ。 |
| `matea-widgets/lib` / performance | wrap10+borrowed2 は List/Table/Form や viewport parse/gutter/highlight/large-varied/resident 全体を覆わない。 | 機能を先に埋め、parse/cold reflow/warm scroll/幅変更/highlight更新、List filter/select/Table/Form を別 operation として source 完全出力/保持量と比較する。 |
| `matea-widgets/cursor` / reference | 21宣言リンクと stale-tick 改善はあるが、rich decorators と source の cursor appearance/terminal 契約全体は部分。 | virtual/native cursor の appearance と rich character decoration を別契約にし、focus/mode/blur/reduced-motion/重複通知を含む source fixture を追加する。 |
| `matea-widgets/cursor` / performance | 7 finite case で制御改善後も source より遅い。active配送、scale/rich style/RSSはない。 | 制御単体と文字所有/描画を分けて残コストを特定する。実timer/blur/mode変更/通知破棄と多数cursor、styled character、保持/解放を測り、stale-tick改善を維持する。 |
| `matea-widgets/spinner` / reference | 29宣言はリンク済みだが Model.Style/WithStyle の rich decorators が部分。Huh の action/accessibility/lifecycle は別の不足。 | spinner の frame を共通 decorator で描けるようにし、複数行・padding/border・frame差替えを検証する。Huh spinner を単なる label 結合で完了扱いしない。 |
| `matea-widgets/spinner` / performance | 14 case は借用frame/ignored tick。Go Style.Render と Rust frame借用の経路差を含み、active timer とdescriptorだけの比較は不公平。 | plain/label/styled view/constructor/custom差替え/active通知を分ける。source 実timerを含む等しい lifecycle、12preset/custom scale/reduced-motion/retained を測る。 |
| `matea-widgets/progress` / reference | 35宣言リンク済みでも一般 premultiplied/high-depth input と PercentageStyle geometry/MaxWidth/transform/hyperlink/whitespace が部分。 | 色入力の変換規約と decoration を共通 API で実装し、progress の幅配分・bar/percentage/gap の分離を source の raw ANSI と cell projection で検証する。 |
| `matea-widgets/textinput` / reference | editing/suggestions と presentation/lifecycle は別。後二者は partial で、clipboard/blink/装飾/default/style 条件を閉じていない。 | TextInput の純粋編集と runtime が所有する focus/blink/clipboard を分け、エラー・取消・echo/privacy・装飾と suggestion geometry を source 条件別に検証する。 |
| `matea-widgets/textinput` / performance | finite editing/suggestion fixtures があるが長文/Unicode境界/候補scale/validation/表示/retained を網羅しない。 | 短長ASCII/Unicode/combining/ZWJ、端/中央編集、limits、候補0/1/多・case expansion・validation、echo/focus/viewportの同意味操作をsourceと測り、再走査/割当を削減する。 |
| `matea/lib` / reference | reexport の追加処理がないことは性能証拠になるが、再公開先の全 feature/target/public API の同等性は別。 | facade の各再公開を compiler export と capability に結び、feature 組合せと独立利用例を検証する。名前/ライセンス/公開固定は別の最終 review とする。 |
| `examples` / performance | 12旧snapshotのprocess分布は現在の全examplesと操作中のlatency/RSSを覆わない。 | 現行全exampleの決定的snapshotとinteractive入力→表示/退出、resize、reduced motion、メモリ保持を測る。support/mod は呼ばれる経路で扱い、standalone架空時間を作らない。 |
| `tools/perf-probe` / performance | probeのrecord内operationを測っても計測器自身のtimer/startup/serialization/校正 overheadは未定量。reference-textも別経路。 | 空/既知コストcontrol、fixed/calibrated/output-only、CLI startup/record量/失敗の全維持モードを測る。reference-text API全経路とRSSを追加し、instrumented timingを除く。 |
| `tools/go-performance-probe` / performance | source fixtureが正しいこととprobe自身の overhead は別。MemStats bytes と System requested bytes の非同等性も残る。 | Go probe のtimer/loop/calibration/output-only/serialization/startup/RSSを独立 control で測る。言語間のallocation定義はそのまま明記し、無理に同一化/高速化しない。 |
| `scripts/compiler-export-audit` / performance | 20 retained validation+2 native extractは compiler/fetch/他profile scale/native RSSまで測っていない。 | collect/fetch/parse/extract/import の全維持経路を分離し、graph規模・失敗・native RSSを追加する。外部compiler/network時間とPythonアルゴリズムを同じ比率へ混ぜない。 |
| `scripts/coverage-and-issues` / performance | current ledger CLIと841 mergeの31標本は全reference audit・pagination/fetch・規模/native RSSではない。 | audit/scan/全ledger/merge/publishを測り、file数/graph数/issue数を増やした入出力一致、失敗、native RSSを追加する。822未分析issueはツール速度とは独立に処理する。 |
| `scripts/independent-terminal-oracles` / performance | 旧測定は5小fixtureのみ。行消去が画面全走査でscaleが悪く、不完全RGB/座標/遅延wrap/過大入力の試験も不足。bridgeの完全check/RSSも未測定。 | 独立flat cell回帰、bounded malformed入力、行/画面/文字消去の範囲処理へ改修する。3維持programの小/大/失敗/CLI/probe→publicationを同出力で旧新版測定し、native RSSを別passで記録する。 |
| `scripts/performance-reporters` / performance | 84case report/46compare/readerは他reporter・prepare・external pathとnative RSSを覆わない。 | 全維持reporterとprepare/journal/qualificationの代表成功/失敗/増加入力を実行し、完全rawhash・出力・時間/RSSを記録する。外部tool待機はalgorithmと別集計にする。 |
| `scripts/test-and-acceptance-drivers` / performance | 全3OS動作はverifiedだが性能測定はGit discovery/assessのみ。47suite、LLVM、PTY/native console、fault scale/RSSが未測定。 | 既存完全実行journalから壁時間を事実として扱い、専用の反復でorchestration/child時間/RSSを分ける。dry-run/helperだけで全driverを高評価にしない。 |
| `matea-ratatui/bridge` / reference | 77直接利用宣言の map は全 source module ではない。raw diff/skip/forced width と半角カナ width policy は明示 error のまま。 | 管理 scene へ取り込む契約と source の raw backend 契約を分け、危険な escape/width を黙って受理せず、不足の保持表現または独立 adapter を実装して全条件へ結ぶ。 |
| `matea-ratatui/bridge` / performance | full projection比較で2追加allocationと約40–52µsのimport overhead。render-only/varied/hostile/retained未測定。直接sourceは所有sceneを作らない。 | borrowed source bufferとimmutable sceneの責務を保ち、copy/ownership/cache境界をprofileする。別render-only、反復/変更scene、hostile/大grid、RSSを追加し、全projection/terminal bytesを維持する。 |
| `matea-terminal/device` / reference | quality-plan の『新規実装、qualification pending』は古い。native/CI は既に通過。ただし Plan9/他Unix、error/cancel、全 source 契約は残るため点数は50。 | 既存12 Unix/9 Windows 試験を繰り返すだけにせず、未検証 OS 条件・query/raw/password/error/cancel を宣言単位で分類し、追加の native/fault evidence を付ける。 |
| `matea-terminal/device` / performance | macOS10操作の201records/6231samplesはOS/error/cancel/full moduleではない。canonical long入力改善だけで短入力 adverseを消せない。 | 既存qualification pending文言を整理したうえで、実Windows/Linuxとquery/raw/password/error/cancel/短長入力を測る。OS syscall/待機/encoder/保持を分ける。 |

実行順は、①検証器の既知欠陥とscale/RSSの不足、②共通style/decorator契約と依存するHelp/Cursor/Spinner/Progress/TextInput、③runtimeのactive配送/取消と対応source workload、④全公開export/未実装moduleの閉包、⑤残るnative OS/transport・全driverの測定とする。依存のない項目はこの順序を理由に放置しない。各採用候補は元のadverse caseを保持し、改善が停滞したら原因仮説と比較境界を見直す。

device/bridgeの古いqualification pending記述と、clocksの『可変duration未測定』は実際の最新証拠と食い違う。ただしdevice/bridgeの残るsource条件やactive clockの不足は実在し、文言の訂正だけでは50→75にしない。

全体の別ブロッカーはraw 7,455中6,683未リンク、21部分/未測定capability、91 pending module、822未分析issue、2未解決Rust export graph、全依存/feature/target/signature閉包である。compiler 6,902キーはrawと重なるため加算しない。Stage B、改訂F5、画像エンジン/CUDAの順序を維持する。

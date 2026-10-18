# Matea Foundation gate report

状態: **F0–F4を継続中。F5へ未到達、承認依頼は保留。**
オーナーが全参照元・基盤ライブラリの公開機能、モジュールごとの性能比較と課題分析を追加したため、以前の小規模候補は受け入れ対象から外しました。

最新の評価範囲は111プログラム・41単位、未達49軸です。検証・タイマー修正の
[3 OS・MSRV・参照照合CI](https://github.com/n3dypop/Matea/actions/runs/39088967599)は7ジョブ成功ですが、
参照公開機能の全件実装と性能条件は未達です。[現在の証拠・残件](remaining-optimization.md)。

現検証ではUnix 33／Windows 32の全受入チェック、Python 103試験、Go 5試験、
3 OSでのLLVM収集とGo実計測モード91ケースを実行しました。
棚卸し公開のハッシュ整合性、失敗時に旧成功を残さないjournal、計測の固定単位数、
Windowsの短い標本と改行変換を修正しています。動作・移植性4軸を証拠により75へ更新し、
性能28・参照一致19・動作1・移植性1の未達を維持します。
[各OSのソース対応とartifact](evidence/measurement-drivers/ci-source-bindings.json)。
以下には以前の実装段階の結果も時系列の証拠として保持しています。

期限対応では固定16ms待機と、入力前の時間を新規購読へ加算する不具合をそれぞれ再現・修正しました。
最短期限に合わせた待機、購読なしの時計読み取り省略、入力前の時刻同期を実装しています。
ローカル33検証、Rust 148＋2 doctest、Python 103、13 snapshot、18 Unix PTY、
新規5テストのMiri 9実行とLLVM収集が成功しました。
1msのheadless通知条件は最終比較で旧21.58–24.02ms対新1.512–1.525msですが、
16ms条件やCPU対照には混在・悪化が残ります。性能軸は50を維持します。
[全分布と制約](evidence/deadlines/report.json)、
[ADR 0039](decisions/0039-deadline-aware-terminal-polling.md)、
[ADR 0040](decisions/0040-align-logical-time-before-input-dispatch.md)。
[各OSの最終program/build/check入力・ログ・LLVM・参照照合の独立検査](evidence/deadlines/ci-source-bindings.json)
でも、全107 programを同じCI commitへ対応付けています。

文字装飾の拡張では、型付き属性・6種の下線・下線色・blink抑制を追加し、
13,824状態を構築・解除・参照SGRと照合しました。セルは40 byteを維持し、
色だけをキーとするキャッシュで装飾の相互追い出しを除去しています。
ローカル33検証、Rust 155＋2 doctest、Python 105、14 snapshot、19 Unix PTY、
6 targeted Miri、LLVM 62 source／67 instanceが成功しました。
[監査と悪化を含む性能記録](text-attributes-audit.md)、
[ADR 0041](decisions/0041-compact-resolved-text-attributes.md)。
4d1a397の[3 OS CI](https://github.com/n3dypop/Matea/actions/runs/39091736105)は7必須ジョブ成功です。
[全111 program／127入力・各OSの17ログ・LLVM・参照照合の独立検査](evidence/attributes/ci-source-bindings.json)も完了しました。
属性の38宣言候補を部分対応へ連結しましたが、6,896候補、13部分能力、91 module group、
参照一致・性能などの49軸は未達で、Stage B・F5・画像エンジン／CUDAへ進みません。

## 現在の範囲と進捗

全実装の再評価として、86プログラムファイルを40評価単位へ登録し、Rust設計・メモリ安全性・動作・参照元一致・性能・移植性の6軸を採点しました。[評価項目と判定基準](quality-evaluation.md)、[ファイル別の証拠・未達項目](evidence/quality/report.json)。未測定を高評価とせず、平均点で不合格を隠しません。通常CIの台帳整合性チェックと、全軸75点以上を要求する`check_quality.py --gate`は別です。全高評価には未到達です。

この評価でViewportの末尾空行・CRLF・リサイズ位置・無視入力の不整合を修正し、表示時の行ごとの複製を除去しました。12件のRust内前後測定では、ASCII 24行のView生成が26割り当てから1へ減少しました。初期測定のp95悪化も保存し、構築・描画込み・初期解析／常駐メモリ未測定を分けています。Rustの棚卸しツールではフィールド等の子`cfg`欠落を修正し、Crossterm 80件とRatatui 42件の条件を補正しました。宣言ID・件数は維持しています。Python `-O`が検証を省略する不具合も再現し、14スクリプトの138判定を明示検査へ変更しました。[ADR 0023](decisions/0023-indexed-viewport-and-evidence-based-quality-gates.md)。

追加試験は1,296レイアウト組合せ、4,096連続描画の所有セル・差分不変条件、Kitty 20入力の分割・復元・出力上限、Sixel 54入力のバンド・透過・run、Viewportの独立行モデルを含みます。Miriは描画・購読・Textの合計9テストに成功しました。[正確な対象・seed・制約](evidence/quality/miri-context.json)。計測用ビルドでは51 Rustソースファイルの行・関数実行を記録しましたが、埋め込み単体テストを含む値であり、分岐・参照機能の網羅率や安全性の証明ではありません。[実行率](evidence/quality/rust-execution-coverage.json)。

- 固定SHAの11参照単位を走査し、94モジュール、7,455件の宣言候補と117件のソース課題マーカーを記録。[台帳](coverage/summary.json)。マクロ展開・再公開・cfgを解決した正確な公開API一覧は未完成です。
- 公開機能の対応付けは500候補までで、6,955候補は未対応付けです。実装済みと部分対応を区別しています。[能力台帳](coverage/capabilities.json)、[手順](coverage-plan.md)。
- 841件のopen issueを収集しました。未確認報告を不具合と断定していません。[一覧](coverage/upstream-issues.json)、[初期分析と回帰対策](upstream-defects.md)。全件の分析・再現は未完了です。
- 7 crateにRuntime、描画、スタイル、基本widget、端末、facade、独立motionを配置。Harmonica相当のバネ・投射体、カスタムfilter/keymap、ページ分割を追加しました。公開APIとcrate名は暫定です。
- 描画の割り当てを削減し、84ケースを固定参照元と比較。出力fixtureは一致しましたが、全モジュールの同等性能は未達です。1セル更新の通信量は当初37対2バイトから、状態付きエンコーダにより双方2バイトへ改善しました。バネは参照元と同じIEEE演算のsampleと、双方で入力・結果を検査するchecked updateを分けて測定しています。キー照合は型付き比較で文字列化を省き、6ケースで検証しました。[測定条件と生データ](performance.md)。
- キーのPress/Repeat/Release、6修飾子、ロック状態、機能キー、マウスを公開型で保持。widgetのRelease誤操作、修飾Tabによるフォーカス移動、無視イベントによるForm状態消失を修正しました。端末モードを個別管理し、復旧リトライでキーボードスタックを二重popしないよう改善しました。自動能力交渉、物理キー、IME、WindowsのKitty要求などは残課題です。[入力監査](input-audit.md)、[ADR 0011](decisions/0011-input-events-and-mode-leases.md)。
- オーナーの例外承認により、Kitty/Sixelの純粋なエンコーダをFoundationへ追加。画像エンジン、画像デコード・変換、CUDAは未実装です。[ADR 0009](decisions/0009-foundation-protocol-encoders.md)。実端末、完全な公開機能、Runtimeの自動能力交渉・資源管理は未検証／未実装です。

Harmonica（20宣言）とBubbles paginator（29宣言）は、文書化したRustの契約に沿った実装・正しさ・性能fixtureのレビューを完了しました。91モジュールは未完了です。投射体のraw/checked更新、ページ状態・キー操作・書式・表示予算を実装し、参照元のゼロ除算・古いslice範囲・空件数時の挙動も実行して分析しました。普遍的な性能互換の主張ではありません。[モジュール監査](motion-pagination-audit.md)、[変更前後の生データ](evidence/performance/2026-10-16/motion-pagination-before-report.json)。

TextInputでは77宣言を対応付け、grapheme編集・単語操作・補完・検証・表示を拡張しました。固定参照元でAlt+Dのpanic／過剰削除、上限を超える補完、空候補の参照panic、placeholderのNUL混入を再現し、回帰対策を追加しました。入力8ケースの出力一致を確認し、ASCII候補検索バッファの再利用で測定した割り当てを2回から0回へ削減しました。cursor appearance・blink・OS clipboard・light defaultなどは部分対応のままです。[入力編集監査](input-editing-audit.md)、[ADR 0013](decisions/0013-grapheme-input-and-completion.md)。

native cursorの位置と表示状態を追加しました。widgetの局所座標をレイアウトで合成し、日本語の桁幅、スクロール、重なりとクリッピングを扱います。Bubbles #906／#1001の固定参照元での位置ずれを再現し、Mateaの回帰テストとUnix PTYを追加しました。既知位置の同軸移動は測定fixtureで6→4バイト、割り当て0回です。独立再生で画面・位置・可視性を検査しています。pure encoderから条件付きWinAPI実行経路も除去しました。[ADR 0014](decisions/0014-composed-cursor-and-pure-encoding.md)。実端末の見え方やWindows native consoleの保証ではありません。

## 検証

Rustの公開API監査をコンパイラベースで拡張しました。固定SHA／lockfileを隔離コピーし、3 OSの20構成をコンパイル済みです。マクロ生成、再公開、戻り値経由の公開型とメソッドを追跡し、6,902件のitem／implレビューキーを記録しました。すべて未対応付けで、元の7,455宣言候補とは重複します。全platform/featureと外部署名依存の閉包は未解決です。監査ロジック4テストとLF/CRLF取り込みの回帰テストも成功。[監査](rust-public-api-audit.md)、[ADR 0016](decisions/0016-compiler-api-coverage-before-renderer-integration.md)。

Bubbles keyの17宣言もレビューし、3モジュール・66宣言分の実装／正しさ／有限性能fixtureを確認済みとしました。Bindingを型付きキーに加えて独自トークンにも対応させ、借用比較・空集合時の無効化・ヘルプの一貫性を実装しています。短いメタデータのインライン化で構築時の割り当てを3→1回、測定p50を76.47→29.43nsへ削減しました。shared／distinct storageを含む12比較ケースは出力一致。全モジュールの普遍的な同等性能を示すものではありません。[監査](key-bindings-audit.md)、[ADR 0015](decisions/0015-generic-shortcut-bindings.md)。

ローカル検証の最新の実コマンド・終了コードは[local-validation.json](evidence/local-validation.json)と各ログに保存します。`python3 scripts/verify_foundation.py --offline`で再実行できます。

今回のローカル結果は120 testsと2 doc tests（compile-only／compile-fail）成功、12サンプルsnapshot成功、Unix PTY 17ケース成功です。23検証コマンドが成功し、fmt、clippy（-D warnings）、all-targets、release、rustdocと、Python 16ケースの通常／最適化実行を含みます。端末差分の独立再生では、初回出力97対95バイト、更新2対2バイトで同じ画面を確認しました。初回は可視性が不明なためHideを明示します。別のcursor-only traceは移動4対6バイトで同じ位置・可視性です。

色出力では、明示した能力が依存のNO_COLOR設定に上書きされるMateaの統合不具合を修正しました。4色モードを独立再生し、固定サイズの量子化キャッシュ、属性・行移動の出力改善を検証。ANSI256の全画面fixtureはp50 1.58ms→28.78µs、測定割り当て2→0回です。行移動のCRLF短縮には、既存のスクロール領域内で画面を動かす不具合があり、失敗traceを保存して絶対指定へ修正しました。現在の無彩色出力は1,745→1,704バイトで、参照元の1,655バイトより49バイト多くなります。途中の性能後退と、標準／独自スクロール領域の再生検証も記録。任意のパレット・キャッシュ分布・実端末の性能は未検証です。[監査](frame-color-audit.md)、[ADR 0017](decisions/0017-frame-color-policy-and-bounded-quantization.md)。

キー操作ヘルプを追加し、27宣言を対応付けました。短縮／複数列表示、無効bindingの除外、独立した余白スタイルとセル幅制限を実装。参照元の狭い幅でのはみ出しを再現し、#571の無装飾スペースを改善しました。平文4ケースは出力一致、割り当ては短縮5／4→1回、複数列9→5回です。Unicodeの安全性と描画器の幅に合わせた経路は従来の平文処理より遅くなったため、その測定も保存しました。Lip Glossの任意の装飾・折り返し等は未対応で、helpモジュール全体は部分対応のままです。[監査](key-help-audit.md)、[ADR 0018](decisions/0018-borrowed-and-width-bounded-key-help.md)。

stopwatch／countdownの43宣言を対応付けました。実際の単調経過時間、正のtick間隔、世代付きschedule、部分経過を保持するpause、ゼロに止める一度限りのtimeoutを実装。固定参照元のゼロ既定値、stale tick、重複start、負のtimeout表示を実行して分析しました。8件の時計テスト、269件の書式fixture、2回の満了と再開を含むUnix PTYが成功。RuntimeのDuration::MAXでは、表現できない次回期限をNoneにして同一期限の無限再通知を防ぐ回帰テストも追加しています。[監査](clocks-audit.md)、[ADR 0019](decisions/0019-monotonic-clocks-and-generation-safe-schedules.md)。

時計実装時点では16比較ケースを追加し、計70ケースの出力が一致しました。owned書式化の最大共有durationは3回／56バイトから1回／24バイト、p50 148.17→54.42nsへ改善しましたが、参照元32.08nsには未達です。呼び出し側バッファ再利用は双方0割り当て。停止中／別IDのtickの4比較は双方0割り当てです。active schedulingはGo側の実timer作成と同じ意味の測定が未実施で、遅延通知・購読作成を含む性能はpendingです。時計2モジュールを性能同等と扱わず、現在の台帳の部分対応／未測定能力は11件、未分析issue報告は824件です。

Spinnerの29宣言を対応付け、12プリセット・66フレーム、検証済みカスタムフレーム、共有データ、世代付き更新と単調時間による位相を実装しました。初回zero-tag tickの再送による追加回転、空／短縮フレームのerror表示、ゼロ周期の受理を参照元で実行して分析。Mateaは重複・古いtickを無視し、設定変更で位相をリセットします。5件の統合テストと、12プリセットのサンプル・操作PTYが成功しました。ラベルなし文字列はCowで借用し、14件の有限比較で双方割り当て0回。active schedulingと任意のLip Gloss装飾は未完了です。[監査](spinner-audit.md)、[ADR 0020](decisions/0020-validated-spinner-animations.md)。

購読の再構築をRust側で追加測定し、型付き名前空間を持つSubscriptionIdとtimer表の再利用を導入しました。IDは表のキーへ移動し、Timer内で二重所有しません。64件維持の測定は割り当て203→65回、要求バイト20,992→9,216、p50 14.11→7.77µsです。途中で購読0件の遅延が20.96→24.96nsへ増えた測定も保存し、空設定の経路で19.33nsとなりました。5件はRust内の比較で、上記84件の参照元比較とは別です。通知遅延や初期化時の保持メモリは未測定で、callback／Vecの割り当ては残ります。[測定](subscription-performance.md)、[ADR 0021](decisions/0021-scoped-subscription-identities-and-retained-timers.md)。

同時に、重複IDを拒否する途中で既存timerを失うMateaの不具合を実行して再現しました。設定全体の検証後に表を更新し、元の期限を保持します。既知の失敗ログと3件の回帰テストで、重複・ゼロ周期・件数超過、名前空間とrevision、既存callback保持を検証。モデル状態・effect全体を巻き戻す保証ではありません。

時計の文字列化は、安全な固定バッファへ逆順に直接桁を書き込み、一時配列とコピーを減らしました。269件の参照元出力とRust Duration::MAXを維持。短い測定は未変更の経路にも揺れが大きかったため、旧／新版のバイナリを分離し、両言語の全16ケースを20倍のバッチで再測定しました。最大共有durationのowned書式化はp50 54.97→49.05nsですが、Go 32.50nsには未達です。短い結果も保持し、時計全体の性能判定はpendingのままです。これらは既存84ケース中の16ケースの再測定です。[詳細](clock-format-performance.md)。

Viewの本文／見出しを、短い値をinline保持できる不変のTextへ変更しました。標準文字列とformat_args!から直ちに所有値を作り、制御文字を一度だけ正規化します。公開Canvas.textは任意文字列を引き続き検査。カウンターと独立widgetで中間Stringを省き、全Unicodeスカラー値の検証、複数サイズ・focus/themeのセル比較、formatter失敗時のPTY復旧を追加しました。[APIと互換性上の変更](text-api.md)、[ADR 0022](decisions/0022-owned-normalized-scene-text.md)。コンストラクタ境界は暫定のInto<Text>となり、独自Into<String>だけを実装する型は明示変換が必要です。

Rust内20比較では短いViewの構築・複製・数値書式化が1→0割り当て、短いpanelが4→2回となりました。正規化を構築へ移したため、長いASCIIの構築は28.02→86.01ns、制御文字入りは24.38→101.49nsへ遅くなる結果も保持。構築＋描画と複製＋描画を別に測定し、1フレーム全体を無割り当てとは扱いません。既存84ケースも新コードで再実行して出力一致を確認しました。追加20件はRust内比較で、参照元との84件とは区別します。[正しさ・前後・途中の性能](view-performance.md)。

`python3 scripts/check_coverage.py`は台帳の参照整合性を検査します。`--gate`を付けた厳格検査は、未対応付け、部分対応、未測定、未解決のRust公開exportがある間は失敗します。通常CIの成功をF5通過と扱いません。

初回候補 `48ea6e978d69838f9b6a55c460368dba2d223f2c` は[Linux/Windows/macOSとMSRV 1.88のCI](https://github.com/n3dypop/Matea/actions/runs/38811891109)に成功しました。これは旧スコープの履歴です。前回の実装コミット `8697d5454a39512f4b76df153dca87e7127f4279` も[3 OS + MSRV 1.88のCI](https://github.com/n3dypop/Matea/actions/runs/38819360615)に成功しました。[GitHubから取得したjob/step証拠](evidence/coverage-ci-run.json)を保存しています。WindowsのPTYは対象外です。

入力・端末モード改善 `35de86601a845736c7227616a15bc4bb496210d8` は[Linux/Windows/macOS + MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/38823502200)に成功しました。[ジョブ・step証拠](evidence/input-ci-run.json)を保存しています。Linux/macOSのPTYも成功、WindowsのPTYはskipです。

motion・paginator改善 `9ce1b4ff51405a5f5114f444729c057ad1a356ca` は[3 OS + Rust 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/38826613401)に成功しました。[ジョブ証拠](evidence/motion-pagination-ci-run.json)。入力編集 `3fe410b44be6bc980cf6e6893b223e5c86dd5ba1` も[3 OS + MSRV CI](https://github.com/n3dypop/Matea/actions/runs/38830825900)成功。[ジョブ証拠](evidence/textinput-ci-run.json)。native cursor `448d2f7be3d5455a238d4d633b1b12ff037a1706` も[3 OS + MSRV CI](https://github.com/n3dypop/Matea/actions/runs/38845394481)成功。[ジョブ証拠](evidence/cursor-ci-run.json)。key `cea1ddbec0fddb46ea4630b3d8b0b9a0caec011a` も[3 OS + MSRV CI](https://github.com/n3dypop/Matea/actions/runs/38846267433)成功。[ジョブ証拠](evidence/key-ci-run.json)。

## F5の前に残る作業

公開API収集の初回[3 OS CI](https://github.com/n3dypop/Matea/actions/runs/38850278693)では参照元6ジョブが成功しました。[結果](evidence/compiler-audit-ci-run.json)。本体Windows検証は監査JSONのCRLF変換によるhash不一致で失敗し、LF固定・取り込み正規化・回帰テストを追加しました。

修正コミット `a6d3b81d1d10a9f773e79a2c67f2edbc19116b12` の[再実行](https://github.com/n3dypop/Matea/actions/runs/38851892735)は参照元6・本体3 OS・MSRVの計10ジョブ成功。[証拠](evidence/compiler-audit-fixed-ci-run.json)。現在の20構成・6,902レビューキーは未対応付けで、全feature／backend／依存閉包を示すものではありません。

色出力 `f9c4b0c991d36d4a26fe012965367441078db1db` は[3 OS + MSRV CI](https://github.com/n3dypop/Matea/actions/runs/38854767940)成功。[証拠](evidence/color-ci-run.json)。ヘルプ `456308057196036ce8106823273da6bae27f7c2f` も[3 OS + MSRV CI](https://github.com/n3dypop/Matea/actions/runs/38856048761)成功。[証拠](evidence/help-ci-run.json)。

最新のスクロール領域の追補修正 `80b3570e18e8ed3849006e80ecac4b30bfcb35a0` は[Linux・Windows・macOS + MSRV CI](https://github.com/n3dypop/Matea/actions/runs/38856755912)に成功しました。[ジョブ・step証拠](evidence/margin-ci-run.json)。独立再生oracleの3回帰テストと、独自スクロール領域での初回・復旧出力も通常CIへ含めています。参照元再収集ジョブはこの通常runではskipで、上記の専用runが証拠です。

時計実装 `783b1e77f489fdd7a47e103c9c0df2d6975ef0be` は[Linux・Windows・macOS + MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39010247644)の4ジョブに成功しました。[ジョブ・step証拠](evidence/clocks-ci-run.json)。Unix PTYはLinux/macOSで実行し、Windowsではskipです。参照元再収集はこの通常runではskipです。

Spinner実装 `8ee28f9cf9e0ecec750bbd516937a1d14ceeb5b4` は[Linux・Windows・macOS + MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39010918504)の4ジョブに成功しました。[ジョブ・step証拠](evidence/spinner-ci-run.json)。Unix PTYはLinux/macOSで実行、Windowsはskipです。通常runの参照元再収集もskipで、専用の20構成収集結果とは区別します。

購読改善 `fcf97f1a6e3065d8e01e4f6044191114a4f9c7d0` は[Linux・Windows・macOS + MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39011713186)の4ジョブに成功しました。[ジョブ・step証拠](evidence/subscriptions-ci-run.json)。WindowsのPTYと通常runの参照元再収集はskipです。

逆順時計書式化 `b6dfcca442e1a4bc810cc8655065065c81d232ac` は[Linux・Windows・macOS + MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39012460120)の4ジョブに成功しました。[ジョブ・step証拠](evidence/clock-format-ci-run.json)。WindowsのPTY、通常runの参照元再収集はskipです。

Text実装 `e91b0a5b9a03e4460f48dbc276a5ee8ed061861a` は[Linux・Windows・macOS + MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39014877580)の4ジョブに成功しました。[ジョブ・step証拠](evidence/text-ci-run.json)。Unicode全スカラー値、formatter error、独立widgetとサイズ／style検証を含みます。Unix PTYはLinux/macOSで実行し、Windowsはskip。通常runの参照元再収集もskipです。

1. 全宣言から利用者機能への対応付け、Rustマクロ・再公開・platform/feature条件と基盤依存の範囲（128宣言辺）の解決、全機能の実装と正しさの証拠。
2. 全モジュールの条件を揃えた性能検証、課題分析、遅い経路・出力量の改善と再測定。現測定は一台のmacOS arm64、メモリsinkであり、実端末応答の検証ではありません。
3. Runtime拡張、style/layout、Bubbles/Huh/Glamour、Ratatui/Crossterm/Ultraviolet/xの残機能。依存の追加や再公開だけで対応完了としません。
4. 残機能を実装した際の3 OS CI再検証と、Windows native console、実端末のUnicode/IME/SSH/tmux、Kitty/Sixelの端末別検証。OS対応はプロトコル対応を意味しません。
5. 公開API・ライセンス・crate名の最終レビュー。全packageはpublish=falseで、Mateaの配布ライセンス、crates.io／商標の可用性は未確定です。
6. 協調しないユーザーworkerはRuntime終了後も残り得ます。キャンセル・復旧保証の範囲は[アーキテクチャ](architecture.md)に記録しています。

全公開機能と性能要件を満たした後にF5安定化結果と未解決事項を改めて報告し、次フェーズへ進む前に明示承認を求めます。現時点では承認を求めません。

GitルートはMatea/、remoteは[n3dypop/Matea](https://github.com/n3dypop/Matea)。Reference、AGENTS、CODEX_START、Skillsはリポジトリ外に保っています。

全プログラム評価・Viewport・cfg／検証ツール修正の実装 `9ce5564ad525d047594c5e1debecabd7c3bb4e37` は[Linux・Windows・macOS + MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39043211231)の4ジョブに成功しました。[job／step証拠](evidence/quality/ci-run.json)。初回runではmacOSのGo fixtureが1件skipされたため、公式setup-goの固定SHA・Go 1.26.0導入と必須実行へ変更して再実行しました。同時点の品質評価は81ファイル・40単位で、性能28、参照元一致18、動作5、外部ツール移植性5の計56評価軸が75未満です。これは56件の確認済みバグという意味ではなく、未測定・部分対応・未検証を含みます。品質ゲートとF5は閉じたままです。

個別最適化では127ケースの測定を追加し、色変換、フォーカス初期化、純粋なKitty／Sixelエンコーダ、宣言収集CLIを改善しました。Unicode非対応時の全角セルの背景欠落も回帰試験で修正しました。[生データ・改善・不利な結果・残件](individual-optimization.md)、[ADR 0024–0027](decisions/0024-exact-bounded-color-quantization.md)。追加11件の対象限定Miriに成功し、従来分と合わせて20件です。[追加の対象と制約](evidence/optimization/miri-context.json)。ローカル25検証、Rust 129テスト＋2 doctest、明示実行の全RGB検証、Python 24テスト、12 snapshot、17 Unix PTYに成功しました。既存84ケースの参照元比較も全fixtureの出力・意味が一致しました。86ファイル・40単位にはなお56評価軸の未達があり、全項目高評価・横断最適化への移行・F5には未到達です。

個別最適化の実装 `203ba5d7c56f3588a2dadb14410e3aafe3ead59f` は
[Linux・Windows・macOS + MSRV 1.88 CI](https://github.com/n3dypop/Matea/actions/runs/39049417573)の4ジョブ成功です。
各OSのGo収集2テスト、最適化Python 24テスト、46／37／41の追加CLIケースをログで確認しました。
[ジョブ・stepと除外範囲](evidence/optimization/ci-run.json)。品質ゲート56未達とF5閉鎖は維持します。


### 未測定12単位への追補

[追加78件の測定と修正](remaining-optimization.md)を実施しました。
完全未測定の性能単位は0ですが、品質の未達は51軸で、参照機能の全件実装・一致も
未完了です。Foundation F5、横断最適化Stage B、画像エンジン／CUDAのgateは閉じています。
新しいCIでは通常3 OS/MSRVに加え、任意実行の6 compiler profile jobsと3 source/probe jobsで
取込、全11 source inventories、84比較出力、補助dispatch、全5報告CLI、issue API経路を
確認します。CIの時間値を新しい性能結果には使用しません。

100プログラム・40単位の未達内訳は性能27・参照元一致18・動作3・移植性3です。
[最新の3 OS／MSRV通常CI](https://github.com/n3dypop/Matea/actions/runs/39069122055)は4ジョブ成功。
[参照専用run](https://github.com/n3dypop/Matea/actions/runs/39068619681)の9参照ジョブも成功し、
全20 compiler graphの一致を確認しました。この専用runの本体3 OSは測定reportの未同梱で
失敗しており、同梱後の前者で修正を確認しています。単一runの13/13成功という意味ではありません。
[sourceとの対応と制約](evidence/optimization/ci-source-bindings.json)。

ローカル30検証、Rust 137＋2 doctest、Python 72、全RGB、12 snapshot、17 Unix PTYに成功。
新しい7 Miri実行はtarget限定で、全実行順序やFFIの証明ではありません。
[改善・全分布・不利な結果・未達](remaining-optimization.md)、ADR 0028–0034へ記録しました。
参照全件実装・性能条件は未達で、Stage B・F5・画像エンジン／CUDAへは進めていません。

仮想カーソルでは21宣言を対応付け、点滅・モード・focus・文字入力合成とinline表示を実装しました。
新規6統合試験、最終6 Miri、ローカル30検証、Rust 143＋2 doctest、Python 72、全RGB、
13 snapshot、18 Unix PTYが成功。LLVMは59 source／64 compiler instanceを区別しています。
[ADR 0035](decisions/0035-owned-virtual-cursor-and-expiring-notifications.md)、
[全分布と失敗を含む証拠](evidence/cursor/performance/report.json)、
[CI source対応](evidence/cursor/ci-source-bindings.json)を記録しました。
新規独立単位の性能・参照一致は50のため、現在の未達は53軸です。
参照候補6,934件とcompiler key 6,902件（重複あり）の対応付け、12部分capability、
91 module groupなどが残り、全体最適化・F5への移行承認を求める段階ではありません。

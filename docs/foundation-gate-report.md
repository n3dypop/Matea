# Matea Foundation gate report

状態: **F0–F4を継続中。F5へ未到達、承認依頼は保留。**
オーナーが全参照元・基盤ライブラリの公開機能、モジュールごとの性能比較と課題分析を追加したため、以前の小規模候補は受け入れ対象から外しました。

## 現在の範囲と進捗

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

今回のローカル結果は106 testsと1 compile-only doc test成功、12サンプルsnapshot成功、Unix PTY 16ケース成功です。fmt、clippy（-D warnings）、all-targets、rustdocも成功しました。端末差分の独立再生では、初回出力97対95バイト、更新2対2バイトで同じ画面を確認しました。初回は可視性が不明なためHideを明示します。別のcursor-only traceは移動4対6バイトで同じ位置・可視性です。

色出力では、明示した能力が依存のNO_COLOR設定に上書きされるMateaの統合不具合を修正しました。4色モードを独立再生し、固定サイズの量子化キャッシュ、属性・行移動の出力改善を検証。ANSI256の全画面fixtureはp50 1.58ms→28.78µs、測定割り当て2→0回です。行移動のCRLF短縮には、既存のスクロール領域内で画面を動かす不具合があり、失敗traceを保存して絶対指定へ修正しました。現在の無彩色出力は1,745→1,704バイトで、参照元の1,655バイトより49バイト多くなります。途中の性能後退と、標準／独自スクロール領域の再生検証も記録。任意のパレット・キャッシュ分布・実端末の性能は未検証です。[監査](frame-color-audit.md)、[ADR 0017](decisions/0017-frame-color-policy-and-bounded-quantization.md)。

キー操作ヘルプを追加し、27宣言を対応付けました。短縮／複数列表示、無効bindingの除外、独立した余白スタイルとセル幅制限を実装。参照元の狭い幅でのはみ出しを再現し、#571の無装飾スペースを改善しました。平文4ケースは出力一致、割り当ては短縮5／4→1回、複数列9→5回です。Unicodeの安全性と描画器の幅に合わせた経路は従来の平文処理より遅くなったため、その測定も保存しました。Lip Glossの任意の装飾・折り返し等は未対応で、helpモジュール全体は部分対応のままです。[監査](key-help-audit.md)、[ADR 0018](decisions/0018-borrowed-and-width-bounded-key-help.md)。

stopwatch／countdownの43宣言を対応付けました。実際の単調経過時間、正のtick間隔、世代付きschedule、部分経過を保持するpause、ゼロに止める一度限りのtimeoutを実装。固定参照元のゼロ既定値、stale tick、重複start、負のtimeout表示を実行して分析しました。8件の時計テスト、269件の書式fixture、2回の満了と再開を含むUnix PTYが成功。RuntimeのDuration::MAXでは、表現できない次回期限をNoneにして同一期限の無限再通知を防ぐ回帰テストも追加しています。[監査](clocks-audit.md)、[ADR 0019](decisions/0019-monotonic-clocks-and-generation-safe-schedules.md)。

時計実装時点では16比較ケースを追加し、計70ケースの出力が一致しました。owned書式化の最大共有durationは3回／56バイトから1回／24バイト、p50 148.17→54.42nsへ改善しましたが、参照元32.08nsには未達です。呼び出し側バッファ再利用は双方0割り当て。停止中／別IDのtickの4比較は双方0割り当てです。active schedulingはGo側の実timer作成と同じ意味の測定が未実施で、遅延通知・購読作成を含む性能はpendingです。時計2モジュールを性能同等と扱わず、現在の台帳の部分対応／未測定能力は11件、未分析issue報告は824件です。

Spinnerの29宣言を対応付け、12プリセット・66フレーム、検証済みカスタムフレーム、共有データ、世代付き更新と単調時間による位相を実装しました。初回zero-tag tickの再送による追加回転、空／短縮フレームのerror表示、ゼロ周期の受理を参照元で実行して分析。Mateaは重複・古いtickを無視し、設定変更で位相をリセットします。5件の統合テストと、12プリセットのサンプル・操作PTYが成功しました。ラベルなし文字列はCowで借用し、14件の有限比較で双方割り当て0回。active schedulingと任意のLip Gloss装飾は未完了です。[監査](spinner-audit.md)、[ADR 0020](decisions/0020-validated-spinner-animations.md)。

購読の再構築をRust側で追加測定し、型付き名前空間を持つSubscriptionIdとtimer表の再利用を導入しました。IDは表のキーへ移動し、Timer内で二重所有しません。64件維持の測定は割り当て203→65回、要求バイト20,992→9,216、p50 14.11→7.77µsです。途中で購読0件の遅延が20.96→24.96nsへ増えた測定も保存し、空設定の経路で19.33nsとなりました。5件はRust内の比較で、上記84件の参照元比較とは別です。通知遅延や初期化時の保持メモリは未測定で、callback／Vecの割り当ては残ります。[測定](subscription-performance.md)、[ADR 0021](decisions/0021-scoped-subscription-identities-and-retained-timers.md)。

同時に、重複IDを拒否する途中で既存timerを失うMateaの不具合を実行して再現しました。設定全体の検証後に表を更新し、元の期限を保持します。既知の失敗ログと3件の回帰テストで、重複・ゼロ周期・件数超過、名前空間とrevision、既存callback保持を検証。モデル状態・effect全体を巻き戻す保証ではありません。

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

購読改善の3 OS + MSRV CIは送信前です。直前のSpinner成功とは実装コミットを分けて記録します。

1. 全宣言から利用者機能への対応付け、Rustマクロ・再公開・platform/feature条件と基盤依存の範囲（128宣言辺）の解決、全機能の実装と正しさの証拠。
2. 全モジュールの条件を揃えた性能検証、課題分析、遅い経路・出力量の改善と再測定。現測定は一台のmacOS arm64、メモリsinkであり、実端末応答の検証ではありません。
3. Runtime拡張、style/layout、Bubbles/Huh/Glamour、Ratatui/Crossterm/Ultraviolet/xの残機能。依存の追加や再公開だけで対応完了としません。
4. 残機能を実装した際の3 OS CI再検証と、Windows native console、実端末のUnicode/IME/SSH/tmux、Kitty/Sixelの端末別検証。OS対応はプロトコル対応を意味しません。
5. 公開API・ライセンス・crate名の最終レビュー。全packageはpublish=falseで、Mateaの配布ライセンス、crates.io／商標の可用性は未確定です。
6. 協調しないユーザーworkerはRuntime終了後も残り得ます。キャンセル・復旧保証の範囲は[アーキテクチャ](architecture.md)に記録しています。

全公開機能と性能要件を満たした後にF5安定化結果と未解決事項を改めて報告し、次フェーズへ進む前に明示承認を求めます。現時点では承認を求めません。

GitルートはMatea/、remoteは[n3dypop/Matea](https://github.com/n3dypop/Matea)。Reference、AGENTS、CODEX_START、Skillsはリポジトリ外に保っています。

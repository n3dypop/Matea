# Matea Foundation gate report

状態: **F0–F4を継続中。F5へ未到達、承認依頼は保留。**
オーナーが全参照元・基盤ライブラリの公開機能、モジュールごとの性能比較と課題分析を追加したため、以前の小規模候補は受け入れ対象から外しました。

## 現在の範囲と進捗

- 固定SHAの11参照単位を走査し、94モジュール、7,455件の宣言候補と117件のソース課題マーカーを記録。[台帳](coverage/summary.json)。マクロ展開・再公開・cfgを解決した正確な公開API一覧は未完成です。
- 公開機能の対応付けは310候補までで、7,145候補は未対応付けです。実装済みと部分対応を区別しています。[能力台帳](coverage/capabilities.json)、[手順](coverage-plan.md)。
- 841件のopen issueを収集しました。未確認報告を不具合と断定していません。[一覧](coverage/upstream-issues.json)、[初期分析と回帰対策](upstream-defects.md)。全件の分析・再現は未完了です。
- 7 crateにRuntime、描画、スタイル、基本widget、端末、facade、独立motionを配置。Harmonica相当のバネ・投射体、カスタムfilter/keymap、ページ分割を追加しました。公開APIとcrate名は暫定です。
- 描画の割り当てを削減し、21ケースを固定参照元と比較。出力fixtureは一致しましたが、全モジュールの同等性能は未達です。1セル更新の通信量は当初37対2バイトから、状態付きエンコーダにより双方2バイトへ改善しました。バネは参照元と同じIEEE演算のsampleと、双方で入力・結果を検査するchecked updateを分けて測定しています。キー照合は型付き比較で文字列化を省き、6ケースで検証しました。[測定条件と生データ](performance.md)。
- キーのPress/Repeat/Release、6修飾子、ロック状態、機能キー、マウスを公開型で保持。widgetのRelease誤操作、修飾Tabによるフォーカス移動、無視イベントによるForm状態消失を修正しました。端末モードを個別管理し、復旧リトライでキーボードスタックを二重popしないよう改善しました。自動能力交渉、物理キー、IME、WindowsのKitty要求などは残課題です。[入力監査](input-audit.md)、[ADR 0011](decisions/0011-input-events-and-mode-leases.md)。
- オーナーの例外承認により、Kitty/Sixelの純粋なエンコーダをFoundationへ追加。画像エンジン、画像デコード・変換、CUDAは未実装です。[ADR 0009](decisions/0009-foundation-protocol-encoders.md)。実端末、完全な公開機能、Runtimeの自動能力交渉・資源管理は未検証／未実装です。

## 検証

ローカル検証の最新の実コマンド・終了コードは[local-validation.json](evidence/local-validation.json)と各ログに保存します。`python3 scripts/verify_foundation.py --offline`で再実行できます。

今回のローカル結果は51 testsと1 compile-only doc test成功、8サンプルsnapshot成功、Unix PTY 10ケース成功です。fmt、clippy（-D warnings）、all-targets、rustdocも成功しました。端末差分の独立再生では、初回出力94対95バイト、更新2対2バイトで同じ画面を確認しました。

`python3 scripts/check_coverage.py`は台帳の参照整合性を検査します。`--gate`を付けた厳格検査は、未対応付け、部分対応、未測定、未解決のRust公開exportがある間は失敗します。通常CIの成功をF5通過と扱いません。

初回候補 `48ea6e978d69838f9b6a55c460368dba2d223f2c` は[Linux/Windows/macOSとMSRV 1.88のCI](https://github.com/n3dypop/Matea/actions/runs/38811891109)に成功しました。これは旧スコープの履歴です。今回の実装コミット `8697d5454a39512f4b76df153dca87e7127f4279` も[3 OS + MSRV 1.88のCI](https://github.com/n3dypop/Matea/actions/runs/38819360615)に成功しました。[GitHubから取得したjob/step証拠](evidence/coverage-ci-run.json)を保存しています。WindowsのPTYは対象外です。

## F5の前に残る作業

1. 全宣言から利用者機能への対応付け、Rustマクロ・再公開・platform/feature条件と基盤依存の範囲（128宣言辺）の解決、全機能の実装と正しさの証拠。
2. 全モジュールの条件を揃えた性能検証、課題分析、遅い経路・出力量の改善と再測定。現測定は一台のmacOS arm64、メモリsinkであり、実端末応答の検証ではありません。
3. Runtime拡張、style/layout、Bubbles/Huh/Glamour、Ratatui/Crossterm/Ultraviolet/xの残機能。依存の追加や再公開だけで対応完了としません。
4. 残機能を実装した際の3 OS CI再検証と、Windows native console、実端末のUnicode/IME/SSH/tmux、Kitty/Sixelの端末別検証。OS対応はプロトコル対応を意味しません。
5. 公開API・ライセンス・crate名の最終レビュー。全packageはpublish=falseで、Mateaの配布ライセンス、crates.io／商標の可用性は未確定です。
6. 協調しないユーザーworkerはRuntime終了後も残り得ます。キャンセル・復旧保証の範囲は[アーキテクチャ](architecture.md)に記録しています。

全公開機能と性能要件を満たした後にF5安定化結果と未解決事項を改めて報告し、次フェーズへ進む前に明示承認を求めます。現時点では承認を求めません。

GitルートはMatea/、remoteは[n3dypop/Matea](https://github.com/n3dypop/Matea)。Reference、AGENTS、CODEX_START、Skillsはリポジトリ外に保っています。

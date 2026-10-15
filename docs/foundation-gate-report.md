# Matea Foundation gate report

検証日時: **2026-10-16 06:41:09 JST**。状態: **F5未完了・ゲート未通過**。ローカル実装と検証は完了しましたが、3 OSのCI実行結果とオーナーの明示承認は未取得です。画像エンジン／CUDAの実装には進みません。

## 実装と設計

- F0: Bubble Tea v2、Lip Gloss、Bubbles、Huh、Glamour、Harmonica、Ultraviolet、x/ansi・term、Ratatui、Crosstermの固定ソースを調査。[revision一覧](upstream-lock.json)、[Bubble Tea監査](bubbletea-audit.md)、[Charm監査](charm-ecosystem-audit.md)。Goテストは未実行です。
- F1: 型付きModel/Effect、構造化View、描画方式、デザインシステム、フォーカス、crate責務、拡張境界、端末所有権を[ADR 0003〜0006](decisions/)に記録。公開APIは暫定で、固定・公開はしていません。
- F2–F4: リポジトリ直下に6 crateを実装。UIスレッド上の状態遷移、最大8 worker・32 completionのEffect実行、世代付きキャンセル、周期subscription、Unicode Surface、行単位diff、クリッピング、テーマ、レイアウト、input/list/table/viewport/spinner/formを実装しました。
- F5: カウンター、検索リスト、フォーム、ダッシュボード、型付き非同期Effect、別crateのBadgeを実行可能にしました。golden、mock backend、PTY、テスト、ドキュメント、CI設定と検証ログを追加しました。**CI設定の追加をCI成功とは扱っていません。**

[アーキテクチャ](architecture.md) / [デザインシステム](design-system.md) / [拡張契約](extension-contracts.md) / [サンプル実行ガイド](example-apps.md)

## 受け入れ条件ごとの証拠

| ロードマップの条件 | 結果 | 証拠・制約 |
| --- | --- | --- |
| エスケープ列をアプリに書かずcounterを作成 | ローカルPASS | `crates/matea/examples/counter.rs`、snapshot smoke。端末処理はrunが所有 |
| command/subscriptionのキャンセル・エラー・panic/exit復旧 | ローカルPASS・制約あり | core 7 tests、terminal 3 tests、PTY 8ケース。協調キャンセルであり、停止しない任意のユーザー処理を強制終了する保証はない |
| layout/style/theme/componentのcrate間再利用 | ローカルPASS | style → render → core、widgets、terminal、facade。Style/Themeは値、Componentは公開trait |
| 第三者widgetを内部変更なしで統合 | ローカルPASS | 独立package `examples/external-widget` のテスト・snapshot |
| input/focus/Unicode幅/resize/clipping/diffのテスト | ローカルPASS | core/render/style/widgetsテスト、幅広文字の置換・重なり、制御文字、0寸法、過大寸法、PTY resize |
| counter/dashboard/list/formの実行 | ローカルPASS | 全6サンプルのsnapshot、dashboard golden。PTYで実端末I/O経路とEffect完了も検証 |
| Linux/Windows/macOSのCI build/test | **未検証・ブロッカー** | `.github/workflows/foundation.yml` に3 OSとMSRV job。隔離した `Matea/` 内でGitを初期化し、`n3dypop/Matea` をoriginに設定済み。remote CIは初回push後に確認。手元macOSの成功は他OSの証明ではない |
| graphics拡張境界を文書化、画像実装は存在しない | PASS | extension-contracts.md。実装範囲のリポジトリ に画像crate、codec、Kitty/Sixel encoder、CUDA、画像benchmarkなし。提供済みreferencesは実装成果に含めない |
| Foundationの明示的なオーナー承認 | **未取得** | 3 OSの証拠を添付し、残課題をレビューした後に承認を依頼する。現在は次フェーズへの移行を求めない |

## 実行結果

環境: `macOS-27.0.1-arm64-arm-64bit-Mach-O`、`rustc 1.99.0 (b940084d7 2026-09-28)`、`cargo 1.99.0 (5f94df478 2026-08-27)`。MSRV 1.88は設定のみで、手元では未実行です。

再現コマンド（プロジェクトの作業ディレクトリから）:

```sh
python3 scripts/verify_foundation.py --offline
```

依存取得前の環境では `--offline` を外してください。ローカル絶対パスはplaceholderに置換しています。実際の引数・exit code・所要時間は[local-validation.json](evidence/local-validation.json)に保存しています。下表は**このホスト**の実行結果です。

| 検証 | 結果 | ログ |
| --- | --- | --- |
| `fmt` | PASS (exit 0) | [log](evidence/fmt.log) |
| `clippy` | PASS (exit 0) | [log](evidence/clippy.log) |
| `test` | PASS (exit 0) | [log](evidence/test.log) |
| `all-targets` | PASS (exit 0) | [log](evidence/all-targets.log) |
| `docs` | PASS (exit 0) | [log](evidence/docs.log) |
| `build-examples` | PASS (exit 0) | [log](evidence/build-examples.log) |
| `example-smoke` | PASS (exit 0) | [log](evidence/example-smoke.log) |
| `pty-smoke` | PASS (exit 0) | [log](evidence/pty-smoke.log) |
| `render-probe` | PASS (exit 0) | [log](evidence/render-probe.log) |

`cargo test --workspace`: **22 tests + 1 compile-only doc test成功**。all-targetsでも失敗なし。snapshotは6サンプルで成功、dashboardはgolden一致、escape文字なし。

PTYでは通常終了、resize、raw Ctrl-C、SIGTERM、Update panic、最初のView panic、型付きEffect完了、run終了後のSIGTERMを検証しました。termiosの元値と、alternate screen・cursor・paste・focusの解除コマンドを確認しています。PTYは実エミュレータの見た目やWindows consoleを検証するものではありません。

描画probe: 80×24セル、1,000回のrender合計 **118,381 µs**、初回出力 **4,037 bytes**、同一frameの出力 **0 bytes**。単一ホストの1回の測定です。TTY I/O、入力応答、p95、他frameworkとの比較や性能優位性は測定していません。画像benchmarkではありません。

アクセシビリティ確認: 色なしでも選択marker・reverse・Errorラベルを保持、reduced motion、default RGB tokenの計算上のcontrast >= 4.5:1。対話型screen-reader promptや利用者評価は未実施です。

## 検証で見つけて解消した問題

1. 初回描画と最初のpollの間のSIGWINCHが取りこぼされるため、描画前にCrosstermの監視を初期化しました。PTY resizeが成功しています。
2. signal-hookの解除だけでは既存handlerが復元されないため、Unixの小さなsigaction guardへ変更しました。安全性の説明を付けたFFI部分のみunsafeを許可し、他のcrateでは禁止しています。元handlerの保存・部分初期化失敗・終了時復元を扱い、run終了後のSIGTERMも確認しました。[ADR 0006](decisions/0006-terminal-portability.md)
3. Unicodeの幅広glyphを部分的に上書きしたときの残骸を防ぐため、glyph単位の消去と、変更行をclearしてからの描画を採用しました。Ratatuiの再利用は今後の機能拡張前に再評価します。

## 未解決の課題と承認前の作業

1. **CI実行結果の取得。** オーナー指定の `n3dypop/Matea` を使用します。`Matea/` 内にソース・テスト・CI・技術資料を隔離してGitを初期化しました。参照ソース、agent指示書、skillsはリポジトリに含めません。初回push後にLinux/Windows/macOSとMSRVの結果を添付します。
2. オーナーによる公開API・ADR・サンプルのレビュー。各crate名、crates.io／商標の可用性は未確認です。全packageは `publish=false`。Matea本体の配布ライセンスは未選択で、公開前の判断事項です。
3. 実端末のUnicode幅、IME、SSH/tmux、Windows console、カスタムpaletteは未検証。真のinline対話描画、mouse、function key、enhanced keyboard、clipboard、suspend/resume、screen-reader用の線形promptは未実装です。
4. 協調しないユーザーworkerはRuntime終了後も残り得ます。I/O timeoutとtoken確認が必要です。SIGKILL、abort、電源断、出力失敗による復旧失敗は保証できません。セッション中にアプリ側でSIGINT/SIGTERM handlerを同時に置き換えることも非対応です。
5. 現rendererは正しさを優先した行単位描画です。高速・省帯域・全端末互換とは主張しません。詳細は[リスク一覧](risks.md)。

上記の検証範囲・制約をオーナーが確認し、必須CIが揃った後にFoundation F5の明示承認を依頼します。**それまでは画像／CUDAフェーズへ進みません。**

## Repository isolation

The Git root is `Matea/` and its origin is [n3dypop/Matea](https://github.com/n3dypop/Matea). External source checkouts, `.agents/`, `AGENTS.md`, `CODEX_START.md` and skills remain outside the repository. Only framework source, examples, tests, CI, validation scripts and engineering documents are included.

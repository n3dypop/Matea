# Independent terminal oracle performance and failure review

2026-10-22。対象は `check_terminal_trace.py`、`check_color_trace.py`、
`check_ratatui_bridge.py` の3維持プログラム。これはフレームワークの描画速度や
物理端末の適合性の比較ではない。未達全件の分析は
[48軸の原因表](unmet-axes-analysis.md)、判断は [ADR 0077](decisions/0077-bounded-independent-terminal-oracles.md)。

旧測定は小さい5 fixtureの関数実行だけで、大画面、不正入力、CLI全体、
bridge検証、native RSSを欠いていた。実コードでは行消去`EL`が全画面を走査し、
不完全/範囲外RGBを受理する経路があった。絶対カーソル位置は画面外になり、
遅延wrap位置で`ECH`が何も消去しなかった。

[既知の失敗](evidence/oracle-scale/bounded-before-tests.log)は追加4試験のsubcaseで
58失敗/7例外。最初の全否定試験はrunnerがtimeoutを付け忘れ、過大な`REP`で
滞留したため自分で起動した子だけをTERMした（exit -15）。
[その記録](evidence/oracle-scale/before-context.json)も残し、完走した試験とは扱わない。
以後の否定/CLI/RSS子にはtimeoutを付けた。

行/画面/文字消去を対象範囲のslice操作に変更。色blankは不変tupleを共有し、
行listは共有しない。座標を画面へclampし、erasureは現在の色/属性を保持する。
geometry/input/CSI/countを明示的に制限し、palette/RGBの欠落・範囲外を拒否する。
これらは監査用モデルの対応範囲で、Matea公開APIの機能範囲は変更しない。
独立flat-cellモデルは3 gridで全位置・3消去命令・全対応mode、遅延wrapを含む
670構成を両replayerへ適用する。既存margin/fallback試験と合わせ10試験。

最初の候補はCSI正規表現の後にPythonで数字を再走査したため、短い制御列が
遅くなった。第2候補は同じ検証条件を正規表現の文法へ統合し、semanticな
arity/色検査を保持する。旧版・第1候補を保存し、失敗した候補のデータを消さない。

二候補それぞれ、18同出力ケース×旧新版×3cohortと9 current-onlyケース×3で
135 records /4,185正の標本/405 fresh whole-fixture RSS子を記録した。
合計270 records /8,370標本/810 RSS子。
[全cohortとadverse](evidence/oracle-scale/verified-comparison.json)、
[再検証producer](evidence/oracle-scale/verify.py.txt)。
順序は旧→新、新→旧、旧→新。全出力のbyte/hash/長さを比較し、メモリは別pass。
同出力ケースにはASCII/cursor、4色profile/margins/fallback、大画面J/K/X、
12組のbridge projection検証、全3CLI、probe付きcolor CLIとpublicationを含む。
新規rejection/clamp/wrap/大projectionは旧版との比率を作らない。

| 第2候補・3cohortの中央値 | 旧版 | 修正版 |
| --- | ---: | ---: |
| ASCII 256×128、32回EL、再生と全出力 | 184.6–189.7 ms | 12.7–12.8 ms |
| 色付き256×128、32回EL、再生と全出力 | 199.4–201.4 ms | 25.4–25.7 ms |
| retained ASCII fixture全再生 | 547.5–567.5 µs | 92.6–97.0 µs |
| 全4色profile検証 | 18.54–18.62 ms | 12.41–12.42 ms |
| color入力ファイルCLI全体 | 74.5–80.0 ms | 62.7–63.2 ms |
| probe/color/fallback/出力保存CLI全体 | 84.4–85.7 ms | 72.7–73.5 ms |

色付き大画面のPython traced peakは5,220,517→4,965,805 bytes、
4色検証は171,434→120,834 bytes。native RSSは別のfresh process全体で、
interpreter/import/inputを含む。親と子のpeakは同時RSSとして加算しない。
未変更のRatatui checkerはcontrolで、約241–244msのCLI中央値や悪化したtailも
保持する。変更のないbodyへ速度向上を帰属しない。

単発の短制御列には第2候補でも大きな外れたcohortがあるため、元の全分布を残し、
[20ms以上・3確認の校正付き再測定](evidence/oracle-scale/calibrated-controls/context.json)
を追加した。18 records /558 batch標本で、各標本はbatch平均。
個別操作のp95 latencyとは呼ばない。
ASCIIは旧14.02–14.10、第1候補17.44–17.63、第2候補15.35–15.51µs。
色制御列は旧24.69–25.00、第1候補29.21–29.91、第2候補26.62–27.53µs。
重複走査は改善したが、旧版に対し正しい入力検証の約1.3–2.5µsのコストが残る。
検証を省いて旧版の速度へ戻す判断はしない。この明示的な処理追加と、説明不能な
ノイズ/退行を分ける。全ケース高速化、最大速度、ゼロコスト検証とは主張しない。

[完全な失敗journal経路](evidence/oracle-scale/failure-performance/context.json)も
プローブ欠落/exit9/不正出力の3条件×3cohort、9 records /279標本/27 RSS子で測定。
一意のtemp prefixだけを正規化し、例外の種類と失敗journal全体を保持する。
外部サービス障害を再現したとは扱わない。この3プログラムにnetwork APIはなく、
旧remainingの一般的な「network未検証」は別のfetcher単位へ帰属する。

[継続実行のメモリ観測](evidence/oracle-scale/retention/context.json)は別の3 fresh子で、
入力と期待出力を保持し、8回実行→出力破棄→GCを4回行う。初期値を含む15観測、
96完全操作。RSSは子自身のPIDだけを照会する。tracemallocには観測metadataも含む。
有限の観測であり、全入力でのleak不存在を証明しない。

性能75の判断範囲は、ここに列挙した維持CLI/関数と有限の小/大/失敗/反復条件。
正しい検証の固定コストを明示したうえで、残っていたscale/CLI/失敗/native memoryの
測定とsource一致のnative/3 OS qualificationが必要。完了前に昇格しない。
基盤公開機能の全件実装、全export閉包、Stage B、F5は独立した未達のまま。

qualification完了。ce3d203の[3 OS＋MSRV 4ジョブ](https://github.com/n3dypop/Matea/actions/runs/39689754567)を
[200入力・生ログへ照合](evidence/oracle-scale/ci-bindings.json)した。各OSの47受入、新規10試験、Python151試験が成功。
参照／計測ドライバー再収集はskip。評価表はscore1件と古い理由のみを変更し、プログラムのCIソースは同一。
評価更新後のローカルは47受入、Rust285＋14 doctest、Python151（skip 2）、LLVM97/104が成功。
CI証拠ファイル未取得による最初のinventory失敗を保存し、同一入力の成功46項目を保持して、その1項目だけを再実行した。
[両journalと最終source binding](evidence/oracle-scale/final-native/archive.json)。
依存hashbrownの3 profiling警告を診断して保持。Rust/manifestは0c95c37と同一で、Miriの追加実行はない。
性能75はこの3プログラムの実装済み・有限scopeの評価で、全ケース旧版以下の時間や普遍的最大速度を意味しない。
未達は48→47（参照21・性能26）。全閾値・44単位・158プログラムの範囲とF5/Stage Bの順序を維持した。

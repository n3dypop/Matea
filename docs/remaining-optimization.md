# 未測定12単位の測定と改善

個別評価Stage Aを継続中。全項目高評価・Stage B・F5の条件は未達です。
現在は100プログラム、40評価単位、51軸が75未満です。
完全未測定の性能単位は12から0になりました。11単位は部分証拠の50、
facadeの性能は現在の再公開APIに限定して75へ更新しました。
List/Table/Formの実装済み動作も追加の独立境界試験で75へ更新しました。
性能27、参照元一致18、動作3、外部ツールの移植性3が引き続き未達です。
compiler収集の動作・移植性、台帳／課題処理の移植性を、後述の実行証拠に基づいて75へ更新しました。

## 測定対象

先行分76件にcompiler抽出2件を加え、固有ケースは78件です。すべて単一macOS arm64上で31標本を保持しました。
比較にはコンパイル・テスト・Miri・LLVM計測から分離した実行を使っています。
分離を確認できなかった実行も、生データと除外理由を保存してから再測定しています。

| 対象 | 件数 | 検査範囲・限界 |
| --- | ---: | --- |
| ANSI・モード・Unix signal | 33 | private production sourceを直接使用。色とcommandは既存byte sink。モードはliteral全出力とlogical write/flush数。signalは隔離プロセス内の実sigaction。実端末の遅延・物理write数は未測定 |
| 端末イベントループ・facade | 15 | mock backendで開始、終了、idle、resize、キー、5種のI/O失敗。復旧回数と状態を検査。facade/module/preludeの型と関数別名を検査し、同じ文字列構築を3経路で測定 |
| Goソース監査CLI | 4 | Bubble Tea 405、Bubbles 873、Ultraviolet 968、独立生成10,000宣言。起動からpipe出力・終了まで、改善前後の全byteを照合 |
| examples・収集・検証・報告 | 24 | release snapshot 12、独立oracle 5、全20compiler profileの取込計画、全841保存issueのmerge、台帳CLI、全84ケースの報告生成、品質対象列挙、46ケースの厳密比較、Go84ケースの読込 |
| compiler抽出 | 2 | 保存したnative Crossterm/allとRatatui Core/allのJSON読込・解析・抽出・canonical出力。既存graphの全fieldと照合。compiler実行・通信は対象外 |

コンパイラ取込だけを改善後に再測定しました。他の23経路は繰り返していません。
[全出力・入力・Gitソースhashの照合結果](evidence/optimization/remaining-report.json)、
[端末33ケース](evidence/optimization/terminal-internals/report.json)、
[イベントループ等の生データ](evidence/optimization/foundation-paths/current.jsonl)、
[例とツールの生データ](evidence/optimization/remaining-paths/current.jsonl)。

## 改善結果

| 経路 | p50 改善前 → 改善後 | メモリ等 |
| --- | ---: | --- |
| モード最小設定の取得・復旧 | 90.94 → 22.69 ns | 2割り当て・10要求byte → 0、flush 8 → 2、44出力byte一致 |
| モード既定設定の取得・復旧 | 101.00 → 34.62 ns | 2割り当て → 0、flush 12 → 2、76出力byte一致 |
| モード全設定の取得・復旧 | 132.45 → 75.49 ns | 2割り当て → 0、flush 16 → 2、166出力byte一致 |
| Unix signalの取得・復旧 | 682.07 → 649.06 ns | 1割り当て・96要求byte → 0。1ホストの成功syscall経路 |
| Go監査CLI・10,000宣言 | 19.104 → 14.626 ms | 全出力一致。割り当て数・RSSは未測定 |
| 全20profileの取込計画 | 1,643.60 → 1,407.95 ms | Python traced peak 102,175,175 → 79,686,071 byte、全出力hash一致 |

flushだけを減らした中間版はメモリsinkで遅くなりました。その全標本を保存し、
固定属性リセットが作る文字列割り当てを除去して再測定しています。
[中間版](evidence/optimization/terminal-internals/phase-flushing-report.json)も削除していません。
GoのBubble Tea経路のp95は13.31→13.87msへ悪化しており、速度の一律改善とは扱いません。
割り当て要求byte、Python traced peak、OSの常駐メモリは異なる指標です。RSSは未測定です。
全ての分布・不利な結果は生データに残しています。

## 実装修正と試験

- 端末の排他権をRAII leaseで保持し、backendのDrop本体と内部signal guardの破棄まで
  終わってから解放します。従来の順序では復旧失敗後のsignal再試行が次セッションと
  重なる可能性がありました。これはソース順序の欠陥で、実kernel raceの再現とは扱いません。
- signalの2項目を固定長journalへ変更し、成功した復旧は除去、失敗だけ逆順再試行します。
  全失敗maskと部分取得、重複取得、constructor error/panicを試験しました。
- rustdoc取込は全入力の検証後に書き込みます。pin、profile identity、成功exit code、
  重複itemと同名衝突を検証し、解析済みJSONを再利用します。
- issue再取得が既存の判定・証拠を消さないようにしました。open一覧から消えた項目も
  保持し、再openで回復します。一覧からの消失だけでclosedとは断定しません。
- 性能報告は重複case/JSON field、NaN/Inf、不正counterを拒否します。過去のGo平均値と
  古い記録の欠落項目は捏造せずに保持します。全5報告CLIの`--check`は証拠を書きません。
  Windowsの`.exe`と、削除済みのMac一時binaryを必須としていた問題も修正しました。
- custom filterのpanic後にListの選択位置が無効になる失敗を実際に再現しました。
  callback前に選択を初期化し、受理済みprefixの先頭または空選択という状態を保証します。
  [失敗ログ](evidence/optimization/collections/filter-before.log)を保持しています。
- LLVMの同一sourceに対する複数compiler instanceを、別ファイルとして数えたり、
  都合のよい割合で上書きしたりしません。canonical pathごとに全instanceを別々に保持します。

判断は[ADR 0028](decisions/0028-terminal-ownership-and-phase-flushing.md)、
[0029](decisions/0029-research-evidence-refresh-integrity.md)、
[0030](decisions/0030-filter-panic-state-guarantee.md)に記録しました。
新規の7件のtargeted Miriは端末所有権・journal・filter panic・Formを対象とします。
FFI syscall失敗や全実行順序をMiriで証明したという意味ではありません。

## 継続する評価

[明示台帳](quality-plan.json)の未達軸を残しています。全94参照module groupのうち
未対応・未連結の公開機能、platform/feature closure、実際の入力・timer・effect、
常駐メモリ、故障経路、同じ意味と入出力での参照元比較が必要です。
今回の78件は部分経路の測定であり、この範囲を縮めたり閾値を変えたりしていません。
全体最適化Stage Bと、画像エンジン／CUDAへの実装着手は行っていません。


## 時計整形の追加検証

既存16ケースを200000反復・31標本で再実行しました。整数整形crate利用、2桁table、
親関数だけのinline化は十分な改善がなく、全データを残して不採用としました。
最終案は元の算法・UTF-8検証・所有文字列の型を保ち、private helper全体のinline可能性を
渡します。同じ場所で再構築したbaseline binaryは最初のSHA256と一致し、
old/new/new/oldの順でも再測定しました。再利用bufferのns表示は13.89→12.89nsと
13.66→12.99nsで改善しましたが、所有文字列や最大値は混在しています。
text sectionは1600byte増えました。速度の一律改善・最大化・clock全体の高評価は主張しません。
[全分布と不採用・除外理由](evidence/optimization/clock-integers/report.json)、
[ADR 0031](decisions/0031-reviewed-integer-formatting-reuse.md)。これは16/84既存ケースへの追補で、
新しい参照機能やStage Bの試験数には加えていません。

## 証拠の追加検証

compiler snapshotはRust以外も含む全追跡入力とmodeを照合し、コピー完了後に公開します。
出力欠落・破損、未対応形式、再帰エラー、起動失敗・timeout、依存tree失敗も非成功として
記録します。16件の故障・境界試験は実compiler／通信をmockした試験で、実profile収集とは区別します。
判断は[ADR 0032](decisions/0032-complete-atomic-reference-snapshots.md)に記録しました。

台帳の重複caseによる失敗上書きと、文字列`"false"`の一致扱いを再現して修正しました。
古いSHA、証拠の欠落・外部path、重複JSON field、compiler identity/key/exitの不整合を拒否し、
正常な部分台帳でもgateを閉じる10件の独立試験を追加しています。
[ADR 0033](decisions/0033-strict-capability-evidence-validation.md)、
[改善前の失敗ログ](evidence/optimization/coverage-guard/before.log)。

既存の17件の課題評価には、参照SHA・upstream更新時刻・証拠hashを追加しました。
証拠の変更で評価をpendingへ戻し、4件の部分評価も閉鎖条件へ含めます。
総数841件のうち824未調査と4部分評価の計828件に作業が残ります。
これは新しい17件の再現を行ったという意味ではありません。

厳密な台帳検証のJSON処理は、同じ検査・入力・出力でp50 634.10→532.12ms、
p95 680.29→539.67msでした。[全標本と除外した重複実行](evidence/optimization/coverage-guard/report.json)。
検査の弱い旧版との速度比較には使用しません。

compiler抽出は、同じfilenameのpath解決を抽出内で再利用します。
Crosstermはp50 103.73→95.68ms、Ratatui Coreは233.30→192.73msとなり、
それぞれgraph全体が一致しました。Python traced peakは変わらず、RSSは未測定です。
[ADR 0034](decisions/0034-local-source-path-reuse-in-api-extraction.md)、
[全分布・compiler入力hash・別実行profile](evidence/optimization/compiler-extraction/report.json)。

## 最終検証と残件

ローカルの30受入検証、Rust 137テスト＋2 doctest、Python 72テストの通常／最適化実行、
全RGB検証、12 snapshot、17 Unix PTYが成功しました。LLVMは56 source／61 compiler instanceを
区別して保持し、出力専用84 recordも検査しました。追加Miriの7実行は
[対象・source hash・制約](evidence/optimization/remaining-miri-context.json)へ結び付けています。

[最新の3 OS＋MSRV CI](https://github.com/n3dypop/Matea/actions/runs/39069122055)は4ジョブ成功です。
[参照元の専用run](https://github.com/n3dypop/Matea/actions/runs/39068619681)では参照9ジョブが成功し、
本体3 OSは未同梱の測定reportによる証拠欠落で失敗しました。そのreportは同梱済みで、
前者が修正後の結果です。単一runの13ジョブ成功とは表現しません。
[全programのsource hashによる対応付け](evidence/optimization/ci-source-bindings.json)で、
現在の本体と参照実行のprogramを確認できます。参照6ジョブの全20 graphは
[以前のcohortと全fieldが一致](evidence/optimization/compiler-extraction/ci-cohort-comparison.json)しました。
Windowsの改行設定による履歴patchの復元失敗も、Git設定を局所指定して修正しています。
途中の失敗は[CI履歴](evidence/optimization/ci-attempts/)へ保持しています。

この追加証拠でcompiler収集の動作・移植性と、台帳／課題処理の移植性を75へ更新しました。
100プログラム・40単位で未達は51軸です。内訳は性能27、参照元一致18、動作3、移植性3で、
平均や評価範囲の縮小は用いていません。GoのWindows計時、測定driverやLLVMのOS別実行、
残る故障経路・性能分布・RSSは未確認分を維持します。参照宣言6,955件とcompiler keyの
6,902件（重複する集合）は未対応付けで、91 module groupもpendingです。
Stage A、Stage B移行条件、Foundation F5の条件は引き続き未達です。

# 未測定12単位の測定と改善

個別評価Stage Aを継続中。全項目高評価・Stage B・F5の条件は未達です。
現在は103プログラム・41評価単位で53軸が75未満です。性能28、参照元一致19、動作3、移植性3です。
新規カーソルを独立単位へ加えたため、従来の51軸に部分評価2軸が加わりました。
カーソル追加前は100プログラム、40評価単位、51軸が75未満でした。
完全未測定の性能単位は12から0になりました。11単位は部分証拠の50、
facadeの性能は現在の再公開APIに限定して75へ更新しました。
List/Table/Formの実装済み動作も追加の独立境界試験で75へ更新しました。
その時点の内訳は性能27、参照元一致18、動作3、外部ツールの移植性3です。
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

## カーソル追加前の検証と残件

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

## 仮想カーソルの実装・測定

未対応のBubbles cursor groupを調査し、21公開宣言を新しいVirtualCursorと文字入力への
合成APIへ対応付けました。点滅・固定・非表示、focus/blur、正の周期、style、表示文字を
実装しています。任意のLip Gloss decoratorや実際のtimer配送比較は部分対応のままです。
[監査と明示した差分](cursor-audit.md)、[ADR 0035](decisions/0035-owned-virtual-cursor-and-expiring-notifications.md)。

参照元では、期限後の通知を保持したまま固定→点滅へ戻すと、初期化broadcastを処理する
前に古い通知を受理する場合がありました。公開APIと実timer callbackで再現しています。
Mateaは制御時に世代を更新し、古い通知を拒否します。参照元の過去のclosure raceは
固定リビジョンでは修正済みで、未修正の問題とは扱いません。

7つの追加workloadについて意味・全出力・戻り値を照合し、各31標本を採取しました。
この追補の固有測定は78→85件、参照元比較の集合は既存84＋新規7ケースです。
制御の初版はGoより遅かったため、resetとstart/stopを1回の時刻・世代検査を伴う遷移へ
まとめました。旧binaryを同じsource pathで再構築し、元のSHA256一致も確認しました。

| 経路 | 旧→新の中央値（ns） | 注意 |
| --- | --- | --- |
| 固定focus/blur・初回比較 | 5.83→3.64 | Goは2.43→2.37。Mateaは追加の時刻・世代保証を持つ |
| 非表示focus/blur・初回比較 | 5.95→3.86 | Goは2.43→2.41。双方0割り当て |
| 固定・旧/新/新/旧の順序反転 | 5.94 / 3.64 / 3.55 / 5.95 | 同一hashのbinary、同じ全出力 |
| 非表示・同じ順序反転 | 5.55 / 3.64 / 3.64 / 5.56 | 全標本を保持 |

4描画ケースの初回中央値はMatea 298–544ns、Go 1,879–2,199nsでした。GoのANSIから
セルへの処理とMateaの構造化Viewを含めた限定比較で、guard単体の倍率ではありません。
改善後のASCII描画p95は一組で311→482nsへ悪化し、順序反転でも描画の結果は混在しました。
制御の改善を全描画の高速化や性能最大化とは主張しません。
[最終版を含む10実行分の全分布・hash・制約](evidence/cursor/performance/report.json)。

一行表示の追加レビューでは改行・CRLF・タブの差分も修正し、修正前の実装に新試験を
適用して失敗を確認しました。最終6試験を通常・release・Miriで検証しています。
Miriは初版5・遷移改善5・最終6実行を保存し、ソースとの対応とseedを記録しました。
ローカル30受入チェック、Rust 143＋2 doctest、
Python 72、全RGB、13 snapshot、18 Unix PTYが成功。LLVMは59 source／64 compiler instanceを
別々に保持します。既存の文字入力14試験・時計8試験と実参照元の境界ケースも再確認し、
変更でstaleとなった既存10 issueレビューの証拠を更新しました。新たな10件の問題発見や
実端末の再現を行ったという意味ではありません。

一行正規化を含む最終版でも7ケースを再測定し、全出力の一致を確認しました。
固定／非表示制御の中央値は3.43／3.43ns、Goは2.42／2.42nsでした。
描画4件は286–535nsでした。文字設定そのものやconstructorの性能は、この7件の対象外です。

[最終CI](https://github.com/n3dypop/Matea/actions/runs/39075023243)は本体3 OS・MSRV・参照照合3 OSの
7ジョブ成功です。[全103 programのsource対応](evidence/cursor/ci-source-bindings.json)、
[3 OS artifactの独立照合](evidence/cursor/ci-reference-comparison.json)を保存しました。
参照compiler profileは再収集せず、同一の5 programで収集済みの20 profileを継続使用しています。
現在のscore更新後も[品質・公開機能gateは実際にexit 1](evidence/quality/final-gate-status.json)です。
ローカル検証時のquality logは更新前の54軸を記録し、CI後の移植性更新で53軸となりました。
参照元宣言は521件が連結、6,934件は未対応付けで、compiler key 6,902件とは重複する集合です。
12部分capability、91 pending module group、828 issueと115 marker、feature/signature閉包が残ります。
Stage A完了・Stage B移行・Foundation F5承認の条件は満たしていません。

## 棚卸し・測定・受入ドライバの再開検証

現在は107プログラム・41単位、未達49軸です。性能28、参照一致19、動作1、移植性1で、
全高評価には未到達です。今回の4軸の更新はツールの動作・移植性の実行証拠によるものです。
参照機能や性能の範囲・閾値を変更していません。

- 棚卸しを計画と公開に分離し、全11 inventoryのハッシュを含むsummaryを最後に公開します。
  途中の失敗、古いxモジュールの残骸、重複宣言や不正なJSONを拒否し、既存のレビューを保持します。
  全inventory本文は前の正しいcohortとbyte一致です。[ADR 0036](decisions/0036-hash-bound-source-inventory-publication.md)。
- Go計測器は独立した校正で反復数を決定します。Windowsの短バッチ失敗を2回保存し、
  同じ計測ループ・3回確認・20ms目標へ改修しました。標本の除外・補正・ゼロの置換は行いません。
  [ADR 0037](decisions/0037-calibrated-reference-probe-batches.md)。
- 受入／LLVM実行器は開始時に旧成功を無効化し、起動失敗・timeout・部分出力を記録します。
  ソース変化、ログ不一致、未完了の結果をquality reportへ採用しません。40単位の固定条件も
  除去し、41単位や将来の全高評価で測定が止まらないようにしました。
  [ADR 0038](decisions/0038-source-bound-acceptance-journals.md)。

[実装66e058cのCI](https://github.com/n3dypop/Matea/actions/runs/39079660329)は7必須ジョブ成功です。
3 OSごとに全program/build/check入力のハッシュを照合し、Goの91ケースがそれぞれ31の正の
標本を返し、出力・校正・割り当てschemaを検査したことをartifactで確認しました。
共有runnerの時間分布を性能比較へ使用していません。
受入チェックはLinux/macOSで33、Windowsで32、LLVMはUnix 59 source／64 instance、
Windows 58／62です。Windows PTYは対象外です。
[ソースとの対応・各OSの成果物・制約](evidence/measurement-drivers/ci-source-bindings.json)。
新しいPython回帰試験は合計103件、Go単体試験は5件が通過しました。

追加の各31標本では、評価台帳全体がp50 13.145ms／p95 13.553ms、
カーソル例の起動・snapshot・終了が3.921ms／7.878msでした。
[台帳処理](evidence/measurement-drivers/acceptance/current.jsonl)、
[カーソル例](evidence/measurement-drivers/cursor-example/current.jsonl)と各contextに全分布・入力hashを保存しています。
前者はPython heapを別実行で記録しています。native RSS・全受入処理の時間・対参照元比較ではなく、
いずれも対応する性能軸は部分評価50を維持します。

未対応候補6,934件、部分capability 12、pending module group 91などは残っています。
実装済みRuntimeの固定16ms入力待ちについては、次節の再現・修正・比較を追加しました。
横断最適化Stage BとF5へは移行していません。

## 購読期限を考慮する入力待機

独立したbackend試験で、1ms購読があっても16msのinput waitを要求する不具合を再現しました。
[修正前の失敗](evidence/deadlines/before-failure.log)を保存し、
`Runtime::next_subscription_delay()`と、描画からの経過時間を差し引く待機選択を追加しました。
購読数は64以下で、期限の検査は借用のみ、追加の割り当てやplatform clockの読み取りを
coreへ持ち込みません。購読なしの経路では時計を読みません。
[ADR 0039](decisions/0039-deadline-aware-terminal-polling.md)。

実際のsleepを行うheadless backendで、開始・初期frame・通知・終了・復旧を含む4条件を
新しく測定し、既存の入力・fault・facade 15条件を対照に再実行しました。
元binaryのhashを保持したまま、旧／新／新／旧の順序で各31標本を収集しています。
[全raw・3実装段階・source/binary hash・全対照結果](evidence/deadlines/report.json)。

| 要求周期 | 旧の中央値（2実行、ms） | 新の中央値（2実行、ms） |
| --- | ---: | ---: |
| 1ms | 19.06 / 24.02 | 1.516 / 1.516 |
| 5ms | 21.76 / 21.84 | 7.537 / 7.113 |
| 16ms | 22.37 / 22.56 | 21.32 / 22.85 |
| 50ms | 63.50 / 64.80 | 52.70 / 51.89 |

16ms条件は改善を断定できません。初版では64キー処理とidleの中央値が悪化し、購読なしの
不要な時計読み取りを除去しました。それでも最終の64キー処理は旧13.31–13.43µs対
新13.60–13.65µs、idleは旧579–589ns対新593–614nsでした。fault経路にも悪化があり、
全分布を保持しています。通知条件の割り当ては双方11回・要求6,108bytesのままです。
OSのsleep超過を含む一台の結果で、RSS、実端末の応答、Goのtimerとの意味を揃えた比較、
64購読時などの負荷条件は未測定です。性能軸を高評価へ変更しません。

通常／Miriでは5つの独立試験が成功しました。Miriは初版4実行＋idle改善後2再実行＋
入力時刻修正後3実行で、5固有テスト・9実行です。
[対象・seed・sourceとlogのhash](evidence/deadlines/miri-context.json)。
LLVMとCIには4通知条件の出力専用実行を追加しています。計測値を作らず、全条件と復旧の
検査を通したことだけを確認します。worker完了通知の統合wake sourceは別の残課題です。

追試では、入力で新しく開始した2ms購読に、その入力を待った10msが加算され、
即座に発火する不具合も再現しました。
[失敗](evidence/deadlines/input-start-before-failure.log)、
[通知順序の決定](decisions/0040-align-logical-time-before-input-dispatch.md)。
割り込みを先に扱い、期限済みtimerを進めてから通常入力をdispatchすることで、
新規購読を同期済みの時刻から開始します。Goのあらゆるchannel選択順序の再現ではありません。

入力時刻修正後も、新／旧／旧／新で再測定しました。1ms条件の中央値は旧21.58–24.02ms対
新1.512–1.525ms、50msは旧63.64–65.10ms対新52.60–52.85msでした。16ms条件は引き続き混在します。
旧・中間2段階・最終のsource/binary hashと全対照結果を同じreportに保持しています。
[最終141c354のCI](https://github.com/n3dypop/Matea/actions/runs/39088967599)は7必須ジョブ成功です。
ローカル33検証、Rust 148＋2 doctest、Python 103、13 snapshot、18 Unix PTY、
LLVM 59 source／64 instanceが成功しました。未達49軸と両gateの閉鎖は維持します。

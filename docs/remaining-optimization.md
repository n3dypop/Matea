# 未測定12単位の測定と改善

個別評価Stage Aを継続中。全項目高評価・Stage B・F5の条件は未達です。
現在は97プログラム、40評価単位、54軸が75未満です。
完全未測定の性能単位は12から0になりました。11単位は部分証拠の50、
facadeの性能は現在の再公開APIに限定して75へ更新しました。
List/Table/Formの実装済み動作も追加の独立境界試験で75へ更新しました。
性能27、参照元一致18、動作4、外部ツールの移植性5が引き続き未達です。
移植性の更新は新しいCIの実行結果を確認してから行います。

## 測定対象

追加した固有ケースは76件です。すべて単一macOS arm64上で31標本を保持しました。
コンパイル、テスト、Miri、LLVM計測と性能測定を重ねていません。

| 対象 | 件数 | 検査範囲・限界 |
| --- | ---: | --- |
| ANSI・モード・Unix signal | 33 | private production sourceを直接使用。色とcommandは既存byte sink。モードはliteral全出力とlogical write/flush数。signalは隔離プロセス内の実sigaction。実端末の遅延・物理write数は未測定 |
| 端末イベントループ・facade | 15 | mock backendで開始、終了、idle、resize、キー、5種のI/O失敗。復旧回数と状態を検査。facade/module/preludeの型と関数別名を検査し、同じ文字列構築を3経路で測定 |
| Goソース監査CLI | 4 | Bubble Tea 405、Bubbles 873、Ultraviolet 968、独立生成10,000宣言。起動からpipe出力・終了まで、改善前後の全byteを照合 |
| examples・収集・検証・報告 | 24 | release snapshot 12、独立oracle 5、全20compiler profileの取込計画、全824保存issueのmerge、台帳CLI、全84ケースの報告生成、品質対象列挙、46ケースの厳密比較、Go84ケースの読込 |

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
新規の6件のtargeted Miriは端末所有権・journal・filter panic・Formを対象とします。
FFI syscall失敗や全実行順序をMiriで証明したという意味ではありません。

## 継続する評価

[明示台帳](quality-plan.json)の未達軸を残しています。全94参照module groupのうち
未対応・未連結の公開機能、platform/feature closure、実際の入力・timer・effect、
常駐メモリ、故障経路、同じ意味と入出力での参照元比較が必要です。
今回の76件は部分経路の測定であり、この範囲を縮めたり閾値を変えたりしていません。
全体最適化Stage Bと、画像エンジン／CUDAへの実装着手は行っていません。

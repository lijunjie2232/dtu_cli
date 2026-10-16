# コマンド

`dtu --help` が出す一覧と、各サブコマンドの引数です。シェルモードでも同じコマンドが使えます。

## 基本

| コマンド | 説明 |
| --- | --- |
| `dtu` | シェルモード。サービスを起動してコマンドを待つ |
| `dtu status` | デバイス ID、待ち受けポート、ルート、信頼数、稼働秒数 |
| `dtu code [--refresh] [--password <pw>]` | ペアリングコードの表示／再生成（`--password ""` で解除） |
| `dtu version` | バージョンとデバイス ID |
| `dtu log [-n <行数>]` | 認証ログの末尾 |
| `dtu completions <shell>` | 補完スクリプト（bash/zsh/fish/powershell/elvish） |

## デバイス

| コマンド | 説明 |
| --- | --- |
| `dtu device scan [--wait <ms>] [--filter <text>]` | LAN を探索 |
| `dtu device list` | 信頼済みデバイス |
| `dtu device add <host[:port]> [--expires 30d] [--note <text>] [--alias <name>]` | 招待を送り、相手の承認を待つ |
| `dtu device alias <id\|alias> [name]` | 別名の表示／設定（`""` で解除） |
| `dtu device rm <id\|alias>` | 信頼を取り消す（即時） |

## 招待

| コマンド | 説明 |
| --- | --- |
| `dtu invite send <host[:port]> [--expires 30d] [--note] [--alias] [--wait <秒>]` | 招待を送る（`--wait 0` は相手のタイムアウトまで） |
| `dtu invite list` | 承認待ちの招待 |
| `dtu invite accept <id> [--expires 30d]` | 承認 |
| `dtu invite deny <id>` | 拒否 |

## 転送

| コマンド | 説明 |
| --- | --- |
| `dtu push <peer> <paths…> [--to <dir>] [--code] [--password] [-j <n>] [--no-resume] [--verify] [--dry-run] [--trust-after]` | ローカル → 相手（上書き、レジュームあり） |
| `dtu pull <peer> <paths…> [--to <dir>] [--code] [--password] [-j <n>] [--no-resume] [--trust-after]` | 相手 → ローカル（上書き、常に検証） |
| `dtu diff <peer> <local> <remote>` | 2 つのツリーを比較 |
| `dtu async <peer> <local> <remote> [--prefer newer\|local\|remote] [--delete none\|local\|remote] [--dry-run] [-j <n>]` | 差分同期 |

- `<peer>` はデバイス ID、別名、`host[:port]`、`[v6]:port` のいずれか
- `--verify` は両側で全読みするため明確に遅い（loopback で 700 → 70 MiB/s 程度）
- `--dry-run` は認証だけ行い、データは動かさない（ワンタイムコードは消費される）

## 設定

| コマンド | 説明 |
| --- | --- |
| `dtu config` | すべての設定を JSON で表示 |
| `dtu config get <key>` | 1 項目を表示 |
| `dtu config set <key> <value>` | 1 項目を書き換え（次の起動で有効） |
| `dtu config path` | データディレクトリを表示 |

主なキー: `port`、`api_port`、`root`、`parallel`、`unlimit`、`code_one_shot`、`code_rotate_on_connect`、
`code_password`、`invite_timeout`、`session_ttl`、`idle_timeout`、`max_conn_per_ip`、`drain_timeout`。

## シェル内の操作

- `help` で clap のヘルプ、`exit` / `quit` / `q` / Ctrl-D で終了
- 引数は空白区切り。`"a b.txt"` / `'a b.txt'` のように引用すると 1 つの引数になります
- 起動時に表示される `api 127.0.0.1:<port+1>` は第三者向けの REST 面です（`control.token` の鍵を使う）

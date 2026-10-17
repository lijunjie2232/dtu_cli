# 指令

`dtu --help` 印出的清單與各子指令的參數；shell 模式裡用法完全相同。

## 基本

| 指令 | 說明 |
| --- | --- |
| `dtu` | shell 模式：開好服務後等待指令 |
| `dtu status` | 裝置 ID、監聽埠、根目錄、信任數、運行秒數 |
| `dtu code [--refresh] [--password <pw>]` | 顯示／重新產生配對碼（`--password ""` 清除密碼） |
| `dtu version` | 版本與裝置 ID |
| `dtu log [-n <行數>]` | 認證日誌尾端 |
| `dtu completions <shell>` | 補齊腳本（bash/zsh/fish/powershell/elvish） |

## 裝置

| 指令 | 說明 |
| --- | --- |
| `dtu device scan [--wait <ms>] [--filter <text>]` | 掃描區網 |
| `dtu device list` | 已信任裝置 |
| `dtu device add <host[:port]> [--expires 30d] [--note <text>] [--alias <name>]` | 送出邀請並等待對方決定 |
| `dtu device alias <id\|alias> [name]` | 顯示／設定別名（`""` 清除） |
| `dtu device rm <id\|alias>` | 撤銷信任（立即生效） |

## 邀請

| 指令 | 說明 |
| --- | --- |
| `dtu invite send <host[:port]> [--expires 30d] [--note] [--alias] [--wait <秒>]` | 送出邀請（`--wait 0` 等到對方逾時） |
| `dtu invite list` | 待批准的邀請 |
| `dtu invite accept <id> [--expires 30d]` | 批准 |
| `dtu invite deny <id>` | 拒絕 |

## 傳輸

| 指令 | 說明 |
| --- | --- |
| `dtu push <peer> <paths…> [--to <dir>] [--code] [--password] [-j <n>] [--no-resume] [--verify] [--dry-run] [--trust-after]` | 本機 → 對端（覆寫、可續傳） |
| `dtu pull <peer> <paths…> [--to <dir>] [--code] [--password] [-j <n>] [--no-resume] [--trust-after]` | 對端 → 本機（覆寫、一定會驗證） |
| `dtu diff <peer> <local> <remote>` | 比較兩棵樹 |
| `dtu async <peer> <local> <remote> [--prefer newer\|local\|remote] [--delete none\|local\|remote] [--dry-run] [-j <n>]` | 差異同步 |

- `<peer>` 可以是裝置 ID、別名、`host[:port]` 或 `[v6]:port`
- `--verify` 兩邊都多讀一次全檔，明顯較慢（loopback 700 → 70 MiB/s）
- `--dry-run` 只做認證、不動資料（一次性配對碼仍會被消耗）

## 設定

| 指令 | 說明 |
| --- | --- |
| `dtu config` | 以 JSON 印出全部設定 |
| `dtu config get <key>` | 讀一項 |
| `dtu config set <key> <value>` | 改一項（下次啟動生效） |
| `dtu config path` | 印出資料目錄 |

主要鍵：`port`、`api_port`、`root`、`parallel`、`unlimit`、`code_one_shot`、`code_rotate_on_connect`、
`code_password`、`invite_timeout`、`session_ttl`、`idle_timeout`、`max_conn_per_ip`、`drain_timeout`。

## shell 內操作

- `help` 顯示 clap 說明，`exit` / `quit` / `q` / Ctrl-D 離開
- 參數以空白分隔；用 `"a b.txt"` 或 `'a b.txt'` 保留空白
- 啟動時印出的 `api 127.0.0.1:<port+1>` 是給第三方的 REST 介面（金鑰在 `control.token`）

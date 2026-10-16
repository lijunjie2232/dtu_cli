# dtu_cli

在 LAN 的兩台機器之間，做有認證的檔案傳輸的**命令列應用**。
它把 `dtu_core` 載入自己的行程，所以一支程式同時是送方與收方，不需要另一個常駐程式。

## 建置

```bash
cargo build --release
# target/release/dtu
```

## 兩種模式

| 模式 | 啟動方式 | 行為 |
| --- | --- | --- |
| shell | `dtu`（不帶參數） | 開好服務後持續讀指令；`exit` / Ctrl-D 結束 |
| 單次 | `dtu <command> …` | 只在該指令期間載入核心 |

`version` / `config` / `completions` 不會啟動服務。

## 第一次傳輸

收檔端（B）：

```console
$ dtu
dtu 0.1.0 · device 7edjJeQA150uWRQY9E6KtA
  root    C:\Users\me\AppData\Roaming\dtu\share
  listen  0.0.0.0:47000
  api     127.0.0.1:47001
dtu> code
code     RF83BP3R
```

送檔端（A）：

```bash
dtu push 192.168.1.24:47000 ./report.pdf --code RF83BP3R
```

配對一次之後就不用再帶配對碼：

```bash
dtu device add 192.168.1.24:47000 --alias nas --expires 30d
dtu push nas ./report.pdf --to inbox
```

## 永久信任（配對）

- 邀請方：`dtu device add <host:port>` 或 `dtu invite send <host:port>`
- 被邀請方：`dtu invite list` → `dtu invite accept <id>`
- 臨時傳輸後轉為永久：`dtu push … --code <code> --trust-after`

`dtu device list` 可看別名、位址、到期時間；`dtu device rm <id|alias>` 立即撤銷。

## 同步與比較

```bash
dtu diff nas ./docs docs                       # 列出差異
dtu async nas ./docs docs --prefer newer       # 以較新的一方為準
dtu async nas ./docs docs --delete remote      # 刪掉只有對端才有的檔案
```

`--delete remote` 會先拉回本機再刪除，所以最後一份永遠不會消失。

## 設定

```bash
dtu config                       # 以 JSON 印出全部設定
dtu config get port
dtu config set port 47100        # 下次啟動生效
dtu config path                  # 資料目錄
```

## 注意

- 一個資料目錄（`DTU_HOME`，預設 `%APPDATA%\dtu`）同時只能有一個行程使用。
  shell 開著時再跑 `dtu status` 會得到 `another dtu daemon already owns …`。
- 要接收檔案就得有人監聽，所以請讓 shell（或 TUI / GUI）保持開著。
- `dtu completions bash|zsh|fish|powershell|elvish` 可輸出補齊腳本。
- `dtu log -n 20` 看認證紀錄（JSON Lines）。

## 關於本網站

用 MkDocs（Material 主題 + static-i18n）產生，push 到 `main` 時自動發佈到 GitHub Pages。
第一次請把 **Settings → Pages → Source** 設成 **GitHub Actions**。

```bash
pip install -r requirements-docs.txt
mkdocs serve
```

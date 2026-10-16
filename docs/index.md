# dtu_cli

LAN 内の 2 台のマシン間で、認証付きのファイル転送を行う**コマンドラインアプリ**です。
`dtu_core` をプロセス内にホストするので、これ 1 つで送信側にも受信側にもなります。別のデーモンは不要です。

## インストール

```bash
cargo build --release
# target/release/dtu
```

## 2 つのモード

| モード | 起動 | 動き |
| --- | --- | --- |
| シェル | `dtu`（引数なし） | サービスを起動したままコマンドを読み続ける。`exit` / Ctrl-D で停止 |
| 単発 | `dtu <command> …` | そのコマンドの間だけコアを起動し、終わったら停止 |

`version` / `config` / `completions` はどちらのモードでもサービスを起動しません。

## はじめての転送

受ける側（B）:

```console
$ dtu
dtu 0.1.0 · device 7edjJeQA150uWRQY9E6KtA
  root    C:\Users\me\AppData\Roaming\dtu\share
  listen  0.0.0.0:47000
  api     127.0.0.1:47001
dtu> code
code     RF83BP3R
```

送る側（A）:

```bash
dtu push 192.168.1.24:47000 ./report.pdf --code RF83BP3R
```

一度ペアリングすれば、次からはコード不要です。

```bash
dtu device add 192.168.1.24:47000 --alias nas --expires 30d
dtu push nas ./report.pdf --to inbox
```

## 恒久信頼（ペアリング）

- 招待した側: `dtu device add <host:port>` または `dtu invite send <host:port>`
- 招待された側: `dtu invite list` → `dtu invite accept <id>`
- 一時的な転送のあとで恒久化する: `dtu push … --code <code> --trust-after`

`dtu device list` で `alias` / アドレス / 有効期限が確認でき、`dtu device rm <id|alias>` で即時に取り消せます。

## 同期と比較

```bash
dtu diff nas ./docs docs                       # 差分の一覧
dtu async nas ./docs docs --prefer newer       # 新しい側を正として同期
dtu async nas ./docs docs --delete remote      # 相手側だけにあるファイルを消す
```

`--delete remote` は「消す前にローカルへ取り込む」ので、最後の 1 部が失われることはありません。

## 設定

```bash
dtu config                       # 全部を JSON で表示
dtu config get port
dtu config set port 47100        # 次の起動から有効
dtu config path                  # データディレクトリ
```

## 注意

- データディレクトリ（`DTU_HOME`、既定 `%APPDATA%\dtu`）は 1 プロセス専有です。
  シェルを開いたまま別の `dtu status` を実行すると `another dtu daemon already owns …` になります。
- 受信には待ち受けが必要なので、相手から送ってもらうときはシェル（または TUI / GUI）を開いておきます。
- `dtu completions bash|zsh|fish|powershell|elvish` で補完スクリプトを出力できます。
- 認証の記録は `dtu log -n 20`（JSON Lines）。

## このサイトについて

MkDocs（Material テーマ + static-i18n）で生成し、`main` への push で GitHub Pages に公開します。
初回だけ **Settings → Pages → Source** を **GitHub Actions** に設定してください。

```bash
pip install -r requirements-docs.txt
mkdocs serve
```

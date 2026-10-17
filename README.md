[![日本語](https://img.shields.io/badge/lang-日本語-1c7ed6?style=flat-square)](README.md)
[![English](https://img.shields.io/badge/lang-English-868e96?style=flat-square)](README.en.md)
[![繁體中文](https://img.shields.io/badge/lang-繁體中文-868e96?style=flat-square)](README.zh-TW.md)

# dtu_cli

LAN 内の 2 台のマシン間で、認証付きのファイル転送を行う**コマンドラインアプリ**です。
`dtu_core` をプロセス内にホストするので、これ単体で「送る側」も「受け取る側」も務まります。別のデーモンは不要です。

[ドキュメント](docs/index.md) · [dtu_core](../dtu_core) · [dtu_tui](../dtu_tui) · [dtu_gui](../dtu_gui)

## ビルド

```bash
cargo build --release
# target/release/dtu （Windows は dtu.exe）
```

## 2 つの使い方

### 1. シェルとして常駐する（受け取り用）

引数なしで起動すると、そのままサービス（TCP 待ち受け・探索・REST）を立ち上げ、コマンドを読み続けます。

```console
$ dtu
dtu 0.1.0 · device 0tZr635ykNqC3_eYOsNnPQ
  root    C:\Users\me\AppData\Roaming\dtu\share
  listen  0.0.0.0:47000
  api     127.0.0.1:47001
  commands: help · status · code · device · invite · push · pull · async · exit
dtu> code
code     RF83BP3R
dtu> exit
```

### 2. 単発のコマンドとして使う

`dtu push …` のようにサブコマンドを渡すと、そのコマンドの間だけコアを起動して終了します。

```bash
dtu status                                   # この端末の情報
dtu code                                     # ペアリングコード
dtu push 192.168.1.24:47000 ./report.pdf --code RF83BP3R
dtu pull living-room-nas photos/ --to ./backup
dtu async living-room-nas ./docs docs        # 差分同期
dtu device add 192.168.1.24:47000 --alias nas --expires 30d
```

## 注意

- **1 つのデータディレクトリにつき 1 アプリ**。シェルを開いたまま別の `dtu status` を実行すると
  `another dtu daemon already owns …` で拒否されます（意図した動作）。
- `version` / `config` / `completions` はサービスを起動しません（ファイルと標準出力だけ触ります）。
- ポートやルートは `dtu config set port 47100` のように変更でき、次回起動から有効になります。

## ドキュメント

`docs/` は MkDocs（Material + static-i18n）で、日本語 / English / 繁體中文。
`.github/workflows/docs.yml` が `main` への push で GitHub Pages に公開します。

```bash
pip install -r requirements-docs.txt
mkdocs serve
```

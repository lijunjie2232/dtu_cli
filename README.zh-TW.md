[![日本語](https://img.shields.io/badge/lang-日本語-868e96?style=flat-square)](README.md)
[![English](https://img.shields.io/badge/lang-English-868e96?style=flat-square)](README.en.md)
[![繁體中文](https://img.shields.io/badge/lang-繁體中文-1c7ed6?style=flat-square)](README.zh-TW.md)

# dtu_cli

在 LAN 的兩台機器之間，做有認證之檔案傳輸的**命令列應用**。
它把 `dtu_core` 載入自己的行程，所以一支程式同時是「送方」也是「收方」，不需要另外跑常駐程式。

[文件](docs/index.zh-TW.md) · [dtu_core](../dtu_core) · [dtu_tui](../dtu_tui) · [dtu_gui](../dtu_gui)

## 建置

```bash
cargo build --release
# target/release/dtu
```

## 兩種用法

### 1. 當成 shell 常駐（用來收檔）

不帶參數啟動時，會直接開好服務（TCP 監聽、區網探索、REST），並持續讀取指令。

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

### 2. 執行單一指令

帶入子指令（例如 `dtu push …`）時，核心只在這條指令執行期間存活。

```bash
dtu status                                   # 本機資訊
dtu code                                     # 配對碼
dtu push 192.168.1.24:47000 ./report.pdf --code RF83BP3R
dtu pull living-room-nas photos/ --to ./backup
dtu async living-room-nas ./docs docs        # 差異同步
dtu device add 192.168.1.24:47000 --alias nas --expires 30d
```

## 注意

- **一個資料目錄只能有一個應用**。shell 開著時再跑 `dtu status`，會得到
  `another dtu daemon already owns …`（這是刻意設計）。
- `version` / `config` / `completions` 不會啟動服務（只碰檔案與標準輸出）。
- 用 `dtu config set port 47100` 改埠或分享根目錄，下次啟動生效。

## 文件

`docs/` 用 MkDocs（Material + static-i18n），三種語言：日本語 / English / 繁體中文。
`.github/workflows/docs.yml` 會在 push 到 `main` 時發佈到 GitHub Pages。

```bash
pip install -r requirements-docs.txt
mkdocs serve
```

[![日本語](https://img.shields.io/badge/lang-日本語-868e96?style=flat-square)](README.md)
[![English](https://img.shields.io/badge/lang-English-1c7ed6?style=flat-square)](README.en.md)
[![繁體中文](https://img.shields.io/badge/lang-繁體中文-868e96?style=flat-square)](README.zh-TW.md)

# dtu_cli

A **command-line app** for authenticated file transfer between two machines on a LAN.
It hosts `dtu_core` in its own process, so a single binary is both the sender and the receiver —
no separate daemon to run.

[Docs](docs/index.en.md) · [dtu_core](../dtu_core) · [dtu_tui](../dtu_tui) · [dtu_gui](../dtu_gui)

## Build

```bash
cargo build --release
# target/release/dtu
```

## Two ways to use it

### 1. Stay in a shell (to receive files)

With no arguments it starts the services (TCP listener, discovery, REST) and reads commands.

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

### 2. Run one command

Pass a subcommand and the core only lives for the duration of that command.

```bash
dtu status                                   # this device
dtu code                                     # pairing code
dtu push 192.168.1.24:47000 ./report.pdf --code RF83BP3R
dtu pull living-room-nas photos/ --to ./backup
dtu async living-room-nas ./docs docs        # two-way sync
dtu device add 192.168.1.24:47000 --alias nas --expires 30d
```

## Notes

- **One app per data directory.** Running `dtu status` while the shell is open fails with
  `another dtu daemon already owns …` — on purpose.
- `version`, `config` and `completions` never start the services (they only touch files and stdout).
- Change ports or the served root with `dtu config set port 47100`; it applies on the next start.

## Docs

`docs/` is MkDocs (Material + static-i18n) in Japanese / English / 繁體中文.
`.github/workflows/docs.yml` publishes it to GitHub Pages on every push to `main`.

```bash
pip install -r requirements-docs.txt
mkdocs serve
```

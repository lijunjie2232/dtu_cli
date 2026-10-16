# dtu_cli

A **command-line app** for authenticated file transfer between two machines on a LAN.
It hosts `dtu_core` in its own process, so this one binary is both sender and receiver — no separate daemon.

## Install

```bash
cargo build --release
# target/release/dtu
```

## Two modes

| Mode | Start with | Behaviour |
| --- | --- | --- |
| Shell | `dtu` (no arguments) | Starts the services and keeps reading commands. `exit` / Ctrl-D stops it |
| One-shot | `dtu <command> …` | Hosts the core for that command only |

`version`, `config` and `completions` never start the services.

## First transfer

Receiving side (B):

```console
$ dtu
dtu 0.1.0 · device 7edjJeQA150uWRQY9E6KtA
  root    C:\Users\me\AppData\Roaming\dtu\share
  listen  0.0.0.0:47000
  api     127.0.0.1:47001
dtu> code
code     RF83BP3R
```

Sending side (A):

```bash
dtu push 192.168.1.24:47000 ./report.pdf --code RF83BP3R
```

Pair once and the code is no longer needed:

```bash
dtu device add 192.168.1.24:47000 --alias nas --expires 30d
dtu push nas ./report.pdf --to inbox
```

## Permanent trust (pairing)

- Inviter: `dtu device add <host:port>` or `dtu invite send <host:port>`
- Invited: `dtu invite list` → `dtu invite accept <id>`
- Make a temporary transfer permanent: `dtu push … --code <code> --trust-after`

`dtu device list` shows alias, address and expiry; `dtu device rm <id|alias>` revokes immediately.

## Sync and compare

```bash
dtu diff nas ./docs docs                       # list differences
dtu async nas ./docs docs --prefer newer       # the newer side wins
dtu async nas ./docs docs --delete remote      # remove files only the peer has
```

`--delete remote` pulls a copy to the local side before deleting, so the last copy is never lost.

## Configuration

```bash
dtu config                       # everything, as JSON
dtu config get port
dtu config set port 47100        # applies on the next start
dtu config path                  # data directory
```

## Notes

- One process owns the data directory (`DTU_HOME`, default `%APPDATA%\dtu`).
  Running `dtu status` while the shell is open fails with `another dtu daemon already owns …`.
- Receiving needs the listener, so keep the shell (or the TUI / GUI) open when someone sends to you.
- `dtu completions bash|zsh|fish|powershell|elvish` prints a completion script.
- `dtu log -n 20` shows the auth log (JSON Lines).

## About this site

Built with MkDocs (Material theme + static-i18n) and published to GitHub Pages on every push to `main`.
Set **Settings → Pages → Source** to **GitHub Actions** once.

```bash
pip install -r requirements-docs.txt
mkdocs serve
```

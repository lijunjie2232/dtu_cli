# Commands

The list `dtu --help` prints, plus each subcommand's arguments. The same commands work inside the shell.

## Basics

| Command | Purpose |
| --- | --- |
| `dtu` | shell mode: start the services and wait for commands |
| `dtu status` | device id, listening port, root, trust counters, uptime |
| `dtu code [--refresh] [--password <pw>]` | show or rotate the pairing code (`--password ""` clears it) |
| `dtu version` | version and device id |
| `dtu log [-n <lines>]` | tail of the auth log |
| `dtu completions <shell>` | completion script (bash/zsh/fish/powershell/elvish) |

## Devices

| Command | Purpose |
| --- | --- |
| `dtu device scan [--wait <ms>] [--filter <text>]` | probe the LAN |
| `dtu device list` | trusted devices |
| `dtu device add <host[:port]> [--expires 30d] [--note <text>] [--alias <name>]` | send an invitation and wait for the decision |
| `dtu device alias <id\|alias> [name]` | show or set an alias (`""` clears it) |
| `dtu device rm <id\|alias>` | revoke trust (immediately) |

## Invitations

| Command | Purpose |
| --- | --- |
| `dtu invite send <host[:port]> [--expires 30d] [--note] [--alias] [--wait <s>]` | send one (`--wait 0` waits as long as the peer does) |
| `dtu invite list` | pending invitations |
| `dtu invite accept <id> [--expires 30d]` | accept |
| `dtu invite deny <id>` | deny |

## Transfers

| Command | Purpose |
| --- | --- |
| `dtu push <peer> <paths…> [--to <dir>] [--code] [--password] [-j <n>] [--no-resume] [--verify] [--dry-run] [--trust-after]` | local → peer (overwrites, resumes) |
| `dtu pull <peer> <paths…> [--to <dir>] [--code] [--password] [-j <n>] [--no-resume] [--trust-after]` | peer → local (overwrites, always verifies) |
| `dtu diff <peer> <local> <remote>` | compare two trees |
| `dtu async <peer> <local> <remote> [--prefer newer\|local\|remote] [--delete none\|local\|remote] [--dry-run] [-j <n>]` | sync with conflict resolution |

- `<peer>` is a device id, an alias, `host[:port]` or `[v6]:port`
- `--verify` reads the whole file on both sides, so it is clearly slower (700 → 70 MiB/s on loopback)
- `--dry-run` authenticates but moves no data (a one-time code is still consumed)

## Configuration

| Command | Purpose |
| --- | --- |
| `dtu config` | every setting as JSON |
| `dtu config get <key>` | one setting |
| `dtu config set <key> <value>` | change one setting (next start) |
| `dtu config path` | the data directory |

Main keys: `port`, `api_port`, `root`, `parallel`, `unlimit`, `code_one_shot`, `code_rotate_on_connect`,
`code_password`, `invite_timeout`, `session_ttl`, `idle_timeout`, `max_conn_per_ip`, `drain_timeout`.

## Inside the shell

- `help` for clap's help, `exit` / `quit` / `q` / Ctrl-D to leave
- arguments split on whitespace; quote with `"a b.txt"` or `'a b.txt'` to keep spaces
- the `api 127.0.0.1:<port+1>` line shown at startup is the REST surface for third parties (key in `control.token`)

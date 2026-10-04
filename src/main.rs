//! dtu — authenticated file transfer over a LAN. A linux-like CLI that *hosts* the
//! core: every run starts the services in this process, and they stop when it exits.
//! With no subcommand it keeps running and reads commands from stdin, so one session
//! can receive files and drive everything else.

use std::io::Write;
use std::sync::Arc;

use clap::{CommandFactory, Parser, Subcommand};
use dtu_core::cfg::Config;
use dtu_core::state::Decision;
use dtu_core::trust::Store;
use dtu_core::{audit, cfg, client, discover, embed, guard, identity, invite, proto};

#[derive(Parser)]
#[command(
    name = "dtu",
    version,
    about = "Authenticated file transfer between machines on a LAN"
)]
struct Cli {
    /// Without a subcommand dtu stays up and reads commands from stdin
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show this device, its port, root and trust counters
    Status,
    /// Show or refresh the one-time pairing code (pass --password "" to clear)
    Code {
        #[arg(long)]
        refresh: bool,
        #[arg(long)]
        password: Option<String>,
    },
    /// Print version and this device's id
    Version,
    /// Trusted devices and LAN discovery
    Device {
        #[command(subcommand)]
        sub: DeviceCmd,
    },
    /// Pending invitations
    Invite {
        #[command(subcommand)]
        sub: InviteCmd,
    },
    /// Upload local files/dirs to a peer (overwrites remote, resumes partial files)
    Push {
        peer: String,
        #[arg(required = true)]
        paths: Vec<String>,
        #[arg(long, default_value = ".")]
        to: String,
        #[arg(long)]
        code: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(short = 'j', long)]
        parallel: Option<usize>,
        #[arg(long)]
        no_resume: bool,
        /// After a code-authenticated transfer, offer to trust this device
        #[arg(long)]
        trust_after: bool,
        #[arg(long, default_value = "30d")]
        trust_expires: String,
        /// Verify the transfer end to end with sha256 (one extra full read per side;
        /// opt-in because on hardware without SHA extensions it can cost more than the wire)
        #[arg(long)]
        verify: bool,
        /// Show what would be sent without moving any data (still authenticates,
        /// so a one-time pairing code is consumed)
        #[arg(long)]
        dry_run: bool,
    },
    /// Download remote files/dirs from a peer (overwrites local)
    Pull {
        peer: String,
        #[arg(required = true)]
        paths: Vec<String>,
        #[arg(long, default_value = ".")]
        to: String,
        #[arg(long)]
        code: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(short = 'j', long)]
        parallel: Option<usize>,
        #[arg(long)]
        no_resume: bool,
        /// After a code-authenticated transfer, offer to trust this device
        #[arg(long)]
        trust_after: bool,
        #[arg(long, default_value = "30d")]
        trust_expires: String,
    },
    /// Compare a local directory with a remote one
    Diff {
        peer: String,
        local: String,
        remote: String,
        #[arg(long)]
        code: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    /// Sync two trees, resolving conflicts (newest side wins by default)
    Async {
        peer: String,
        local: String,
        remote: String,
        #[arg(long, default_value = "newer")]
        prefer: String,
        #[arg(long)]
        dry_run: bool,
        /// Remove files that exist on one side only: `local` mirrors the peer,
        /// `remote` mirrors the local tree
        #[arg(long, default_value = "none")]
        delete: String,
        #[arg(long)]
        code: Option<String>,
        #[arg(long)]
        password: Option<String>,
        #[arg(short = 'j', long)]
        parallel: Option<usize>,
    },
    /// Show the auth log
    Log {
        #[arg(short = 'n', long, default_value_t = 20)]
        lines: usize,
    },
    /// Read or write configuration
    Config {
        #[command(subcommand)]
        sub: Option<ConfigCmd>,
    },
    /// Print a shell completion script (bash, zsh, fish, powershell, elvish)
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum DeviceCmd {
    /// Broadcast-probe the LAN for devices
    Scan {
        #[arg(long, default_value_t = 1500)]
        wait: u64,
        #[arg(long)]
        filter: Option<String>,
    },
    /// List trusted devices
    List,
    /// Invite a device by address (or add by id when it is already reachable)
    Add {
        target: String,
        #[arg(long, default_value = "30d")]
        expires: String,
        #[arg(long)]
        note: Option<String>,
        /// Name to use in place of the device id (unique, never an id)
        #[arg(long)]
        alias: Option<String>,
    },
    /// Name a trusted device, or print its name when none is given ("" clears it)
    Alias {
        device: String,
        name: Option<String>,
    },
    /// Forget a trusted device
    Rm { id: String },
}

#[derive(Subcommand)]
enum InviteCmd {
    /// Send an invitation and wait for the peer to accept
    Send {
        target: String,
        #[arg(long, default_value = "30d")]
        expires: String,
        #[arg(long)]
        note: Option<String>,
        /// Name to use in place of the device id (unique, never an id)
        #[arg(long)]
        alias: Option<String>,
        /// Seconds to wait for the peer's decision (0 = until the peer times out)
        #[arg(long, default_value_t = 600)]
        wait: u64,
    },
    /// List inbound invitations waiting for a decision
    List,
    /// Accept an inbound invitation
    Accept {
        id: String,
        #[arg(long, default_value = "30d")]
        expires: String,
    },
    /// Deny an inbound invitation
    Deny { id: String },
}

#[derive(Subcommand)]
enum ConfigCmd {
    List,
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
    },
    /// Print the data directory
    Path,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    if let Err(e) = app(cli).await {
        eprintln!("dtu: {e}");
        std::process::exit(1);
    }
}

/// Every run owns the core: the services start here and stop when we exit.
async fn app(cli: Cli) -> Result<(), String> {
    let cfg = Config::load();
    match cli.cmd {
        // nothing to serve for these: they only touch files and stdout
        Some(cmd @ (Cmd::Version | Cmd::Config { .. } | Cmd::Completions { .. })) => {
            run(cmd, None, &cfg).await
        }
        None => {
            let daemon = start(&cfg)?;
            let out = shell(&daemon, &cfg).await;
            daemon.stop();
            out
        }
        Some(cmd) => {
            let daemon = start(&cfg)?;
            let out = run(cmd, Some(&daemon), &cfg).await;
            daemon.stop();
            out
        }
    }
}

/// Bring the core up in this process. Fails when another dtu app owns the data dir.
fn start(cfg: &Config) -> Result<embed::Daemon, String> {
    let daemon = embed::host(cfg.clone())?;
    daemon.wait_ready()?;
    Ok(daemon)
}

/// The little shell: the core is already serving, so commands just act on it.
async fn shell(daemon: &embed::Daemon, cfg: &Config) -> Result<(), String> {
    let st = &daemon.state;
    println!("dtu {} · device {}", dtu_core::VERSION, st.id.id);
    println!("  root    {}", guard::for_display(&st.root));
    println!("  listen  0.0.0.0:{}", cfg.port);
    println!("  api     127.0.0.1:{}", daemon.api_port());
    println!("  commands: help · status · code · device · invite · push · pull · async · exit");
    loop {
        print!("dtu> ");
        std::io::stdout().flush().ok();
        let mut line = String::new();
        if std::io::stdin()
            .read_line(&mut line)
            .map_err(|e| e.to_string())?
            == 0
        {
            println!();
            return Ok(());
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if matches!(line, "exit" | "quit" | "q") {
            return Ok(());
        }
        let args = split_args(line)?;
        match Cli::try_parse_from(std::iter::once("dtu".to_string()).chain(args)) {
            Ok(parsed) => {
                if let Some(cmd) = parsed.cmd {
                    if let Err(e) = run(cmd, Some(daemon), cfg).await {
                        println!("dtu: {e}");
                    }
                }
            }
            Err(e) => print!("{e}"),
        }
    }
}

/// Split a shell line into arguments; `"..."` keeps spaces together.
fn split_args(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in line.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '"' | '\'') => quote = Some(c),
            (None, c) if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            (None, c) => cur.push(c),
        }
    }
    if quote.is_some() {
        return Err("unterminated quote".into());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    Ok(out)
}

async fn run(cmd: Cmd, daemon: Option<&embed::Daemon>, cfg: &Config) -> Result<(), String> {
    let state = daemon.map(|d| d.state.clone());
    let core = |what: &str| {
        state
            .clone()
            .ok_or_else(|| format!("{what} needs the core, which is not running"))
    };
    match cmd {
        Cmd::Version => {
            let id = identity::Identity::load_or_create()?;
            println!("dtu {}", dtu_core::VERSION);
            println!("device {id}", id = id.id);
            Ok(())
        }
        Cmd::Status => {
            let st = core("status")?;
            println!("device   {}", st.id.id);
            println!("version  {} (proto v{})", dtu_core::VERSION, proto::PROTO);
            println!("port     {}", st.cfg.port);
            println!("root     {}", guard::for_display(&st.root));
            println!("unlimit  {}", st.cfg.unlimit);
            println!("trusted  {}", st.trust.lock().await.devices.len());
            println!("pending  {}", st.invites.lock().await.len());
            println!("uptime   {}s", identity::now().saturating_sub(st.started));
            Ok(())
        }
        Cmd::Code { refresh, password } => {
            let st = core("the pairing code")?;
            if refresh {
                // Refresh only rotates the code: the password survives unless
                // explicitly cleared with an empty string.
                let update = match password {
                    None => None,
                    Some(p) if p.is_empty() => Some(None),
                    Some(p) => Some(Some(p)),
                };
                st.rotate_code(update).await;
            }
            let c = st.code.lock().await;
            println!("code     {}", c.code);
            if c.password.is_some() {
                println!("password required");
            }
            if c.consumed {
                println!("state    consumed (refresh to reuse)");
            }
            Ok(())
        }
        Cmd::Log { lines } => {
            for line in audit::tail(lines) {
                println!("{line}");
            }
            Ok(())
        }
        Cmd::Config { sub } => {
            let mut cfg = Config::load();
            match sub.unwrap_or(ConfigCmd::List) {
                ConfigCmd::List => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?
                    );
                    Ok(())
                }
                ConfigCmd::Get { key } => match cfg.get(&key) {
                    Some(v) => {
                        println!("{v}");
                        Ok(())
                    }
                    None => Err(format!("unknown key: {key}")),
                },
                ConfigCmd::Set { key, value } => {
                    cfg.set(&key, &value)?;
                    cfg.save().map_err(|e| format!("save config: {e}"))?;
                    println!("{key} = {value}");
                    Ok(())
                }
                ConfigCmd::Path => {
                    println!("{}", cfg::data_dir().display());
                    Ok(())
                }
            }
        }
        Cmd::Device { sub } => device_cmd(sub, cfg, core("device")?).await,
        Cmd::Invite { sub } => invite_cmd(sub, cfg, core("invite")?).await,
        Cmd::Push {
            peer,
            paths,
            to,
            code,
            password,
            parallel,
            no_resume,
            trust_after,
            trust_expires,
            verify,
            dry_run,
        } => {
            let me = Arc::new(identity::Identity::load_or_create()?);
            let trust = Store::load();
            let target =
                client::Target::resolve(cfg, &trust, &peer, code.as_deref(), password.as_deref())?;
            if dry_run {
                for row in client::plan_push(&target, me, &paths, &to).await? {
                    let state = if row.up_to_date() {
                        "up to date"
                    } else {
                        "would send"
                    };
                    println!(
                        "{state}\t{}\tlocal={} remote={}",
                        row.remote, row.local_size, row.remote_size
                    );
                }
                return Ok(());
            }
            let plan = client::Plan {
                parallel: parallel.unwrap_or(cfg.parallel).max(1),
                resume: !no_resume,
                verify,
            };
            for line in client::push(&target, me.clone(), &paths, &to, plan).await? {
                println!("pushed {line}");
            }
            if trust_after {
                offer_trust(&target, &me, cfg.port, &trust_expires, &core("push")?).await?;
            }
            Ok(())
        }
        Cmd::Pull {
            peer,
            paths,
            to,
            code,
            password,
            parallel,
            no_resume,
            trust_after,
            trust_expires,
        } => {
            let me = Arc::new(identity::Identity::load_or_create()?);
            let trust = Store::load();
            let target =
                client::Target::resolve(cfg, &trust, &peer, code.as_deref(), password.as_deref())?;
            let plan = client::Plan {
                parallel: parallel.unwrap_or(cfg.parallel).max(1),
                resume: !no_resume,
                verify: true,
            };
            for line in client::pull(&target, me.clone(), &paths, &to, plan).await? {
                println!("pulled {line}");
            }
            if trust_after {
                offer_trust(&target, &me, cfg.port, &trust_expires, &core("pull")?).await?;
            }
            Ok(())
        }

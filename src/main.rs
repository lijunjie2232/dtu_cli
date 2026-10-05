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
        Cmd::Diff {
            peer,
            local,
            remote,
            code,
            password,
        } => {
            let me = identity::Identity::load_or_create()?;
            let trust = Store::load();
            let target =
                client::Target::resolve(cfg, &trust, &peer, code.as_deref(), password.as_deref())?;
            let rows = client::diff(&target, &me, &local, &remote).await?;
            for r in &rows {
                println!(
                    "{} {:<8} local={} remote={}",
                    r.kind, r.rel, r.local, r.remote
                );
            }
            println!("{} difference(s)", rows.len());
            Ok(())
        }
        Cmd::Async {
            peer,
            local,
            remote,
            prefer,
            dry_run,
            delete,
            code,
            password,
            parallel,
        } => {
            let me = Arc::new(identity::Identity::load_or_create()?);
            let trust = Store::load();
            let target =
                client::Target::resolve(cfg, &trust, &peer, code.as_deref(), password.as_deref())?;
            let plan = client::Plan {
                parallel: parallel.unwrap_or(cfg.parallel).max(1),
                resume: true,
                verify: false,
            };
            let prefer = client::Prefer::parse(&prefer)?;
            let delete = client::DeleteSide::parse(&delete)?;
            for line in client::sync(
                &target,
                me,
                &local,
                &remote,
                client::SyncOpts {
                    prefer,
                    dry_run,
                    delete,
                },
                plan,
            )
            .await?
            {
                println!("{line}");
            }
            Ok(())
        }
        Cmd::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "dtu", &mut std::io::stdout());
            Ok(())
        }
    }
}

/// Spec: "once the temporary transfer completes, offer to trust this device".
/// `port` is our listening port: it lets the peer keep a dialable address for us.
async fn offer_trust(
    target: &client::Target,
    me: &identity::Identity,
    port: u16,
    expires: &str,
    st: &Arc<dtu_core::State>,
) -> Result<(), String> {
    if target.is_trusted() {
        return Ok(());
    }
    let Some(peer) = target.peer_id().await else {
        return Ok(());
    };
    use std::io::{IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        println!(
            "peer {peer} is still untrusted; run: dtu device add {} --expires {expires}",
            target.addr
        );
        return Ok(());
    }
    print!("trust {peer} permanently? [y/N] ");
    std::io::stdout().flush().ok();
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .map_err(|e| e.to_string())?;
    if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
        println!("kept as temporary access");
        return Ok(());
    }

    let mut store = st.trust.lock().await;
    let expires_at = cfg::parse_expiry(expires)?;
    let note = Some("approved after a temporary transfer".to_string());
    let paired = client::invite_send(
        &target.addr,
        expires_at,
        note,
        me,
        &mut store,
        port,
        |id, from| {
            println!("pending id {id} — accept it on {from} with: dtu invite accept {id}");
        },
    )
    .await?;
    println!("trusted {paired}");
    Ok(())
}

async fn device_cmd(sub: DeviceCmd, cfg: &Config, st: Arc<dtu_core::State>) -> Result<(), String> {
    match sub {
        DeviceCmd::Scan { wait, filter } => {
            let trust = st.trust.lock().await.clone();
            let items = discover::scan(cfg.port, wait, &st.id.id, &trust).await;
            let mut shown = 0;
            for s in items {
                if let Some(f) = &filter {
                    if !s.id.contains(f.as_str()) && !s.addr.contains(f.as_str()) {
                        continue;
                    }
                }
                println!(
                    "{:<24} {:<24} {}",
                    s.id,
                    s.addr,
                    if s.trusted { "trusted" } else { "" }
                );
                shown += 1;
            }
            println!("{shown} device(s)");
            Ok(())
        }
        DeviceCmd::List => {
            let trust = st.trust.lock().await.clone();
            if trust.devices.is_empty() {
                println!("no trusted devices");
            }
            for (id, d) in &trust.devices {
                let expires = if d.expires_at == 0 {
                    "never".to_string()
                } else {
                    audit::iso(d.expires_at)
                };
                let live = if trust.get(id).is_some() {
                    ""
                } else {
                    " [expired]"
                };
                println!(
                    "{:<24} {:<16} {:<24} expires {expires}{live}",
                    id,
                    d.alias.as_deref().unwrap_or("-"),
                    d.addr.clone().unwrap_or_else(|| "-".into())
                );
            }
            Ok(())
        }
        DeviceCmd::Add {
            target,
            expires,
            note,
            alias,
        } => send_invite(&target, &expires, note, alias.as_deref(), 600, cfg, &st).await,
        DeviceCmd::Alias { device, name } => {
            let mut trust = st.trust.lock().await;
            let id = trust
                .resolve(&device)
                .map(|(id, _)| id.to_string())
                .ok_or_else(|| format!("no such device or alias: {device}"))?;
            match name {
                None => {
                    let alias = trust.devices.get(&id).and_then(|d| d.alias.clone());
                    println!("{}", alias.unwrap_or_else(|| "(none)".into()));
                }
                Some(name) => {
                    trust.set_alias(&id, Some(&name))?;
                    trust.save()?;
                    println!(
                        "{id} {}",
                        match trust.devices.get(&id).and_then(|d| d.alias.as_deref()) {
                            Some(a) => format!("is now {a}"),
                            None => "alias cleared".to_string(),
                        }
                    );
                }
            }
            Ok(())
        }
        DeviceCmd::Rm { id } => {
            let mut trust = st.trust.lock().await;
            let id = trust
                .resolve(&id)
                .map(|(id, _)| id.to_string())
                .unwrap_or(id);
            if trust.remove(&id) {
                trust.save()?;
                println!("removed {id}");
                Ok(())
            } else {
                Err(format!("no such device: {id}"))
            }
        }
    }
}

async fn invite_cmd(sub: InviteCmd, cfg: &Config, st: Arc<dtu_core::State>) -> Result<(), String> {
    match sub {
        InviteCmd::Send {
            target,
            expires,
            note,
            alias,
            wait,
        } => send_invite(&target, &expires, note, alias.as_deref(), wait, cfg, &st).await,
        InviteCmd::List => {
            let items = invite::list(&st).await;
            if items.is_empty() {
                println!("no pending invitations");
            }
            for i in items {
                println!(
                    "{} from {} at {} (requested expiry {})",
                    i.id, i.from, i.addr, i.expires_at
                );
            }
            Ok(())
        }
        InviteCmd::Accept { id, expires } => {
            let expires_at = cfg::parse_expiry(&expires)?;
            invite::decide(&st, &id, Decision::Accept { expires_at }).await?;
            println!("accepted invite {id}");
            Ok(())
        }
        InviteCmd::Deny { id } => {
            invite::decide(&st, &id, Decision::Deny).await?;
            println!("denied invite {id}");
            Ok(())
        }
    }
}

async fn send_invite(
    target: &str,
    expires: &str,
    note: Option<String>,
    alias: Option<&str>,
    wait: u64,
    cfg: &Config,
    st: &Arc<dtu_core::State>,
) -> Result<(), String> {
    let me = identity::Identity::load_or_create()?;
    // The live store: pairing must be visible to the listener in this same process.
    let mut trust = st.trust.lock().await;
    let addr = client::normalize_addr(target, cfg.port);
    let expires_at = cfg::parse_expiry(expires)?;
    println!("inviting {addr} ...");
    let invite = client::invite_send(
        &addr,
        expires_at,
        note,
        &me,
        &mut trust,
        cfg.port,
        |id, peer| {
            println!("pending id {id} — accept it on {peer} with: dtu invite accept {id}");
        },
    );
    let peer = if wait == 0 {
        invite.await
    } else {
        match tokio::time::timeout(std::time::Duration::from_secs(wait), invite).await {
            Ok(r) => r,
            Err(_) => {
                return Err(format!(
                    "no answer after {wait}s; the invitation may still be waiting on the peer (dtu invite list)"
                ))
            }
        }
    }?;
    if let Some(a) = alias {
        trust
            .set_alias(&peer, Some(a))
            .map_err(|e| format!("paired with {peer}, but {e}"))?;
        trust.save()?;
    }
    match trust.get(&peer) {
        Some(d) => {
            let exp = if d.expires_at == 0 {
                "never".to_string()
            } else {
                audit::iso(d.expires_at)
            };
            match &d.alias {
                Some(a) => println!("paired with {peer} as {a} (expires {exp})"),
                None => println!("paired with {peer} (expires {exp})"),
            }
            Ok(())
        }
        None => Err("handshake finished but no trust recorded".into()),
    }
}

#[cfg(test)]
mod cli_tests {
    use super::*;

    fn script(shell: clap_complete::Shell) -> String {
        let mut cmd = Cli::command();
        let mut out = Vec::new();
        clap_complete::generate(shell, &mut cmd, "dtu", &mut out);
        String::from_utf8(out).expect("completion script is utf-8")
    }

    #[test]
    fn every_shell_gets_a_usable_script() {
        for shell in [
            clap_complete::Shell::Bash,
            clap_complete::Shell::Zsh,
            clap_complete::Shell::Fish,
            clap_complete::Shell::PowerShell,
        ] {
            let text = script(shell);
            assert!(
                text.len() > 500,
                "{shell:?} script looks empty: {} bytes",
                text.len()
            );
            for command in ["push", "pull", "async", "device", "invite", "completions"] {
                assert!(
                    text.contains(command),
                    "{shell:?} script is missing {command}"
                );
            }
        }
    }


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


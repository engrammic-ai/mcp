use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "engrammic")]
#[command(about = "Engrammic CLI - setup, update, and manage your Engrammic MCP integration")]
#[command(version)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Skip prompts and auto-configure detected tools
    #[arg(short = 'y', long = "yes", global = true)]
    pub yes: bool,

    /// Specify tool directly (see `harnesses --json` for the full list)
    #[arg(long, global = true)]
    pub tool: Option<String>,

    /// Custom skill installation path (overrides harness defaults)
    #[arg(long, global = true)]
    pub skill_path: Option<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Interactive setup - configure Engrammic for your editors
    Install,
    /// Update to latest endpoint
    Update,
    /// Remove Engrammic from one or more editors (keeps other editors intact)
    Remove {
        /// Harness IDs to remove from (e.g. --harness claude --harness cursor).
        /// When omitted, shows an interactive multi-select over all known harnesses.
        #[arg(long = "harness", value_name = "ID")]
        harness: Vec<String>,
    },
    /// Remove Engrammic from ALL editors, skills, config, and optionally the binary
    Uninstall {
        /// Also tear down the self-hosted Docker stack and delete data volumes
        #[arg(long)]
        purge_data: bool,
    },
    /// Show installation status
    Status,
    /// Install skills only (no MCP config changes)
    Skills,
    /// Guided self-hosted setup wizard
    Selfhost {
        /// Use Podman instead of Docker (skips Docker daemon check, adds SELinux volume labels,
        /// uses CDI GPU syntax). Start the socket first:
        ///   podman system service --time=0 unix:///tmp/podman.sock &
        ///   export DOCKER_HOST=unix:///tmp/podman.sock
        #[arg(long)]
        podman: bool,
    },
    /// Alias for 'selfhost' (kept for compatibility)
    #[command(hide = true)]
    Docker,
    /// Upgrade self-hosted Docker stack to latest version
    Upgrade,
    /// Show container resource usage and scaling recommendations
    Scale,
    /// Run diagnostic checks
    Doctor,
    /// View service logs (self-hosted only)
    Logs {
        /// Service name (app, dagster, memgraph, etc.)
        #[arg(short, long)]
        service: Option<String>,
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        lines: u32,
    },
    /// View or update license key (self-hosted only)
    License,
    /// List detected harnesses
    List,
    /// Print all harness facts as JSON (consumed by docs drift check)
    #[command(hide = true)]
    Harnesses {
        /// Emit JSON (always JSON; flag accepted for the documented invocation)
        #[arg(long)]
        json: bool,
    },
}

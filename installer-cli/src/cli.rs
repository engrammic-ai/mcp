use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "engrammic")]
#[command(about = "Engrammic CLI - setup, update, and manage your Engrammic MCP integration")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

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
    /// Install Engrammic MCP (default if no subcommand)
    Install,
    /// Update to latest endpoint
    Update,
    /// Remove Engrammic from config
    Uninstall,
    /// Show installation status
    Status,
    /// Install skills only (no MCP config changes)
    Skills,
    /// Install self-hosted Docker stack
    Docker,
    /// Upgrade self-hosted Docker stack to latest version
    Upgrade,
    /// Show container resource usage and scaling recommendations
    Scale,
    /// Run diagnostic checks
    Doctor,
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

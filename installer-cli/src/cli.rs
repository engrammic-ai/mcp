use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "engrammic-install")]
#[command(about = "Engrammic MCP installer")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Accept defaults without prompting
    #[arg(short = 'y', long = "yes", global = true)]
    pub yes: bool,

    /// Specify tool directly (claude, cursor, windsurf, antigravity, gemini, pi)
    #[arg(long, global = true)]
    pub tool: Option<String>,
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
}

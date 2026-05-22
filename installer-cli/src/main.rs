mod cli;
mod config;
mod skills;
mod tools;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Select};

use cli::{Cli, Commands};
use tools::{Tool, ENDPOINT};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or(Commands::Install) {
        Commands::Install => install(cli.yes, cli.tool.as_deref()),
        Commands::Update => update(cli.tool.as_deref()),
        Commands::Uninstall => uninstall(cli.tool.as_deref()),
        Commands::Status => status(),
    }
}

fn install(yes: bool, tool_id: Option<&str>) -> Result<()> {
    println!();
    println!("{}", "Engrammic MCP Installer".bold());
    println!();

    let tool = select_tool(yes, tool_id)?;

    println!("Installing for {}...", tool.name.cyan());

    config::install(&tool.config_path, ENDPOINT)?;

    println!("{} Added engrammic to {}", "✓".green(), tool.config_path.display());
    println!();
    println!("Tools available: {}", "remember, recall, learn, believe, trace, link".dimmed());

    Ok(())
}

fn update(tool_id: Option<&str>) -> Result<()> {
    println!();
    println!("{}", "Engrammic MCP Updater".bold());
    println!();

    let tool = select_tool(false, tool_id)?;

    if !config::is_installed(&tool.config_path, ENDPOINT) {
        println!("{} Engrammic not installed for {}", "!".yellow(), tool.name);
        return Ok(());
    }

    config::install(&tool.config_path, ENDPOINT)?;
    println!("{} Updated engrammic in {}", "✓".green(), tool.config_path.display());

    Ok(())
}

fn uninstall(tool_id: Option<&str>) -> Result<()> {
    println!();
    println!("{}", "Engrammic MCP Uninstaller".bold());
    println!();

    let tool = select_tool(false, tool_id)?;

    config::uninstall(&tool.config_path)?;
    println!("{} Removed engrammic from {}", "✓".green(), tool.config_path.display());

    Ok(())
}

fn status() -> Result<()> {
    println!();
    println!("{}", "Engrammic MCP Status".bold());
    println!();

    let tools = Tool::all();
    let mut any_installed = false;

    for tool in tools {
        let installed = config::is_installed(&tool.config_path, ENDPOINT);
        let status = if installed {
            any_installed = true;
            "✓ installed".green()
        } else if tool.config_path.parent().map(|p| p.exists()).unwrap_or(false) {
            "- not configured".dimmed()
        } else {
            "- not detected".dimmed()
        };

        println!("  {} {}", status, tool.name);
    }

    if !any_installed {
        println!();
        println!("Run {} to install", "engrammic-install".cyan());
    }

    Ok(())
}

fn select_tool(yes: bool, tool_id: Option<&str>) -> Result<Tool> {
    // If tool specified via --tool flag
    if let Some(id) = tool_id {
        return Tool::from_id(id)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}. Use: claude, cursor, windsurf, antigravity", id));
    }

    let detected = Tool::detect_installed();

    // If -y flag and tools detected, use first one
    if yes {
        if let Some(tool) = detected.first() {
            println!("Auto-selected: {}", tool.name.cyan());
            return Ok(tool.clone());
        }
        // Fall through to show all tools
    }

    // Interactive selection
    let all_tools = Tool::all();
    let items: Vec<&str> = all_tools.iter().map(|t| t.name).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select your tool")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(all_tools[selection].clone())
}

mod banner;
mod cli;
mod cli_install;
mod config;
mod deeplink;
mod docker;
mod doctor;
mod flow;
mod license;
mod logs;
mod manifest;
mod scale;
mod selfhost;
mod skill_format;
mod skills;
mod tools;
mod user_config;

pub use skill_format::*;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use dialoguer::{Confirm, Input, MultiSelect, Select};

use cli::{Cli, Commands};
use tools::{
    ConfigShape, DeepLinkKind, InstallMethod, SkillDest, Tool, CLOUD_ENDPOINT, LOCAL_ENDPOINT,
};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let auto = cli.yes;

    // Interactive commands need a terminal for prompts (dialoguer reads /dev/tty).
    // Detect up front so users get one clear message instead of a prompt crash.
    let interactive_command = matches!(
        cli.command,
        Commands::Install
            | Commands::Update
            | Commands::Uninstall
            | Commands::Skills
            | Commands::Selfhost
            | Commands::Docker
            | Commands::License
    );
    if interactive_command && !auto && !console::user_attended_stderr() {
        eprintln!(
            "{} No interactive terminal detected.",
            "error:".red().bold()
        );
        eprintln!(
            "  Re-run with {} to auto-configure detected editors:",
            "-y".cyan()
        );
        eprintln!(
            "    {}",
            "curl -fsSL https://get.engrammic.ai/install.sh | sh -s -- -y".cyan()
        );
        eprintln!(
            "  Or run {} from an interactive terminal.",
            "engrammic install".cyan()
        );
        std::process::exit(1);
    }

    let result = match cli.command {
        Commands::Install => install(auto, cli.tool.as_deref(), cli.skill_path.as_deref()),
        Commands::Update => update(auto, cli.tool.as_deref(), cli.skill_path.as_deref()),
        Commands::Uninstall => uninstall(auto, cli.tool.as_deref()),
        Commands::Status => status(),
        Commands::Skills => install_skills_only(auto, cli.skill_path.as_deref()),
        Commands::Selfhost => selfhost::run_wizard(),
        Commands::Docker => selfhost::run_wizard(),
        Commands::Upgrade => upgrade_docker(),
        Commands::Scale => scale::show_status(),
        Commands::Doctor => doctor::run_diagnostics(),
        Commands::Logs {
            service,
            follow,
            lines,
        } => logs::show_logs(service.as_deref(), follow, lines),
        Commands::License => manage_license(),
        Commands::List => list(),
        Commands::Harnesses { .. } => print_harnesses_json(),
    };

    if let Err(ref e) = result {
        let msg = e.to_string().to_lowercase();

        // Handle Ctrl+C gracefully
        if msg.contains("interrupted") || msg.contains("operation canceled") {
            println!();
            println!("{}", "Cancelled.".dimmed());
            std::process::exit(130); // Standard exit code for Ctrl+C
        }

        // Handle TTY issues
        if msg.contains("input reader") || msg.contains("terminal") || msg.contains("tty") {
            eprintln!(
                "{} Could not initialize terminal input.",
                "error:".red().bold()
            );
            eprintln!(
                "  Run with {} to auto-configure detected harnesses.",
                "-y".cyan()
            );
            eprintln!("  Or ensure you're running in an interactive terminal.");
            std::process::exit(1);
        }
    }

    result
}

fn list() -> Result<()> {
    banner::print_banner();

    let detected = Tool::detect_installed();
    let all = Tool::all();

    println!("{}", "Detected harnesses".bold());
    println!();
    for tool in &all {
        let is_detected = detected.iter().any(|d| d.id == tool.id);
        let marker = if is_detected {
            "✓".green()
        } else {
            "-".dimmed()
        };
        println!("  {} {:<24} ({})", marker, tool.name, tool.id);
    }
    println!();
    println!(
        "Run {} to configure detected harnesses.",
        "engrammic install".cyan()
    );
    Ok(())
}

fn handle_returning_user(
    config: user_config::UserConfig,
    tool_id: Option<&str>,
    skill_path: Option<&str>,
) -> Result<()> {
    let endpoint = config.endpoint.as_deref().unwrap_or(CLOUD_ENDPOINT);
    let is_self_hosted = endpoint == LOCAL_ENDPOINT;

    println!("{}", "Current setup".bold());
    println!(
        "  Mode: {}",
        if is_self_hosted {
            "Self-hosted".cyan()
        } else {
            "Cloud".cyan()
        }
    );
    println!("  Endpoint: {}", endpoint.dimmed());

    if is_self_hosted {
        if let Some(ref key) = config.license_key {
            match license::validate_license_format(key) {
                Ok(info) => {
                    println!(
                        "  License: {} ({} days remaining)",
                        info.customer.cyan(),
                        info.days_remaining
                    );
                }
                Err(_) => {
                    println!("  License: {}", "invalid or expired".yellow());
                }
            }
        }
    }
    println!();

    let mut options = vec![
        "Add or update harnesses",
        "Refresh skills",
        "View full status",
    ];

    if is_self_hosted {
        options.push("Update license key");
    }

    options.push("Start fresh (reconfigure everything)");

    let idx = Select::new()
        .with_prompt("What would you like to do?")
        .items(&options)
        .default(0)
        .interact()?;
    let choice = options[idx];

    match choice {
        "Add or update harnesses" => {
            let selection = select_tools(false, tool_id)?;

            if selection.to_install.is_empty() && selection.to_remove.is_empty() {
                println!("{} No changes made.", "!".yellow());
                return Ok(());
            }

            // One manifest load for all mutations in this flow.
            let mut m = manifest::Manifest::load_or_migrate(None)?;

            // Handle removals first
            if !selection.to_remove.is_empty() {
                println!("{}", "Removing from deselected harnesses".bold());
                for tool in &selection.to_remove {
                    match remove_tool_outcome(tool, &mut m) {
                        Ok(flow::Outcome::Done) => {
                            println!("  {} {} {}", "✓".green(), tool.name, "(removed)".dimmed());
                        }
                        Ok(_) => {} // Manual outcomes already printed inside remove_tool_outcome
                        Err(e) => {
                            println!("  {} {} — {}", "✗".red(), tool.name, e);
                        }
                    }
                }
                println!();
            }

            if !selection.to_install.is_empty() {
                println!("{}", "Configuring harnesses".bold());
                for tool in &selection.to_install {
                    install_tool(tool, endpoint, &mut m);
                }
                println!();
            }

            m.save()?;
            println!("{} Done.", "✓".green());
        }

        "Refresh skills" => {
            install_skills_step(false, skill_path)?;
            println!();
            println!("{} Skills refreshed.", "✓".green());
        }

        "View full status" => {
            status()?;
        }

        "Update license key" => {
            manage_license()?;
        }

        "Start fresh (reconfigure everything)" => {
            let endpoint = match select_deployment_mode(&config)? {
                DeploymentChoice::Cloud(ep) => ep,
                DeploymentChoice::SelfHost => return selfhost::run_wizard(),
            };
            run_full_install(endpoint, false, tool_id, skill_path)?;
        }

        _ => {}
    }

    Ok(())
}

fn install(auto: bool, tool_id: Option<&str>, skill_path: Option<&str>) -> Result<()> {
    banner::print_banner();

    let existing_config = user_config::UserConfig::load().unwrap_or_default();
    let has_existing_setup = existing_config.endpoint.is_some();

    // For returning users (not -y mode), show menu
    if has_existing_setup && !auto {
        return handle_returning_user(existing_config, tool_id, skill_path);
    }

    // Fresh install or -y mode
    let endpoint = if auto {
        existing_config
            .endpoint
            .unwrap_or_else(|| CLOUD_ENDPOINT.to_string())
    } else {
        match select_deployment_mode(&existing_config)? {
            DeploymentChoice::Cloud(ep) => ep,
            DeploymentChoice::SelfHost => return selfhost::run_wizard(),
        }
    };

    run_full_install(endpoint, auto, tool_id, skill_path)
}

fn run_full_install(
    endpoint: String,
    auto: bool,
    tool_id: Option<&str>,
    skill_path: Option<&str>,
) -> Result<()> {
    // ---- Interview: every question, zero side effects ----
    let selection = select_tools(auto, tool_id)?;

    if selection.to_install.is_empty() && selection.to_remove.is_empty() {
        println!(
            "{} No editors selected — nothing was changed.",
            "!".yellow()
        );
        println!(
            "  Run {} anytime to configure editors.",
            "engrammic install".cyan()
        );
        println!();
        println!("Add this to your MCP config manually:");
        println!();
        println!(
            r#"  "engrammic": {{ "type": "http", "url": "{}" }}"#,
            endpoint
        );
        println!();
        return Ok(());
    }

    let skill_dests = ask_skill_dests(auto, skill_path)?;

    let answers = flow::Answers {
        endpoint: endpoint.clone(),
        to_install: selection.to_install,
        to_remove: selection.to_remove,
        skill_dests,
    };

    // ---- Plan summary ----
    println!();
    print!("{}", flow::render_plan(&answers));
    println!();
    if !auto {
        let proceed = Confirm::new()
            .with_prompt("Proceed?")
            .default(true)
            .interact()?;
        if !proceed {
            println!("{}", "Nothing was changed.".dimmed());
            return Ok(());
        }
        println!();
    }

    // ---- Execute: skip-and-continue, one manifest load/save ----
    let mut m = manifest::Manifest::load_or_migrate(None)?;
    let mut results: Vec<flow::StepResult> = Vec::new();

    for tool in &answers.to_remove {
        let outcome = match remove_tool_outcome(tool, &mut m) {
            Ok(o) => o,
            Err(e) => flow::Outcome::Failed(format!("{e:#}")),
        };
        results.push(flow::StepResult {
            label: format!("remove {}", tool.name),
            outcome,
        });
    }

    for tool in &answers.to_install {
        let tool_id_str = tool.id.to_string();
        let outcome = install_tool(tool, &answers.endpoint, &mut m);
        // Embed the retry hint into Failed messages so the summary can print it
        // without needing a separate id field on StepResult.
        let outcome = match outcome {
            flow::Outcome::Failed(msg) => flow::Outcome::Failed(format!(
                "{msg}\n    {} {}",
                "→ retry:".dimmed(),
                format!("engrammic install --tool {tool_id_str}").cyan()
            )),
            other => other,
        };
        results.push(flow::StepResult {
            label: tool.name.to_string(),
            outcome,
        });
    }

    if !answers.skill_dests.is_empty() || skill_path.is_some() {
        let outcome = match install_skills_to(&answers.skill_dests, &mut m, skill_path) {
            Ok(()) => flow::Outcome::Done,
            Err(e) => flow::Outcome::Failed(format!("{e:#}")),
        };
        results.push(flow::StepResult {
            label: "skills".to_string(),
            outcome,
        });
    }

    m.save()?;

    // Save endpoint so returning users get the menu (merges via manifest).
    let existing = user_config::UserConfig::load().unwrap_or_default();
    let config = user_config::UserConfig {
        endpoint: Some(endpoint),
        license_key: existing.license_key,
        selfhost_dir: existing.selfhost_dir,
    };
    config.save()?;

    // ---- Summary ----
    println!();
    let (done, failed, manual) = flow::summarize_results(&results);
    for r in &results {
        match &r.outcome {
            flow::Outcome::Done => {}
            flow::Outcome::Failed(msg) => {
                println!("  {} {} — {}", "✗".red(), r.label, msg);
            }
            flow::Outcome::Manual(msg) => {
                println!("  {} {} — {}", "▸".cyan(), r.label, msg);
            }
        }
    }
    println!(
        "{} {} configured, {} need a manual step, {} failed.",
        if failed == 0 {
            "✓".green()
        } else {
            "!".yellow()
        },
        done,
        manual,
        failed
    );

    println!();
    println!(
        "Done. Tools available: {}",
        "remember, recall, learn, believe, trace, link".dimmed()
    );
    print_restart_reminder();
    println!();
    cli_install::offer_cli_install(auto)?;
    Ok(())
}

enum DeploymentChoice {
    Cloud(String),
    SelfHost,
}

fn select_deployment_mode(_existing_config: &user_config::UserConfig) -> Result<DeploymentChoice> {
    let modes = vec![
        "Cloud - free tier, no setup (recommended)",
        "Self-hosted - run locally with Docker (license required)",
    ];
    println!(
        "  {}",
        "(Self-hosted requires Docker and a license key)".dimmed()
    );
    let idx = Select::new()
        .with_prompt("Where should Engrammic run?")
        .items(&modes)
        .default(0)
        .interact()?;
    if idx == 1 {
        Ok(DeploymentChoice::SelfHost)
    } else {
        Ok(DeploymentChoice::Cloud(CLOUD_ENDPOINT.to_string()))
    }
}

fn update(auto: bool, tool_id: Option<&str>, skill_path: Option<&str>) -> Result<()> {
    banner::print_banner();

    let selection = select_tools(auto, tool_id)?;

    // One manifest load for all mutations in this flow.
    let mut m = manifest::Manifest::load_or_migrate(None)?;

    // Remove deselected harnesses
    if !selection.to_remove.is_empty() {
        println!("{}", "Removing from deselected harnesses".bold());
        for tool in &selection.to_remove {
            match remove_tool_outcome(tool, &mut m) {
                Ok(flow::Outcome::Done) => {
                    println!("  {} {} {}", "✓".green(), tool.name, "(removed)".dimmed());
                }
                Ok(_) => {} // Manual outcomes already printed inside remove_tool_outcome
                Err(e) => {
                    println!("  {} {} — {}", "✗".red(), tool.name, e);
                }
            }
        }
        println!();
    }

    for tool in &selection.to_install {
        match tool.method {
            InstallMethod::FileEdit(shape) => {
                if let Some(ep) = detect_installed_endpoint(tool) {
                    let backup = config::ensure_backup(&tool.config_path)?;
                    let _ = config::install(&tool.config_path, &ep, shape)?;
                    println!("{} Refreshed engrammic in {}", "✓".green(), tool.name);
                    m.record_harness(tool.id, &tool.config_path, backup, &ep);
                } else {
                    println!("{} Not installed for {}", "!".yellow(), tool.name);
                }
            }
            InstallMethod::DeepLink(_) => {
                println!(
                    "{} {} is managed in-app; re-run {} to re-open the install link",
                    "-".dimmed(),
                    tool.name,
                    "engrammic install".cyan()
                );
            }
            InstallMethod::PrintInstructions(_) => {
                println!(
                    "{} {} is GUI-managed; no file to update",
                    "-".dimmed(),
                    tool.name
                );
            }
        }
    }

    // Refresh skills - use custom path if provided, otherwise refresh existing
    if let Some(custom_path) = skill_path {
        let path = std::path::PathBuf::from(custom_path);
        let results = skills::install_skills_to_paths(&[path])?;
        for (p, count) in results {
            println!(
                "{} Refreshed {} skills in {}",
                "✓".green(),
                count,
                p.display()
            );
        }
    } else {
        let dests_with_skills: Vec<SkillDest> = SkillDest::all()
            .into_iter()
            .filter(|d| skills::count_skills_formatted(d) > 0)
            .collect();

        if !dests_with_skills.is_empty() {
            let results = skills::install_skills(&dests_with_skills)?;
            for (path, count) in results {
                println!(
                    "{} Refreshed {} skills in {}",
                    "✓".green(),
                    count,
                    path.display()
                );
            }
            for dest in &dests_with_skills {
                m.record_skill(
                    dest.harness,
                    &dest.path,
                    manifest::skill_format_str(dest.format),
                    manifest::skill_scope_str(dest.scope),
                );
            }
        }
    }

    m.save()?;

    Ok(())
}

/// Remove engrammic from a single tool's config, using a caller-owned manifest.
/// Returns Outcome::Done on success, Outcome::Manual for deep-link/print tools,
/// or Err on unexpected IO failure.
fn remove_tool_outcome(tool: &Tool, m: &mut manifest::Manifest) -> Result<flow::Outcome> {
    match tool.method {
        InstallMethod::FileEdit(shape) => {
            config::uninstall(&tool.config_path, shape)?;
            m.forget_harness(tool.id);
            Ok(flow::Outcome::Done)
        }
        InstallMethod::DeepLink(_) => {
            println!("  {} {} - remove via app settings", "!".yellow(), tool.name);
            Ok(flow::Outcome::Manual("remove via app settings".to_string()))
        }
        InstallMethod::PrintInstructions(hint) => {
            println!("  {} {} - remove via {}", "!".yellow(), tool.name, hint);
            Ok(flow::Outcome::Manual(format!("remove via {hint}")))
        }
    }
}

fn uninstall(auto: bool, tool_id: Option<&str>) -> Result<()> {
    banner::print_banner();

    let selection = select_tools(auto, tool_id)?;
    // For uninstall, we remove everything that was selected
    let tools_to_remove = if selection.to_install.is_empty() {
        // Nothing selected means remove all installed
        Tool::all()
            .into_iter()
            .filter(|t| detect_installed_endpoint(t).is_some())
            .collect::<Vec<_>>()
    } else {
        selection.to_install
    };

    let mut m = manifest::Manifest::load_or_migrate(None)?;

    for tool in &tools_to_remove {
        match remove_tool_outcome(tool, &mut m) {
            Ok(flow::Outcome::Done) => {
                println!("  {} {} {}", "✓".green(), tool.name, "(removed)".dimmed());
            }
            Ok(_) => {} // Manual outcomes already printed inside remove_tool_outcome
            Err(e) => {
                println!("  {} {} — {}", "✗".red(), tool.name, e);
            }
        }
    }

    for dest in SkillDest::all() {
        let removed = skills::remove_skills_formatted(&dest)?;
        if removed > 0 {
            println!(
                "{} Removed {} skills from {}",
                "✓".green(),
                removed,
                dest.path.display()
            );
            m.forget_skill(&dest.path);
        }
    }
    m.save()?;

    Ok(())
}

/// Register the engrammic MCP server for one tool. Never returns Err for
/// per-harness problems — those become Outcome::Failed so other steps continue.
/// Prints the per-step result inline (Created/Updated/Unchanged) for FileEdit;
/// keeps existing deep-link/print-instructions output unchanged.
fn install_tool(tool: &Tool, endpoint: &str, m: &mut manifest::Manifest) -> flow::Outcome {
    match tool.method {
        InstallMethod::FileEdit(shape) => {
            let backup = match config::ensure_backup(&tool.config_path) {
                Ok(b) => b,
                Err(e) => return flow::Outcome::Failed(format!("backup failed: {e:#}")),
            };
            let result = match config::install(&tool.config_path, endpoint, shape) {
                Ok(r) => r,
                Err(e) => return flow::Outcome::Failed(format!("{e:#}")),
            };
            match result {
                config::InstallResult::Created => {
                    println!(
                        "  {} {} {}",
                        "✓".green(),
                        tool.name,
                        "(added engrammic)".dimmed()
                    );
                }
                config::InstallResult::Updated { old_url } => {
                    println!(
                        "  {} {} {}",
                        "✓".green(),
                        tool.name,
                        format!("(updated: {} -> {})", old_url, endpoint).dimmed()
                    );
                }
                config::InstallResult::Unchanged => {
                    println!(
                        "  {} {} {}",
                        "-".dimmed(),
                        tool.name,
                        "(already configured)".dimmed()
                    );
                }
            }
            println!("    {}", tool.config_path.display().to_string().dimmed());
            m.record_harness(tool.id, &tool.config_path, backup, endpoint);
            flow::Outcome::Done
        }
        InstallMethod::DeepLink(DeepLinkKind::VsCode) => {
            let links = deeplink::vscode_links(endpoint);
            if deeplink::try_open(&links.uri) {
                println!(
                    "  {} {} {}",
                    "✓".green(),
                    tool.name,
                    "(opening VS Code - approve the prompt to add the server)".dimmed()
                );
            } else {
                println!(
                    "  {} {} {}",
                    "▸".cyan(),
                    tool.name,
                    "(manual step - open this link to add the server)".dimmed()
                );
            }
            println!("    {}", links.redirect.cyan());
            flow::Outcome::Manual("requires an in-app step (shown above)".to_string())
        }
        InstallMethod::DeepLink(DeepLinkKind::Cursor) => {
            let links = deeplink::cursor_links(endpoint);
            if deeplink::try_open(&links.uri) {
                println!(
                    "  {} {} {}",
                    "✓".green(),
                    tool.name,
                    "(opening Cursor - approve the prompt to add the server)".dimmed()
                );
            } else {
                println!(
                    "  {} {} {}",
                    "▸".cyan(),
                    tool.name,
                    "(manual step - open this link to add the server)".dimmed()
                );
            }
            println!("    {}", links.redirect.cyan());
            flow::Outcome::Manual("requires an in-app step (shown above)".to_string())
        }
        InstallMethod::PrintInstructions(hint) => {
            let block = serde_json::json!({
                "mcpServers": {
                    "engrammic": {
                        "url": endpoint
                    }
                }
            });
            println!(
                "  {} {} {}",
                "▸".cyan(),
                tool.name,
                "(manual step - add via GUI)".dimmed()
            );
            println!("    Add via: {}", hint.cyan());
            println!(
                "    {}",
                serde_json::to_string_pretty(&block)
                    .unwrap_or_default()
                    .dimmed()
            );
            flow::Outcome::Manual("requires an in-app step (shown above)".to_string())
        }
    }
}

fn detect_installed_endpoint(tool: &Tool) -> Option<String> {
    // Deep-link harnesses (VS Code) manage config through their own UI; we cannot
    // read back whether the server is registered, so report unknown.
    let InstallMethod::FileEdit(shape) = tool.method else {
        return None;
    };
    config::get_installed_endpoint(&tool.config_path, shape)
}

fn status() -> Result<()> {
    banner::print_banner();

    println!("{}", "Harnesses".bold());
    let mut any_installed = false;
    let mut has_cloud = false;
    let mut has_local = false;
    for tool in Tool::all() {
        let present = tool
            .config_path
            .parent()
            .map(|p| p.exists())
            .unwrap_or(false);
        let label = match tool.method {
            // Deep-link harnesses are managed in-app; we can only report presence.
            InstallMethod::DeepLink(_) => {
                if present {
                    format!("{:<18}", "▸ deep-link").cyan()
                } else {
                    format!("{:<18}", "- not detected").dimmed()
                }
            }
            // Print-instructions harnesses are GUI-managed; report marker presence only.
            InstallMethod::PrintInstructions(_) => {
                if present {
                    format!("{:<18}", "▸ manual (GUI)").cyan()
                } else {
                    format!("{:<18}", "- not detected").dimmed()
                }
            }
            InstallMethod::FileEdit(_) => match detect_installed_endpoint(&tool) {
                Some(ep) => {
                    any_installed = true;
                    if ep == CLOUD_ENDPOINT {
                        has_cloud = true;
                        format!("{:<18}", "✓ cloud").green()
                    } else {
                        has_local = true;
                        format!("{:<18}", "✓ self-hosted").green()
                    }
                }
                None if present => format!("{:<18}", "- not configured").dimmed(),
                None => format!("{:<18}", "- not detected").dimmed(),
            },
        };
        println!("  {} {}", label, tool.name);
    }

    if has_cloud {
        println!();
        println!("  Cloud endpoint: {}", CLOUD_ENDPOINT.cyan());
    }
    if has_local {
        println!();
        println!("  Self-hosted endpoint: {}", LOCAL_ENDPOINT.cyan());
        println!(
            "  To change: edit the {} key in your harness config",
            "engrammic.url".cyan()
        );
    }

    println!();
    println!("{}", "Skills".bold());
    for dest in SkillDest::all() {
        let count = skills::count_skills_formatted(&dest);
        let label = if count > 0 {
            format!("{:<18}", format!("✓ {} skills", count)).green()
        } else {
            format!("{:<18}", "- none").dimmed()
        };
        println!("  {} {}", label, dest.name);
    }

    if !any_installed {
        println!();
        println!("Run {} to install", "engrammic".cyan());
    }

    Ok(())
}

/// Result of tool selection: tools to install and tools to remove.
struct ToolSelection {
    to_install: Vec<Tool>,
    to_remove: Vec<Tool>,
}

fn select_tools(auto: bool, tool_id: Option<&str>) -> Result<ToolSelection> {
    // Explicit --tool flag wins.
    if let Some(id) = tool_id {
        let tool = Tool::from_id(id)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}. Use: {}", id, Tool::valid_ids()))?;
        return Ok(ToolSelection {
            to_install: vec![tool],
            to_remove: vec![],
        });
    }

    let detected = Tool::detect_installed();

    // Find tools that currently have engrammic installed
    let installed: Vec<Tool> = Tool::all()
        .into_iter()
        .filter(|t| detect_installed_endpoint(t).is_some())
        .collect();
    let installed_ids: std::collections::HashSet<_> = installed.iter().map(|t| t.id).collect();

    // -y mode: take all detected, no prompt, no removals.
    if auto {
        if detected.is_empty() {
            println!(
                "{} No harnesses detected. Run {} to see available tools.",
                "!".yellow(),
                "engrammic list".cyan()
            );
            return Ok(ToolSelection {
                to_install: vec![],
                to_remove: vec![],
            });
        }
        for tool in &detected {
            println!("Auto-selected: {}", tool.name.cyan());
        }
        return Ok(ToolSelection {
            to_install: detected,
            to_remove: vec![],
        });
    }

    // Interactive mode: label detected/configured tools; nothing pre-checked.
    let all_tools = Tool::all();
    let detected_ids: std::collections::HashSet<_> = detected.iter().map(|t| t.id).collect();
    let options: Vec<String> = all_tools
        .iter()
        .map(|t| flow::harness_label(t, detected_ids.contains(t.id), installed_ids.contains(t.id)))
        .collect();

    println!(
        "  {}",
        "(↑↓ move · space toggle · enter confirm (deselect a configured one to remove))".dimmed()
    );
    let selection_indices = MultiSelect::new()
        .with_prompt("Select editors to configure")
        .items(&options)
        .interact()?;

    let selected: std::collections::HashSet<usize> = selection_indices.into_iter().collect();

    let to_install: Vec<Tool> = all_tools
        .iter()
        .enumerate()
        .filter(|(i, _)| selected.contains(i))
        .map(|(_, t)| t.clone())
        .collect();

    // Tools to remove: were installed, but now deselected
    let to_remove: Vec<Tool> = all_tools
        .iter()
        .enumerate()
        .filter(|(i, t)| installed_ids.contains(t.id) && !selected.contains(i))
        .map(|(_, t)| t.clone())
        .collect();

    Ok(ToolSelection {
        to_install,
        to_remove,
    })
}

/// Prompting half of the skills step: ask the user which destinations to install
/// skills to. Returns a vec of chosen destinations (may be empty on decline or
/// zero-selection). In auto mode returns detected (default) dests. When
/// `skill_path` is Some, the custom-path branch is handled entirely inside
/// `install_skills_to`, so we skip prompts and return `Ok(vec![])`.
fn ask_skill_dests(auto: bool, skill_path: Option<&str>) -> Result<Vec<SkillDest>> {
    // Custom path bypasses the destination prompts entirely.
    if skill_path.is_some() {
        return Ok(vec![]);
    }

    let proceed = if auto {
        true
    } else {
        Confirm::new()
            .with_prompt("Also install 21 Engrammic skills?")
            .default(true)
            .interact()?
    };

    if !proceed {
        println!("  {} Skipped skills.", "-".dimmed());
        return Ok(vec![]);
    }

    let all_dests = SkillDest::all();
    if auto {
        return Ok(all_dests.into_iter().filter(|d| d.default).collect());
    }

    let options: Vec<String> = all_dests
        .iter()
        .map(|d| {
            let scope = match d.scope {
                tools::SkillScope::User => "(user)",
                tools::SkillScope::Project => "(project)",
            };
            let detected = if d.default { "  (detected)" } else { "" };
            format!("{:<25} {}{}", d.name, scope.dimmed(), detected.dimmed())
        })
        .collect();

    println!("  {}", "(↑↓ move · space toggle · enter confirm)".dimmed());
    let options_strs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
    let picked_indices = MultiSelect::new()
        .with_prompt("Install skills to")
        .items(&options_strs)
        .interact()?;

    let chosen: Vec<SkillDest> = all_dests
        .into_iter()
        .enumerate()
        .filter(|(i, _)| picked_indices.contains(i))
        .map(|(_, d)| d)
        .collect();

    if chosen.is_empty() {
        println!("  {} No skill destination selected.", "-".dimmed());
    }

    Ok(chosen)
}

/// Acting half of the skills step: download/copy skills to the given destinations
/// and record them in the caller-owned manifest. When `skill_path` is Some the
/// custom path is used instead of `dests` (and is NOT recorded in the manifest,
/// matching the existing behavior).
fn install_skills_to(
    dests: &[SkillDest],
    m: &mut manifest::Manifest,
    skill_path: Option<&str>,
) -> Result<()> {
    if let Some(custom_path) = skill_path {
        let path = std::path::PathBuf::from(custom_path);
        let results = skills::install_skills_to_paths(&[path])?;
        println!("{}", "Installing skills".bold());
        for (p, count) in results {
            println!(
                "  {} {} skills  {}",
                "✓".green(),
                count,
                p.display().to_string().dimmed()
            );
        }
        return Ok(());
    }

    if dests.is_empty() {
        return Ok(());
    }

    let results = skills::install_skills(dests)?;

    println!("{}", "Installing skills".bold());
    for (path, count) in results {
        println!(
            "  {} {} skills  {}",
            "✓".green(),
            count,
            path.display().to_string().dimmed()
        );
    }
    for dest in dests {
        m.record_skill(
            dest.harness,
            &dest.path,
            manifest::skill_format_str(dest.format),
            manifest::skill_scope_str(dest.scope),
        );
    }
    Ok(())
}

/// Convenience wrapper for call sites that don't participate in the
/// interview→execute flow (e.g. "Refresh skills" in the returning-user menu).
fn install_skills_step(auto: bool, skill_path: Option<&str>) -> Result<()> {
    let dests = ask_skill_dests(auto, skill_path)?;
    if dests.is_empty() && skill_path.is_none() {
        return Ok(());
    }
    let mut m = manifest::Manifest::load_or_migrate(None)?;
    install_skills_to(&dests, &mut m, skill_path)?;
    m.save()?;
    Ok(())
}

fn install_skills_only(auto: bool, skill_path: Option<&str>) -> Result<()> {
    banner::print_banner();

    println!("{}", "Skills-only install".bold());
    println!(
        "  {}",
        "This installs skills without modifying MCP config.".dimmed()
    );
    println!();

    let dests = ask_skill_dests(auto, skill_path)?;

    if dests.is_empty() && skill_path.is_none() {
        println!("{} No skill destination selected.", "!".yellow());
        println!(
            "  Run {} anytime to install them.",
            "engrammic skills".cyan()
        );
        return Ok(());
    }

    let mut m = manifest::Manifest::load_or_migrate(None)?;
    install_skills_to(&dests, &mut m, skill_path)?;
    m.save()?;
    print_restart_reminder();
    Ok(())
}

fn print_restart_reminder() {
    println!();
    println!("{}", "────────────────────────────────────────".dimmed());
    println!(
        "{} {}",
        "⟳".cyan(),
        "Restart your editor to apply changes.".bold()
    );
    println!("{}", "────────────────────────────────────────".dimmed());
}

fn upgrade_docker() -> Result<()> {
    banner::print_banner();

    let config = user_config::UserConfig::load().unwrap_or_default();
    let dir = user_config::UserConfig::dir();

    if config.endpoint.as_deref() != Some(LOCAL_ENDPOINT) {
        println!("{} No self-hosted installation found.", "!".yellow());
        println!(
            "  Run {} first to install the Docker stack.",
            "engrammic docker".cyan()
        );
        return Ok(());
    }

    // Check if compose file has updates available
    if let Some(new_services) = docker::check_compose_updates(&dir)? {
        println!(
            "{} New services available: {}",
            "!".yellow(),
            new_services.join(", ").cyan()
        );

        println!(
            "  {}",
            "(Your .env will be preserved. Old compose backed up to .bak)".dimmed()
        );
        let update_compose = Confirm::new()
            .with_prompt("Update docker-compose.yml to include new services?")
            .default(true)
            .interact()?;

        if update_compose {
            docker::refresh_compose(&dir)?;
            println!("  {} docker-compose.yml updated", "✓".green());
        }
        println!();
    }

    docker::upgrade_docker_stack(&dir)?;

    println!();
    println!(
        "{} Self-hosted stack upgraded to latest version.",
        "✓".green()
    );

    Ok(())
}

fn print_harnesses_json() -> Result<()> {
    use serde_json::json;
    let tools = Tool::all();
    let entries: Vec<serde_json::Value> = tools
        .iter()
        .map(|tool| {
            let (format, container, method_str) = match tool.method {
                InstallMethod::FileEdit(shape) => match shape {
                    ConfigShape::JsonMap { container, .. } => {
                        ("json", Some(container), "file-edit")
                    }
                    ConfigShape::CodexToml => ("toml", Some("mcp_servers"), "file-edit"),
                    ConfigShape::GooseYaml => ("yaml", Some("extensions"), "file-edit"),
                    ConfigShape::OpenCodeJson => ("json", Some("mcp"), "file-edit"),
                    ConfigShape::ContinueYaml => ("yaml", Some("mcpServers"), "file-edit"),
                },
                InstallMethod::DeepLink(_) => ("none", None, "deep-link"),
                InstallMethod::PrintInstructions(_) => ("none", None, "print"),
            };
            let config_path_str = tool.config_path.display().to_string();
            match container {
                Some(c) => json!({
                    "id": tool.id,
                    "name": tool.name,
                    "config_path": config_path_str,
                    "format": format,
                    "container": c,
                    "method": method_str,
                }),
                None => json!({
                    "id": tool.id,
                    "name": tool.name,
                    "config_path": config_path_str,
                    "format": format,
                    "container": null,
                    "method": method_str,
                }),
            }
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&entries)?);
    Ok(())
}

fn manage_license() -> Result<()> {
    banner::print_banner();

    let config = user_config::UserConfig::load().unwrap_or_default();

    if config.endpoint.as_deref() != Some(LOCAL_ENDPOINT) {
        println!(
            "{} License management is only for self-hosted installations.",
            "!".yellow()
        );
        println!("  Cloud users don't need a license key.");
        return Ok(());
    }

    println!("{}", "Current license".bold());
    if let Some(ref key) = config.license_key {
        match license::validate_license_format(key) {
            Ok(info) => {
                println!("  Customer: {}", info.customer.cyan());
                println!("  Days remaining: {}", info.days_remaining);
                println!();
            }
            Err(e) => {
                println!("  {} {}", "!".yellow(), e);
                println!();
            }
        }
    } else {
        println!("  {} No license key configured.", "-".dimmed());
        println!();
    }

    let update = Confirm::new()
        .with_prompt("Update license key?")
        .default(false)
        .interact()?;

    if !update {
        return Ok(());
    }

    println!(
        "  {}",
        "(Starts with ENGR_ - get yours at engrammic.ai/self-hosted)".dimmed()
    );
    let mut prompt = Input::<String>::new().with_prompt("License key (input visible)");

    if let Some(ref key) = config.license_key {
        prompt = prompt.default(key.clone());
    }

    let new_key = prompt.interact_text()?;

    println!("{}", "Validating license".bold());
    match license::validate_license_format(&new_key) {
        Ok(info) => {
            println!(
                "  {} Valid - customer: {}, {} days remaining",
                "✓".green(),
                info.customer.cyan(),
                info.days_remaining
            );
        }
        Err(e) => {
            println!("  {} {}", "✗".red(), e);
            return Ok(());
        }
    }
    println!();

    let dir = user_config::UserConfig::dir();
    docker::update_license_key(&dir, &new_key)?;

    let new_config = user_config::UserConfig {
        endpoint: config.endpoint,
        license_key: Some(new_key),
        selfhost_dir: config.selfhost_dir,
    };
    new_config.save()?;

    println!(
        "{} License key updated. Restart Docker services to apply.",
        "✓".green()
    );
    println!(
        "  Run: {}",
        format!(
            "docker compose -f {}/docker-compose.yml restart",
            dir.display()
        )
        .cyan()
    );

    Ok(())
}

//! Self-hosted setup wizard - guided installation flow.

use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Confirm, Select, Text};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::docker;
use crate::license;
use crate::render_config;
use crate::tools::Tool;
use crate::user_config::UserConfig;

const DEFAULT_PORT: u16 = 8000;
const DEFAULT_DAGSTER_PORT: u16 = 3000;

#[derive(Debug, Clone)]
pub struct SelfHostConfig {
    pub license_key: String,
    pub port: u16,
    pub dagster_port: u16,
    pub install_dir: PathBuf,
    pub llm_provider: Option<LlmProvider>,
    pub telemetry_enabled: bool,
    pub postgres_password: String,
}

#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAI { api_key: String, model: String },
    Anthropic { api_key: String, model: String },
    VertexAI { project: String, location: String },
}

pub fn run_wizard() -> Result<()> {
    print_welcome();

    // Step 1: Prerequisites
    println!();
    println!("{}", "Step 1/6: Prerequisites".bold());
    println!();
    check_prerequisites()?;

    // Step 2: License
    println!();
    println!("{}", "Step 2/6: License".bold());
    println!();
    let license_key = prompt_license()?;

    // Step 3: Configuration
    println!();
    println!("{}", "Step 3/6: Configuration".bold());
    println!();
    let port = prompt_port()?;
    let dagster_port = prompt_dagster_port(port)?;
    let install_dir = prompt_install_dir()?;
    let postgres_password = prompt_postgres_password()?;

    // Step 4: LLM (optional)
    println!();
    println!("{}", "Step 4/6: LLM Provider (optional)".bold());
    println!(
        "  {}",
        "SAGE uses an LLM for synthesis, deduplication, and insight generation.".dimmed()
    );
    println!(
        "  {}",
        "Without an LLM, Engrammic runs in passive mode (storage + recall only).".dimmed()
    );
    println!();
    let llm_provider = prompt_llm_provider()?;

    // Step 5: Telemetry
    println!();
    println!("{}", "Step 5/6: Telemetry".bold());
    println!();
    let telemetry_enabled = Confirm::new("Share anonymous usage statistics?")
        .with_default(true)
        .with_help_message("Helps improve Engrammic. No content or user data collected.")
        .with_render_config(render_config())
        .prompt()?;

    let config = SelfHostConfig {
        license_key,
        port,
        dagster_port,
        install_dir,
        llm_provider,
        telemetry_enabled,
        postgres_password,
    };

    // Step 6: Install
    println!();
    println!("{}", "Step 6/6: Install".bold());
    println!();
    write_config_files(&config)?;

    // Offer to start
    println!();
    let start_now = Confirm::new("Start Engrammic now?")
        .with_default(true)
        .with_render_config(render_config())
        .prompt()?;

    if start_now {
        start_and_wait(&config)?;
        configure_editors(&config)?;
    } else {
        print_manual_start_instructions(&config);
    }

    // Save user config
    let user_config = UserConfig {
        endpoint: Some(format!("http://localhost:{}/mcp", config.port)),
        license_key: Some(config.license_key.clone()),
    };
    user_config.save()?;

    print_quick_reference(&config);

    Ok(())
}

fn print_welcome() {
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_black()
    );
    println!();
    println!("  {}", "Engrammic Self-Hosted Setup".bold());
    println!(
        "  {}",
        "Memory infrastructure for AI agents".dimmed()
    );
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_black()
    );
    println!();
    println!("  This wizard will:");
    println!("    1. Check Docker is running");
    println!("    2. Validate your license");
    println!("    3. Configure ports and storage");
    println!("    4. Set up LLM integration (optional)");
    println!("    5. Start the services");
    println!("    6. Configure your code editor");
    println!();
}

fn check_prerequisites() -> Result<()> {
    // Docker
    print!("  Checking Docker... ");
    if !docker::check_docker()? {
        println!("{}", "not found".red());
        println!();
        println!(
            "  {} Docker is required. Install from: {}",
            "!".yellow(),
            "https://docs.docker.com/get-docker/".cyan()
        );
        anyhow::bail!("Docker not available");
    }
    println!("{}", "ok".green());

    // Docker Compose v2
    print!("  Checking Docker Compose... ");
    let compose_check = Command::new("docker")
        .args(["compose", "version"])
        .output();
    match compose_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version_short = version.trim().split_whitespace().last().unwrap_or("v2");
            println!("{} ({})", "ok".green(), version_short.dimmed());
        }
        _ => {
            println!("{}", "not found".red());
            println!();
            println!(
                "  {} Docker Compose v2 is required. Upgrade Docker Desktop or install the compose plugin.",
                "!".yellow()
            );
            anyhow::bail!("Docker Compose v2 not available");
        }
    }

    // Memory check
    print!("  Checking available memory... ");
    let mem_gb = get_available_memory_gb();
    if mem_gb < 4.0 {
        println!("{} ({:.1} GB)", "low".yellow(), mem_gb);
        println!(
            "  {} Engrammic needs ~5GB RAM. Performance may be degraded.",
            "!".yellow()
        );
    } else {
        println!("{} ({:.1} GB)", "ok".green(), mem_gb);
    }

    Ok(())
}

fn get_available_memory_gb() -> f64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<f64>() {
                            return kb / 1024.0 / 1024.0;
                        }
                    }
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
            if let Ok(bytes_str) = String::from_utf8(output.stdout) {
                if let Ok(bytes) = bytes_str.trim().parse::<f64>() {
                    return bytes / 1024.0 / 1024.0 / 1024.0;
                }
            }
        }
    }
    8.0 // fallback assumption
}

fn prompt_license() -> Result<String> {
    let existing = UserConfig::load().ok().and_then(|c| c.license_key);

    if let Some(ref key) = existing {
        if let Ok(info) = license::validate_license_format(key) {
            println!(
                "  Found existing license: {} ({} days remaining)",
                info.customer.cyan(),
                info.days_remaining
            );
            let keep = Confirm::new("  Use this license?")
                .with_default(true)
                .with_render_config(render_config())
                .prompt()?;
            if keep {
                return Ok(key.clone());
            }
        }
    }

    loop {
        let key = Text::new("License key")
            .with_help_message("Starts with ENGR_ - get yours at engrammic.ai/self-hosted")
            .with_render_config(render_config())
            .prompt()?;

        match license::validate_license_format(&key) {
            Ok(info) => {
                println!(
                    "  {} Valid - {}, {} days remaining",
                    "✓".green(),
                    info.customer.cyan(),
                    info.days_remaining
                );
                return Ok(key);
            }
            Err(e) => {
                println!("  {} {}", "✗".red(), e);
                println!();
            }
        }
    }
}

fn prompt_port() -> Result<u16> {
    let port_str = Text::new("MCP server port")
        .with_default(&DEFAULT_PORT.to_string())
        .with_help_message("Your editor will connect to this port")
        .with_render_config(render_config())
        .prompt()?;

    let port: u16 = port_str
        .parse()
        .context("Invalid port number")?;

    // Check if port is in use
    if is_port_in_use(port) {
        println!(
            "  {} Port {} appears to be in use",
            "!".yellow(),
            port
        );
        let proceed = Confirm::new("  Continue anyway?")
            .with_default(false)
            .with_render_config(render_config())
            .prompt()?;
        if !proceed {
            return prompt_port(); // Recurse to try again
        }
    }

    Ok(port)
}

fn prompt_dagster_port(mcp_port: u16) -> Result<u16> {
    let default = if mcp_port == DEFAULT_PORT {
        DEFAULT_DAGSTER_PORT
    } else {
        mcp_port + 1000 // Offset from MCP port
    };

    let port_str = Text::new("Dagster UI port (SAGE pipeline dashboard)")
        .with_default(&default.to_string())
        .with_help_message("Optional - for monitoring SAGE jobs")
        .with_render_config(render_config())
        .prompt()?;

    port_str.parse().context("Invalid port number")
}

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

fn prompt_install_dir() -> Result<PathBuf> {
    let default = UserConfig::dir();
    let default_str = default.display().to_string();

    let dir_str = Text::new("Install directory")
        .with_default(&default_str)
        .with_help_message("docker-compose.yml and .env will be created here")
        .with_render_config(render_config())
        .prompt()?;

    let path = PathBuf::from(dir_str);

    if path.exists() && path.join("docker-compose.yml").exists() {
        println!(
            "  {} Existing installation found",
            "!".yellow()
        );
        let overwrite = Confirm::new("  Overwrite?")
            .with_default(false)
            .with_render_config(render_config())
            .prompt()?;
        if !overwrite {
            anyhow::bail!("Cancelled - existing installation preserved");
        }
    }

    Ok(path)
}

fn prompt_postgres_password() -> Result<String> {
    let password = Text::new("PostgreSQL password")
        .with_default("engrammic")
        .with_help_message("For local dev the default is fine; use a strong password in production")
        .with_render_config(render_config())
        .prompt()?;

    if password == "engrammic" || password.len() < 8 {
        println!(
            "  {} Weak password - ok for local dev, change for production",
            "!".yellow()
        );
    }

    Ok(password)
}

fn prompt_llm_provider() -> Result<Option<LlmProvider>> {
    let options = vec![
        "None (passive mode - storage and recall only)",
        "OpenAI (GPT-4o recommended)",
        "Anthropic (Claude)",
        "Google Vertex AI",
    ];

    let choice = Select::new("LLM provider for SAGE", options)
        .with_render_config(render_config())
        .prompt()?;

    match choice {
        "None (passive mode - storage and recall only)" => Ok(None),

        "OpenAI (GPT-4o recommended)" => {
            let api_key = Text::new("  OpenAI API key")
                .with_help_message("sk-...")
                .with_render_config(render_config())
                .prompt()?;

            let model = Text::new("  Model")
                .with_default("gpt-4o-mini")
                .with_render_config(render_config())
                .prompt()?;

            Ok(Some(LlmProvider::OpenAI { api_key, model }))
        }

        "Anthropic (Claude)" => {
            let api_key = Text::new("  Anthropic API key")
                .with_help_message("sk-ant-...")
                .with_render_config(render_config())
                .prompt()?;

            let model = Text::new("  Model")
                .with_default("claude-sonnet-4-6-20250514")
                .with_render_config(render_config())
                .prompt()?;

            Ok(Some(LlmProvider::Anthropic { api_key, model }))
        }

        "Google Vertex AI" => {
            println!(
                "  {}",
                "Requires gcloud auth application-default login".dimmed()
            );
            let project = Text::new("  GCP Project ID")
                .with_render_config(render_config())
                .prompt()?;

            let location = Text::new("  Location")
                .with_default("us-central1")
                .with_render_config(render_config())
                .prompt()?;

            Ok(Some(LlmProvider::VertexAI { project, location }))
        }

        _ => Ok(None),
    }
}

fn write_config_files(config: &SelfHostConfig) -> Result<()> {
    std::fs::create_dir_all(&config.install_dir)?;

    // Generate compose with custom ports
    let compose = generate_compose(config);
    let compose_path = config.install_dir.join("docker-compose.yml");
    std::fs::write(&compose_path, compose)?;
    println!(
        "  {} {}",
        "✓".green(),
        compose_path.display().to_string().dimmed()
    );

    // Generate .env
    let env = generate_env(config);
    let env_path = config.install_dir.join(".env");
    std::fs::write(&env_path, env)?;
    println!(
        "  {} {}",
        "✓".green(),
        env_path.display().to_string().dimmed()
    );

    // README
    let readme = generate_readme(config);
    let readme_path = config.install_dir.join("README.md");
    std::fs::write(&readme_path, readme)?;
    println!(
        "  {} {}",
        "✓".green(),
        readme_path.display().to_string().dimmed()
    );

    Ok(())
}

fn generate_compose(config: &SelfHostConfig) -> String {
    // Replace ports in the template
    let template = docker::COMPOSE_TEMPLATE;
    template
        .replace("- \"8000:8000\"", &format!("- \"{}:8000\"", config.port))
        .replace("- \"3000:3000\"", &format!("- \"{}:3000\"", config.dagster_port))
}

fn generate_env(config: &SelfHostConfig) -> String {
    let mut env = format!(
        r#"# Engrammic Self-Hosted Configuration
# Generated by: engrammic selfhost

# License
ENGRAMMIC_LICENSE_KEY={}

# Database
POSTGRES_PASSWORD={}

# Telemetry
TELEMETRY_ENABLED={}
"#,
        config.license_key,
        config.postgres_password,
        config.telemetry_enabled,
    );

    if let Some(ref provider) = config.llm_provider {
        env.push_str("\n# LLM Provider\n");
        match provider {
            LlmProvider::OpenAI { api_key, model } => {
                env.push_str(&format!("LLM_PROVIDER=openai\n"));
                env.push_str(&format!("LLM_API_KEY={}\n", api_key));
                env.push_str(&format!("LLM_MODEL={}\n", model));
            }
            LlmProvider::Anthropic { api_key, model } => {
                env.push_str(&format!("LLM_PROVIDER=anthropic\n"));
                env.push_str(&format!("LLM_API_KEY={}\n", api_key));
                env.push_str(&format!("LLM_MODEL={}\n", model));
            }
            LlmProvider::VertexAI { project, location } => {
                env.push_str(&format!("LLM_PROVIDER=vertex_ai\n"));
                env.push_str(&format!("VERTEX_PROJECT={}\n", project));
                env.push_str(&format!("VERTEX_LOCATION={}\n", location));
            }
        }
    }

    env
}

fn generate_readme(config: &SelfHostConfig) -> String {
    format!(
        r#"# Engrammic Self-Hosted

Generated by `engrammic selfhost`

## Quick Reference

- MCP endpoint: `http://localhost:{}/mcp`
- Dagster UI: `http://localhost:{}`
- Health check: `curl http://localhost:{}/health`

## Commands

```bash
# Start
docker compose up -d

# Stop
docker compose down

# View logs
docker compose logs -f app

# Upgrade
engrammic upgrade

# Diagnostics
engrammic doctor
```

## Configuration

Edit `.env` to change settings, then restart:

```bash
docker compose down && docker compose up -d
```
"#,
        config.port, config.dagster_port, config.port
    )
}

fn start_and_wait(config: &SelfHostConfig) -> Result<()> {
    println!("  Starting services...");

    let compose_path = config.install_dir.join("docker-compose.yml");

    // Pull images first
    println!("  Pulling images (this may take a few minutes)...");
    let pull = Command::new("docker")
        .args([
            "compose",
            "-f",
            compose_path.to_str().unwrap(),
            "pull",
        ])
        .current_dir(&config.install_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let output = pull.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        println!("  {} Failed to pull images: {}", "✗".red(), stderr);
        anyhow::bail!("docker compose pull failed");
    }
    println!("  {} Images pulled", "✓".green());

    // Start services
    let status = Command::new("docker")
        .args([
            "compose",
            "-f",
            compose_path.to_str().unwrap(),
            "up",
            "-d",
        ])
        .current_dir(&config.install_dir)
        .status()
        .context("Failed to run docker compose up")?;

    if !status.success() {
        anyhow::bail!("docker compose up failed");
    }

    println!("  {} Services started", "✓".green());

    // Wait for healthy
    println!("  Waiting for services to become healthy...");
    wait_for_healthy(config)?;

    Ok(())
}

fn wait_for_healthy(config: &SelfHostConfig) -> Result<()> {
    let health_url = format!("http://localhost:{}/health", config.port);
    let max_attempts = 60;
    let delay = Duration::from_secs(2);

    for attempt in 1..=max_attempts {
        std::thread::sleep(delay);

        let result = Command::new("curl")
            .args(["-sf", &health_url])
            .output();

        match result {
            Ok(output) if output.status.success() => {
                println!("  {} All services healthy", "✓".green());
                return Ok(());
            }
            _ => {
                if attempt % 10 == 0 {
                    println!(
                        "  {} Still waiting ({}/{})",
                        "...".dimmed(),
                        attempt,
                        max_attempts
                    );
                }
            }
        }
    }

    println!(
        "  {} Services didn't become healthy in time",
        "!".yellow()
    );
    println!("  Run {} to diagnose", "engrammic doctor".cyan());
    Ok(())
}

fn configure_editors(config: &SelfHostConfig) -> Result<()> {
    use crate::tools::InstallMethod;

    println!();
    println!("{}", "Configuring editors".bold());
    println!();

    let detected = Tool::detect_installed();
    if detected.is_empty() {
        println!(
            "  {} No supported editors detected",
            "-".dimmed()
        );
        println!(
            "  Add this to your MCP config: {}",
            format!("\"engrammic\": {{ \"type\": \"http\", \"url\": \"http://localhost:{}/mcp\" }}", config.port).cyan()
        );
        return Ok(());
    }

    let endpoint = format!("http://localhost:{}/mcp", config.port);

    for tool in &detected {
        match &tool.method {
            InstallMethod::FileEdit(shape) => {
                match crate::config::install(&tool.config_path, &endpoint, *shape) {
                    Ok(_) => {
                        println!("  {} {}", "✓".green(), tool.name);
                    }
                    Err(e) => {
                        println!("  {} {} - {}", "!".yellow(), tool.name, e);
                    }
                }
            }
            InstallMethod::DeepLink(_) | InstallMethod::PrintInstructions(_) => {
                println!(
                    "  {} {} - configure manually in app settings",
                    "-".dimmed(),
                    tool.name
                );
            }
        }
    }

    Ok(())
}

fn print_manual_start_instructions(config: &SelfHostConfig) {
    println!();
    println!("{}", "To start manually:".bold());
    println!();
    println!(
        "  cd {} && docker compose up -d",
        config.install_dir.display()
    );
    println!();
    println!("  Then configure your editor to connect to:");
    println!(
        "    {}",
        format!("http://localhost:{}/mcp", config.port).cyan()
    );
}

fn print_quick_reference(config: &SelfHostConfig) {
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_black()
    );
    println!();
    println!("  {}", "Setup complete!".bold().green());
    println!();
    println!("  {}", "Quick Reference".bold());
    println!(
        "    MCP endpoint:  {}",
        format!("http://localhost:{}/mcp", config.port).cyan()
    );
    println!(
        "    Health check:  {}",
        format!("curl http://localhost:{}/health", config.port).dimmed()
    );
    println!(
        "    Dagster UI:    {}",
        format!("http://localhost:{}", config.dagster_port).dimmed()
    );
    println!();
    println!("  {}", "Commands".bold());
    println!("    engrammic status   - show current state");
    println!("    engrammic doctor   - run diagnostics");
    println!("    engrammic upgrade  - pull latest version");
    println!("    engrammic logs     - view service logs");
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_black()
    );
    println!();
}

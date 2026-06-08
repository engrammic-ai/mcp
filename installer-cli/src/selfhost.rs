//! Self-hosted setup wizard - guided installation flow.

use anyhow::{Context, Result};
use colored::Colorize;
use inquire::{Confirm, Select, Text};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::cli_install;
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
    pub postgres_password: String,
    pub embedding_model: String,
    pub embedding_dimensions: u32,
    pub embedding_credential: Option<(String, String)>,
}

pub fn run_wizard() -> Result<()> {
    print_welcome();

    // Step 1: Prerequisites
    println!();
    println!("{}", "Step 1/5: Prerequisites".bold());
    println!();
    check_prerequisites()?;

    // Step 2: License
    println!();
    println!("{}", "Step 2/5: License".bold());
    println!();
    let license_key = prompt_license()?;

    // Step 3: Embeddings
    println!();
    println!("{}", "Step 3/5: Embeddings".bold());
    println!();
    let (embedding_model, embedding_dimensions, embedding_credential) = prompt_embeddings()?;

    // Step 4: Configuration
    println!();
    println!("{}", "Step 4/5: Configuration".bold());
    println!();
    let install_dir = prompt_install_dir()?;
    let existing_ports = read_existing_ports(&install_dir);
    let port = prompt_port(existing_ports.0)?;
    let dagster_port = prompt_dagster_port(port, existing_ports.1)?;
    let postgres_password = prompt_postgres_password()?;

    let config = SelfHostConfig {
        license_key,
        port,
        dagster_port,
        install_dir,
        postgres_password,
        embedding_model,
        embedding_dimensions,
        embedding_credential,
    };

    // Step 5: Install
    println!();
    println!("{}", "Step 5/5: Install".bold());
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

    cli_install::offer_cli_install(false)?;

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
    println!("    3. Configure your embedding model");
    println!("    4. Configure ports and storage");
    println!("    5. Start the services and configure your code editor");
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
            .with_help_message("Starts with ENGR_ - request at dev@engrammic.ai")
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

fn prompt_embeddings() -> Result<(String, u32, Option<(String, String)>)> {
    let providers = vec![
        "OpenAI (cloud, paid)",
        "Ollama (local, free)",
        "Vertex AI (GCP)",
        "Other (manual config)",
    ];

    let provider = Select::new("Embedding provider", providers)
        .with_help_message("Choose where embeddings are computed")
        .with_render_config(render_config())
        .prompt()?;

    let (model, dimensions, credential) = match provider {
        "OpenAI (cloud, paid)" => {
            let models = vec![
                "text-embedding-3-small (1536 dims, recommended)",
                "text-embedding-3-large (3072 dims, higher quality)",
            ];
            let model_choice = Select::new("Model", models)
                .with_render_config(render_config())
                .prompt()?;

            let (model, dims) = if model_choice.starts_with("text-embedding-3-small") {
                ("openai/text-embedding-3-small", 1536)
            } else {
                ("openai/text-embedding-3-large", 3072)
            };

            println!("  {} Dimensions: {} (auto-filled)", "✓".green(), dims);

            let key = Text::new("OPENAI_API_KEY")
                .with_help_message("Your OpenAI API key (starts with sk-)")
                .with_render_config(render_config())
                .prompt()?;

            (model.to_string(), dims, Some(("OPENAI_API_KEY".to_string(), key)))
        }
        "Ollama (local, free)" => {
            let models = vec![
                "nomic-embed-text (768 dims, recommended)",
                "all-minilm (384 dims, smaller)",
                "Other (enter manually)",
            ];
            let model_choice = Select::new("Model", models)
                .with_help_message("Make sure this model is pulled in Ollama")
                .with_render_config(render_config())
                .prompt()?;

            let (model, dims) = if model_choice.starts_with("nomic-embed-text") {
                ("ollama/nomic-embed-text".to_string(), 768u32)
            } else if model_choice.starts_with("all-minilm") {
                ("ollama/all-minilm".to_string(), 384)
            } else {
                let name = Text::new("Model name")
                    .with_help_message("Just the model name, e.g. mxbai-embed-large")
                    .with_render_config(render_config())
                    .prompt()?;
                let dims = prompt_dimensions()?;
                (format!("ollama/{}", name), dims)
            };

            println!("  {} Dimensions: {}", "✓".green(), dims);

            let base = Text::new("OLLAMA_API_BASE")
                .with_default("http://localhost:11434")
                .with_help_message("URL where your Ollama server is running")
                .with_render_config(render_config())
                .prompt()?;

            (model, dims, Some(("OLLAMA_API_BASE".to_string(), base)))
        }
        "Vertex AI (GCP)" => {
            let model = "vertex_ai/text-embedding-005";
            let dims = 768;

            println!("  {} Model: {}", "✓".green(), model);
            println!("  {} Dimensions: {}", "✓".green(), dims);

            let project = Text::new("VERTEX_PROJECT")
                .with_help_message("Your Google Cloud project ID")
                .with_render_config(render_config())
                .prompt()?;
            let location = Text::new("VERTEX_LOCATION")
                .with_default("us-central1")
                .with_help_message("GCP region for Vertex AI")
                .with_render_config(render_config())
                .prompt()?;

            (
                model.to_string(),
                dims,
                Some(("VERTEX".to_string(), format!("{}\x00{}", project, location))),
            )
        }
        _ => {
            let model = Text::new("Embedding model")
                .with_help_message("Format: provider/model-name")
                .with_render_config(render_config())
                .prompt()?;

            let dims = prompt_dimensions()?;

            println!(
                "  {} Configure credentials manually in .env after setup",
                "!".yellow()
            );

            (model, dims, None)
        }
    };

    Ok((model, dimensions, credential))
}

fn prompt_dimensions() -> Result<u32> {
    println!();
    println!(
        "  {} Wrong dimensions will corrupt your Qdrant collection.",
        "WARNING:".yellow().bold()
    );
    println!("           Fixing requires wiping and re-embedding all data.");
    println!();
    let dims_str = Text::new("Embedding dimensions")
        .with_help_message("Check your model's documentation for the correct value")
        .with_render_config(render_config())
        .prompt()?;
    dims_str.parse::<u32>().context("Invalid dimensions - must be a positive integer")
}

fn read_existing_ports(install_dir: &Path) -> (Option<u16>, Option<u16>) {
    let compose_path = install_dir.join("docker-compose.yml");
    let Ok(content) = std::fs::read_to_string(&compose_path) else {
        return (None, None);
    };

    let mut app_port = None;
    let mut dagster_port = None;

    for line in content.lines() {
        let trimmed = line.trim();
        // Look for port mappings like '- "9000:8000"' or '- "3001:3000"'
        if trimmed.starts_with("- \"") && trimmed.contains(":8000\"") {
            if let Some(port_str) = trimmed.strip_prefix("- \"").and_then(|s| s.split(':').next()) {
                app_port = port_str.parse().ok();
            }
        } else if trimmed.starts_with("- \"") && trimmed.contains(":3000\"") {
            if let Some(port_str) = trimmed.strip_prefix("- \"").and_then(|s| s.split(':').next()) {
                dagster_port = port_str.parse().ok();
            }
        }
    }

    (app_port, dagster_port)
}

fn prompt_port(existing: Option<u16>) -> Result<u16> {
    let default = existing.unwrap_or(DEFAULT_PORT);
    let port_str = Text::new("MCP server port")
        .with_default(&default.to_string())
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
            return prompt_port(existing);
        }
    }

    Ok(port)
}

fn prompt_dagster_port(mcp_port: u16, existing: Option<u16>) -> Result<u16> {
    let default = existing.unwrap_or_else(|| {
        if mcp_port == DEFAULT_PORT {
            DEFAULT_DAGSTER_PORT
        } else {
            mcp_port + 1000
        }
    });

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

const MODELS_YAML_TEMPLATE: &str = include_str!("../assets/models.yaml");

fn write_config_files(config: &SelfHostConfig) -> Result<()> {
    std::fs::create_dir_all(&config.install_dir)?;

    // Check if config files already exist and prompt before overwriting
    let env_path = config.install_dir.join(".env");
    let models_path = config.install_dir.join("config/models.yaml");
    if env_path.exists() || models_path.exists() {
        let overwrite = Confirm::new("Config files exist. Overwrite?")
            .with_default(false)
            .with_render_config(render_config())
            .prompt()?;
        if !overwrite {
            println!(
                "  Skipping config generation. Edit manually at: {}",
                config.install_dir.display()
            );
            return Ok(());
        }
    }

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

    // models.yaml template
    let config_dir = config.install_dir.join("config");
    std::fs::create_dir_all(&config_dir)?;
    let models_path = config_dir.join("models.yaml");
    std::fs::write(&models_path, MODELS_YAML_TEMPLATE)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&models_path, std::fs::Permissions::from_mode(0o644))?;
    }
    println!(
        "  {} {}",
        "✓".green(),
        models_path.display().to_string().dimmed()
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
    // Build the credential lines for the embedding section
    let credential_lines = match &config.embedding_credential {
        Some((var, val)) if var == "VERTEX" => {
            // Decode the packed VERTEX_PROJECT\x00VERTEX_LOCATION value
            let parts: Vec<&str> = val.splitn(2, '\x00').collect();
            let project = parts.first().copied().unwrap_or("");
            let location = parts.get(1).copied().unwrap_or("us-central1");
            format!("VERTEX_PROJECT={project}\nVERTEX_LOCATION={location}")
        }
        Some((var, val)) => format!("{var}={val}"),
        None => String::new(),
    };

    let credential_section = if credential_lines.is_empty() {
        String::new()
    } else {
        format!("{credential_lines}\n")
    };

    format!(
        r#"# Engrammic Self-Hosted Configuration
# Generated by: engrammic selfhost
#
# Lines prefixed with '#' are optional/commented out.
# Uncomment and fill in values to enable those features.

# License
ENGRAMMIC_LICENSE_KEY={license_key}

# Database
POSTGRES_PASSWORD={postgres_password}

# EMBEDDINGS (required)
EMBEDDING_MODEL={embedding_model}
EMBEDDING_DIMENSIONS={embedding_dimensions}
{credential_section}
# INFRASTRUCTURE (defaults work with bundled compose)
# Override only if you're pointing at external services.
# QDRANT_HOST=qdrant
# QDRANT_PORT=6333
# QDRANT_API_KEY=
# REDIS_URL=redis://redis:6379
# MEMGRAPH_HOST=memgraph
# MEMGRAPH_PORT=7687
ENGRAMMIC_CONFIG_DIR=/app/config-override

# RERANKING (optional, improves recall quality)
# Uncomment to enable. Cohere is the recommended provider.
# RERANKING__ENABLED=true
# RERANKING__PROVIDER=cohere
# RERANKING__MODEL=rerank-english-v3.0
# COHERE_API_KEY=your-key

# LLM CREDENTIALS (configure models in config/models.yaml)
# Add keys for the providers you want to use for LLM tasks.
# OPENAI_API_KEY=your-key
# ANTHROPIC_API_KEY=your-key
# GEMINI_API_KEY=your-key
# OLLAMA_BASE_URL=http://localhost:11434

# TELEMETRY
TELEMETRY__ENABLED=false
"#,
        license_key = config.license_key,
        postgres_password = config.postgres_password,
        embedding_model = config.embedding_model,
        embedding_dimensions = config.embedding_dimensions,
        credential_section = credential_section,
    )
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

- `.env` - environment variables (embeddings, credentials, telemetry)
- `config/models.yaml` - LLM model configuration for SAGE

Full reference: https://docs.engrammic.ai/docs/reference/configuration

**Note:** Config changes require a service restart:

```bash
docker compose restart
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

#[cfg(test)]
mod tests {
    #[test]
    fn models_yaml_template_schema_valid() {
        let template = include_str!("../assets/models.yaml");
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(template).expect("assets/models.yaml must be valid YAML");

        let mapping = yaml.as_mapping().expect("assets/models.yaml must be a YAML mapping");

        assert!(
            mapping.contains_key("default_tier"),
            "assets/models.yaml missing required key: default_tier"
        );
        assert!(
            mapping.contains_key("tiers"),
            "assets/models.yaml missing required key: tiers"
        );

        let tiers = mapping
            .get("tiers")
            .and_then(|v| v.as_mapping())
            .expect("tiers must be a mapping");

        for (tier_name, tier_config) in tiers {
            // Tiers with all keys commented out parse as null; skip them.
            if tier_config.is_null() {
                continue;
            }

            let config = tier_config
                .as_mapping()
                .unwrap_or_else(|| panic!("tier {:?} must be a mapping", tier_name));

            // Only validate keys that are present (template may have commented-out sections).
            for key in ["reasoning", "fast", "query_expander"] {
                if let Some(role) = config.get(key) {
                    assert!(
                        role.as_mapping().is_some(),
                        "tier {:?} key '{}' must be a mapping",
                        tier_name,
                        key
                    );
                }
            }
        }
    }
}

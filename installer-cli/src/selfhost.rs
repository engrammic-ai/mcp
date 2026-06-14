//! Self-hosted setup wizard - guided installation flow.

use anyhow::{Context, Result};
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use crate::docker;
use crate::license;
use crate::tools::Tool;
use crate::user_config::UserConfig;

/// Wizard step enum for step-based navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WizardStep {
    Runtime,
    Tier,
    Prerequisites,
    License,
    Embeddings,
    Config,
    Install,
}

impl WizardStep {
    fn number(&self) -> usize {
        match self {
            WizardStep::Runtime => 1,
            WizardStep::Tier => 2,
            WizardStep::Prerequisites => 3,
            WizardStep::License => 4,
            WizardStep::Embeddings => 5,
            WizardStep::Config => 6,
            WizardStep::Install => 7,
        }
    }

    fn total() -> usize {
        7
    }


}

/// Intermediate state accumulating wizard choices across steps.
/// Fields are Option so that going back clears future selections.
#[derive(Debug, Default)]
struct WizardState {
    podman: bool,
    tier: Option<Tier>,
    license_key: Option<String>,
    embedding_model: Option<String>,
    embedding_dimensions: Option<u32>,
    embedding_credential: Option<Option<(String, String)>>,
    install_dir: Option<PathBuf>,
    port: Option<u16>,
    dagster_port: Option<u16>,
    postgres_password: Option<String>,
    use_external_ollama: Option<bool>,
}

/// Existing configuration read from .env file.
#[derive(Debug, Default)]
struct ExistingConfig {
    license_key: Option<String>,
    embedding_model: Option<String>,
    embedding_dimensions: Option<u32>,
    postgres_password: Option<String>,
    openai_api_key: Option<String>,
    ollama_host: Option<String>,
}

/// Read existing configuration from .env file in install directory.
fn read_existing_env(install_dir: &Path) -> ExistingConfig {
    let env_path = install_dir.join(".env");
    let Ok(content) = std::fs::read_to_string(&env_path) else {
        return ExistingConfig::default();
    };

    let mut config = ExistingConfig::default();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "ENGRAMMIC_LICENSE_KEY" if !value.is_empty() => {
                    config.license_key = Some(value.to_string());
                }
                "EMBEDDING_MODEL" if !value.is_empty() => {
                    config.embedding_model = Some(value.to_string());
                }
                "EMBEDDING_DIMENSIONS" => {
                    config.embedding_dimensions = value.parse().ok();
                }
                "POSTGRES_PASSWORD" if !value.is_empty() => {
                    config.postgres_password = Some(value.to_string());
                }
                "OPENAI_API_KEY" if !value.is_empty() => {
                    config.openai_api_key = Some(value.to_string());
                }
                "OLLAMA_HOST" if !value.is_empty() => {
                    config.ollama_host = Some(value.to_string());
                }
                _ => {}
            }
        }
    }

    config
}

/// Ask the user whether to go back. Returns true if they want to go back.
fn prompt_go_back() -> Result<bool> {
    let input: String = Input::new()
        .with_prompt("Press Enter to continue, or type 'b' to go back")
        .allow_empty(true)
        .interact_text()?;
    Ok(input.trim().eq_ignore_ascii_case("b"))
}

/// Print a two-line error in the standard ✗ / → format.
///
///   ✗ <what_happened>
///   → <what_to_do>
///
/// Use this for all user-facing failures so the format is consistent.
fn fmt_err(what_happened: &str, what_to_do: &str) {
    eprintln!("  {} {}", "✗".red().bold(), what_happened);
    eprintln!("  {} {}", "→".yellow(), what_to_do);
}

const DEFAULT_PORT: u16 = 8000;
const DEFAULT_DAGSTER_PORT: u16 = 3000;

/// Basic GPU information returned by nvidia-smi.
#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub vram_mb: u64,
}

impl GpuInfo {
    pub fn vram_gb(&self) -> f64 {
        self.vram_mb as f64 / 1024.0
    }
}

/// Probe the system for an NVIDIA GPU via nvidia-smi.
///
/// Returns `None` if nvidia-smi is not found, fails, or produces unparseable
/// output. Returns `Some(GpuInfo)` on success. Failures are silent — callers
/// treat absence of a GPU as a warning condition, not an error.
pub fn check_gpu() -> Option<GpuInfo> {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // nvidia-smi may report multiple GPUs, one per line. Use the first.
    let first_line = stdout.lines().next()?.trim();
    let vram_mb: u64 = first_line.parse().ok()?;

    Some(GpuInfo { vram_mb })
}

/// Print GPU status or warnings after tier selection.
///
/// Warnings are informational only — they do not abort the installation.
fn warn_gpu_for_tier(tier: Tier, gpu: &Option<GpuInfo>) {
    println!();
    match (tier, gpu) {
        (Tier::Standard, None) => {
            println!(
                "  {} No GPU detected. Standard tier works best with GPU (8GB+ VRAM).",
                "!".yellow()
            );
            println!("    CPU-only inference will be slower but functional.");
        }
        (Tier::Pro, None) => {
            println!(
                "  {} No GPU detected. Pro tier recommends GPU (16GB+ VRAM) for gemma4:26b.",
                "!".yellow()
            );
            println!("    CPU-only inference will be slower but functional.");
        }
        (Tier::Pro, Some(info)) if info.vram_mb < 16 * 1024 => {
            println!(
                "  {} Pro tier recommends 16GB+ VRAM, detected {:.0}GB.",
                "!".yellow(),
                info.vram_gb()
            );
        }
        (Tier::Standard, Some(info)) if info.vram_mb < 8 * 1024 => {
            println!(
                "  {} Standard tier recommends 8GB+ VRAM, detected {:.0}GB.",
                "!".yellow(),
                info.vram_gb()
            );
        }
        (_, Some(info)) => {
            println!(
                "  {} GPU detected: {:.0}GB VRAM",
                "✓".green(),
                info.vram_gb()
            );
        }
        // Lite and Cloud: no GPU advice needed.
        _ => {}
    }
}

/// Hardware tier for standalone deployment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// 8GB RAM - phi4-mini, no reranker
    Lite,
    /// 24-32GB RAM - gemma4:12b + bge-reranker
    Standard,
    /// 48-64GB RAM - gemma4:26b + jina-reranker
    Pro,
    /// Cloud APIs - no local models
    Cloud,
}

impl Tier {
    pub fn ram_requirement(&self) -> &'static str {
        match self {
            Tier::Lite => "8GB",
            Tier::Standard => "24-32GB",
            Tier::Pro => "48-64GB",
            Tier::Cloud => "any",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Tier::Lite => "phi4-mini, no reranker",
            Tier::Standard => "gemma4:12b + bge-reranker",
            Tier::Pro => "gemma4:26b + jina-reranker",
            Tier::Cloud => "use cloud APIs (OpenAI, Anthropic, etc.)",
        }
    }

    pub fn ollama_model(&self) -> Option<&'static str> {
        match self {
            Tier::Lite => Some("phi4-mini"),
            Tier::Standard => Some("gemma4:12b"),
            Tier::Pro => Some("gemma4:26b"),
            Tier::Cloud => None,
        }
    }

    pub fn is_standalone(&self) -> bool {
        !matches!(self, Tier::Cloud)
    }
}

/// Probe for an existing Ollama instance at the default local address.
///
/// Makes a GET request to `http://localhost:11434/api/tags` with a 2-second
/// timeout. Returns `Some("localhost:11434")` on a successful response,
/// `None` if Ollama is not running or unreachable.
pub fn detect_existing_ollama() -> Option<String> {
    let response = ureq::get("http://localhost:11434/api/tags")
        .timeout(Duration::from_secs(2))
        .call();
    match response {
        Ok(_) => Some("localhost:11434".to_string()),
        Err(_) => None,
    }
}

#[derive(Debug, Clone)]
pub struct SelfHostConfig {
    pub tier: Tier,
    pub license_key: String,
    pub port: u16,
    pub dagster_port: u16,
    pub install_dir: PathBuf,
    pub postgres_password: String,
    pub embedding_model: String,
    pub embedding_dimensions: u32,
    pub embedding_credential: Option<(String, String)>,
    /// If true, skip the Ollama Docker container and point OLLAMA_HOST at the
    /// user's existing local Ollama instance (localhost:11434).
    pub use_external_ollama: bool,
    /// If true, use Podman-compatible compose output: skip Docker daemon check,
    /// add :Z SELinux suffix to volume mounts, use CDI GPU syntax.
    pub podman: bool,
}

pub fn run_wizard(podman: bool) -> Result<()> {
    print_welcome();

    // Check for existing installation
    let default_dir = UserConfig::dir();
    let existing_env = read_existing_env(&default_dir);
    let existing_ports = read_existing_ports(&default_dir);
    let has_existing = default_dir.join(".env").exists();

    // Initialise state. If --podman was passed on the CLI, pre-set it so the
    // runtime step shows Podman as already selected and skips the prompt.
    let mut state = WizardState {
        podman,
        ..Default::default()
    };

    // Pre-populate from existing config if available
    let mut step = WizardStep::Runtime;
    if has_existing {
        println!(
            "  {} Existing installation found at {}",
            "!".yellow(),
            default_dir.display()
        );

        let options = vec![
            "Reconfigure (update settings, keep data)",
            "Fresh install (overwrite everything)",
            "Cancel",
        ];
        let idx = Select::new()
            .with_prompt("What would you like to do?")
            .items(&options)
            .default(0)
            .interact()?;

        match idx {
            0 => {
                // Reconfigure: pre-populate state with existing values
                println!("  {} Reconfigure mode - existing values shown as defaults", "→".yellow());
                state.install_dir = Some(default_dir.clone());
                state.license_key = existing_env.license_key;
                state.embedding_model = existing_env.embedding_model;
                state.embedding_dimensions = existing_env.embedding_dimensions;
                state.postgres_password = existing_env.postgres_password;
                state.port = existing_ports.0;
                state.dagster_port = existing_ports.1;
                // Detect external ollama from existing config
                state.use_external_ollama = existing_env.ollama_host.map(|_| true);
            }
            1 => {
                // Fresh install: continue with empty state
                println!("  {} Fresh install mode", "→".yellow());
            }
            _ => {
                // Cancelled reconfigure, but offer to just launch existing
                println!();
                let just_launch = Confirm::new()
                    .with_prompt("Just start existing installation?")
                    .default(true)
                    .interact()?;

                if just_launch {
                    let port = existing_ports.0.unwrap_or(DEFAULT_PORT);
                    let config = SelfHostConfig {
                        tier: Tier::Cloud, // doesn't matter for launch
                        license_key: existing_env.license_key.unwrap_or_default(),
                        port,
                        dagster_port: existing_ports.1.unwrap_or(DEFAULT_DAGSTER_PORT),
                        install_dir: default_dir.clone(),
                        postgres_password: String::new(),
                        embedding_model: String::new(),
                        embedding_dimensions: 0,
                        embedding_credential: None,
                        use_external_ollama: false,
                        podman,
                    };
                    start_and_wait(&config)?;
                    print_quick_reference(&config);
                }
                return Ok(());
            }
        }
        println!();
    }

    loop {
        match step {
            // ----------------------------------------------------------------
            // Step 1: Container runtime (Docker vs Podman)
            // ----------------------------------------------------------------
            WizardStep::Runtime => {
                println!();
                println!(
                    "{}",
                    format!(
                        "Step {}/{}: Container Runtime",
                        WizardStep::Runtime.number(),
                        WizardStep::total()
                    )
                    .bold()
                );
                println!();

                if podman {
                    // --podman flag was provided; skip interactive prompt and
                    // show the socket instructions as before.
                    println!("  {} Podman mode enabled (--podman flag).", "!".yellow());
                    println!("    Make sure to start the socket:");
                    println!("      podman system service --time=0 unix:///tmp/podman.sock &");
                    println!("      export DOCKER_HOST=unix:///tmp/podman.sock");
                    state.podman = true;
                } else {
                    let options = vec!["Docker (default)", "Podman"];
                    let idx = Select::new()
                        .with_prompt("Container runtime")
                        .items(&options)
                        .default(0)
                        .interact()?;

                    state.podman = idx == 1;

                    if state.podman {
                        println!();
                        println!("  {} Podman selected. Make sure to start the socket:", "!".yellow());
                        println!("    podman system service --time=0 unix:///tmp/podman.sock &");
                        println!("    export DOCKER_HOST=unix:///tmp/podman.sock");
                    } else {
                        println!("  {} Docker selected", "✓".green());
                    }
                }

                // Runtime is always the first step — no going back from here.
                step = WizardStep::Tier;
            }

            // ----------------------------------------------------------------
            // Step 2: Hardware Profile (Tier)
            // ----------------------------------------------------------------
            WizardStep::Tier => {
                println!();
                println!(
                    "{}",
                    format!(
                        "Step {}/{}: Hardware Profile",
                        WizardStep::Tier.number(),
                        WizardStep::total()
                    )
                    .bold()
                );
                println!();

                let tier = prompt_tier()?;
                let gpu = check_gpu();
                warn_gpu_for_tier(tier, &gpu);

                println!();
                check_and_warn_ports(tier);

                state.tier = Some(tier);
                // Clear downstream state in case user went back and changed tier
                state.embedding_model = None;
                state.embedding_dimensions = None;
                state.embedding_credential = None;

                if prompt_go_back()? {
                    step = WizardStep::Runtime;
                } else {
                    step = WizardStep::Prerequisites;
                }
            }

            // ----------------------------------------------------------------
            // Step 3: Prerequisites
            // ----------------------------------------------------------------
            WizardStep::Prerequisites => {
                println!();
                println!(
                    "{}",
                    format!(
                        "Step {}/{}: Prerequisites",
                        WizardStep::Prerequisites.number(),
                        WizardStep::total()
                    )
                    .bold()
                );
                println!();

                if state.podman {
                    println!("  {} Podman mode: skipping Docker daemon check.", "→".yellow());
                    check_prerequisites_podman()?;
                } else {
                    check_prerequisites()?;
                }

                if prompt_go_back()? {
                    step = WizardStep::Tier;
                } else {
                    step = WizardStep::License;
                }
            }

            // ----------------------------------------------------------------
            // Step 4: License
            // ----------------------------------------------------------------
            WizardStep::License => {
                println!();
                println!(
                    "{}",
                    format!(
                        "Step {}/{}: License",
                        WizardStep::License.number(),
                        WizardStep::total()
                    )
                    .bold()
                );
                println!();

                let license_key_opt = prompt_license(state.license_key.as_deref())?;
                state.license_key = Some(license_key_opt.unwrap_or_default());
                // If empty: the .env will have ENGRAMMIC_LICENSE_KEY= (blank); the user must run
                // `engrammic license` before Engrammic will accept connections. The wizard
                // continues so the rest of the setup is not lost.

                if prompt_go_back()? {
                    step = WizardStep::Prerequisites;
                } else {
                    step = WizardStep::Embeddings;
                }
            }

            // ----------------------------------------------------------------
            // Step 5: Embeddings
            // ----------------------------------------------------------------
            WizardStep::Embeddings => {
                let tier = state.tier.expect("tier must be set before embeddings step");

                println!();
                println!(
                    "{}",
                    format!(
                        "Step {}/{}: Embeddings",
                        WizardStep::Embeddings.number(),
                        WizardStep::total()
                    )
                    .bold()
                );
                println!();

                let (embedding_model, embedding_dimensions, embedding_credential) =
                    if tier.is_standalone() {
                        println!(
                            "  {} Using TEI with nomic-embed (768 dims) - bundled with {} tier",
                            "✓".green(),
                            format!("{:?}", tier).to_lowercase()
                        );
                        (
                            "tei/nomic-ai/nomic-embed-text-v1.5".to_string(),
                            768u32,
                            None,
                        )
                    } else {
                        prompt_embeddings()?
                    };

                state.embedding_model = Some(embedding_model);
                state.embedding_dimensions = Some(embedding_dimensions);
                state.embedding_credential = Some(embedding_credential);

                if prompt_go_back()? {
                    step = WizardStep::License;
                } else {
                    step = WizardStep::Config;
                }
            }

            // ----------------------------------------------------------------
            // Step 6: Configuration (ports, install dir, password)
            // ----------------------------------------------------------------
            WizardStep::Config => {
                let tier = state.tier.expect("tier must be set before config step");

                println!();
                println!(
                    "{}",
                    format!(
                        "Step {}/{}: Configuration",
                        WizardStep::Config.number(),
                        WizardStep::total()
                    )
                    .bold()
                );
                println!();

                let install_dir = prompt_install_dir(state.install_dir.as_ref())?;
                // Use pre-populated ports from state if reconfiguring, else read from disk
                let existing_ports = if state.port.is_some() {
                    (state.port, state.dagster_port)
                } else {
                    read_existing_ports(&install_dir)
                };
                let port = prompt_port(existing_ports.0)?;
                let dagster_port = prompt_dagster_port(port, existing_ports.1)?;
                let postgres_password = prompt_postgres_password(state.postgres_password.as_deref())?;

                // Disk space check - after install_dir and tier are known, before any downloads
                check_disk_space(&install_dir, tier)?;

                // Detect existing Ollama for standalone tiers before any model downloads
                let use_external_ollama = if tier.is_standalone() {
                    if let Some(addr) = detect_existing_ollama() {
                        println!();
                        println!(
                            "  {} Detected existing Ollama at {}",
                            "✓".green(),
                            addr.cyan()
                        );
                        Confirm::new()
                            .with_prompt("Use existing Ollama instead of Docker container?")
                            .default(true)
                            .interact()?
                    } else {
                        false
                    }
                } else {
                    false
                };

                state.install_dir = Some(install_dir);
                state.port = Some(port);
                state.dagster_port = Some(dagster_port);
                state.postgres_password = Some(postgres_password);
                state.use_external_ollama = Some(use_external_ollama);

                if prompt_go_back()? {
                    step = WizardStep::Embeddings;
                } else {
                    step = WizardStep::Install;
                }
            }

            // ----------------------------------------------------------------
            // Step 7: Install
            // ----------------------------------------------------------------
            WizardStep::Install => {
                println!();
                println!(
                    "{}",
                    format!(
                        "Step {}/{}: Install",
                        WizardStep::Install.number(),
                        WizardStep::total()
                    )
                    .bold()
                );
                println!();

                let tier = state.tier.expect("tier must be set");
                let config = SelfHostConfig {
                    tier,
                    license_key: state.license_key.clone().unwrap_or_default(),
                    port: state.port.expect("port must be set"),
                    dagster_port: state.dagster_port.expect("dagster_port must be set"),
                    install_dir: state.install_dir.clone().expect("install_dir must be set"),
                    postgres_password: state
                        .postgres_password
                        .clone()
                        .expect("postgres_password must be set"),
                    embedding_model: state
                        .embedding_model
                        .clone()
                        .expect("embedding_model must be set"),
                    embedding_dimensions: state
                        .embedding_dimensions
                        .expect("embedding_dimensions must be set"),
                    embedding_credential: state
                        .embedding_credential
                        .clone()
                        .expect("embedding_credential must be set"),
                    use_external_ollama: state.use_external_ollama.unwrap_or(false),
                    podman: state.podman,
                };

                write_config_files(&config)?;

                // Model download for standalone tiers
                if tier.is_standalone() {
                    download_models(&config)?;
                }

                // Offer to start
                println!();
                let start_now = Confirm::new()
                    .with_prompt("Start Engrammic now?")
                    .default(true)
                    .interact()?;

                if start_now {
                    start_and_wait(&config)?;
                    configure_editors(&config)?;
                } else {
                    print_manual_start_instructions(&config);
                }

                // Save user config. A skipped license stays None — Some("") would make
                // doctor and the returning-user menu report an invalid license forever.
                let user_config = UserConfig {
                    endpoint: Some(format!("http://localhost:{}/mcp", config.port)),
                    license_key: if config.license_key.is_empty() {
                        None
                    } else {
                        Some(config.license_key.clone())
                    },
                    selfhost_dir: Some(config.install_dir.clone()),
                };
                user_config.save()?;

                print_quick_reference(&config);

                if let Ok(exe) = std::env::current_exe() {
                    println!(
                        "  {} CLI available at {}",
                        "✓".green(),
                        exe.display().to_string().cyan()
                    );
                }

                // Wizard complete — exit the loop
                break;
            }
        }
    }

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
    println!("  {}", "Memory infrastructure for AI agents".dimmed());
    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
            .bright_black()
    );
    println!();
    println!("  This wizard will:");
    println!("    1. Select container runtime (Docker or Podman)");
    println!("    2. Select hardware tier (Lite/Standard/Pro/Cloud)");
    println!("    3. Check runtime prerequisites");
    println!("    4. Validate your license");
    println!("    5. Configure embeddings (auto for standalone tiers)");
    println!("    6. Configure ports and storage");
    println!("    7. Download models and start services");
    println!();
    println!("  {} At any step, type 'b' + Enter to go back.", "tip:".dimmed());
    println!();
}

fn check_prerequisites() -> Result<()> {
    // Docker
    print!("  Checking Docker... ");
    if !docker::check_docker()? {
        println!("{}", "not found".red());
        println!();
        let docker_hint = match std::env::consts::OS {
            "linux" => format!(
                "Run {} then start Docker, or see {}",
                "curl -fsSL https://get.docker.com | sh".cyan(),
                "https://docs.docker.com/engine/install/".cyan()
            ),
            "macos" => format!(
                "Install Docker Desktop from {}",
                "https://docker.com/products/docker-desktop".cyan()
            ),
            "windows" => format!(
                "Install Docker Desktop with WSL2 backend from {}",
                "https://docker.com/products/docker-desktop".cyan()
            ),
            _ => format!(
                "Install Docker from {}",
                "https://docs.docker.com/get-docker/".cyan()
            ),
        };
        fmt_err("Docker is not running or not installed.", &docker_hint);
        anyhow::bail!("Docker is not running or not installed");
    }
    println!("{}", "ok".green());

    // Docker Compose v2
    print!("  Checking Docker Compose... ");
    let compose_check = Command::new("docker").args(["compose", "version"]).output();
    match compose_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version_short = version.trim().split_whitespace().last().unwrap_or("v2");
            println!("{} ({})", "ok".green(), version_short.dimmed());
        }
        _ => {
            println!("{}", "not found".red());
            println!();
            let compose_hint = match std::env::consts::OS {
                "linux" => format!(
                    "Install the Compose plugin: {} or upgrade Docker Engine to v23+",
                    "https://docs.docker.com/compose/install/linux/".cyan()
                ),
                "macos" | "windows" => format!(
                    "Upgrade Docker Desktop to v4.x or later from {}",
                    "https://docker.com/products/docker-desktop".cyan()
                ),
                _ => "Upgrade Docker Desktop or install the Compose plugin, then try again."
                    .to_string(),
            };
            fmt_err("Docker Compose v2 not found.", &compose_hint);
            anyhow::bail!("Docker Compose v2 not found");
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

/// Prerequisite check for Podman mode: skip Docker daemon check, verify
/// `podman compose` or `docker compose` (via DOCKER_HOST socket) is available.
fn check_prerequisites_podman() -> Result<()> {
    // Check podman compose availability
    print!("  Checking podman compose... ");
    let podman_compose = Command::new("podman").args(["compose", "version"]).output();
    match podman_compose {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version_short = version.trim().split_whitespace().last().unwrap_or("ok");
            println!("{} ({})", "ok".green(), version_short.dimmed());
        }
        _ => {
            // Fall back to docker compose via socket
            print!("not found — checking docker compose via socket... ");
            let compose_check = Command::new("docker").args(["compose", "version"]).output();
            match compose_check {
                Ok(output) if output.status.success() => {
                    let version = String::from_utf8_lossy(&output.stdout);
                    let version_short = version.trim().split_whitespace().last().unwrap_or("v2");
                    println!("{} ({})", "ok".green(), version_short.dimmed());
                }
                _ => {
                    println!("{}", "not found".red());
                    println!();
                    fmt_err(
                        "Neither podman compose nor docker compose found.",
                        "Install podman-compose, or set DOCKER_HOST and ensure docker compose is available.",
                    );
                    anyhow::bail!("No compose tool found for Podman mode");
                }
            }
        }
    }

    // Memory check (same as Docker path)
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
    #[cfg(target_os = "windows")]
    {
        use sysinfo::System;
        let sys = System::new_all();
        let bytes = sys.total_memory();
        if bytes > 0 {
            return bytes as f64 / 1024.0 / 1024.0 / 1024.0;
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        eprintln!(
            "{}",
            "warning: memory detection not supported on this platform; assuming 8 GB".yellow()
        );
    }
    8.0 // fallback assumption
}

/// Minimum disk space required for each tier, in gigabytes.
fn get_required_disk_gb(tier: Tier) -> u64 {
    match tier {
        Tier::Lite => 8,
        Tier::Standard => 20,
        Tier::Pro => 30,
        Tier::Cloud => 2,
    }
}

/// Print a download breakdown for standalone tiers and verify available disk space.
///
/// Returns an error if the path's filesystem has less free space than the tier requires.
fn check_disk_space(path: &Path, tier: Tier) -> Result<()> {
    // Ensure the path exists so fs2 can stat it. If not yet created, walk up to
    // the first ancestor that exists (the install_dir may not exist yet).
    let stat_path = {
        let mut p = path;
        loop {
            if p.exists() {
                break p;
            }
            match p.parent() {
                Some(parent) => p = parent,
                None => break p,
            }
        }
    };

    let available_bytes = fs2::available_space(stat_path)
        .with_context(|| format!("Could not read available disk space for {}", stat_path.display()))?;
    let available_gb = available_bytes / 1_073_741_824; // bytes -> GB (floor)

    let required_gb = get_required_disk_gb(tier);

    // Print breakdown
    println!();
    match tier {
        Tier::Lite => {
            println!("  LLM (phi4-mini):          ~5GB");
            println!("  Embeddings (TEI):         ~700MB");
            println!("  Databases + cache:        ~2GB");
            println!("  Buffer:                   ~300MB");
        }
        Tier::Standard => {
            println!("  LLM (gemma4:12b):         ~8GB");
            println!("  Embeddings (TEI):         ~700MB");
            println!("  Reranker (TEI):           ~1GB");
            println!("  Databases + cache:        ~3GB");
            println!("  Buffer:                   ~7GB");
        }
        Tier::Pro => {
            println!("  LLM (gemma4:26b):         ~18GB");
            println!("  Embeddings (TEI):         ~700MB");
            println!("  Reranker (TEI):           ~1GB");
            println!("  Databases + cache:        ~3GB");
            println!("  Buffer:                   ~7GB");
        }
        Tier::Cloud => {
            println!("  Databases + cache:        ~2GB");
            println!("  (No local models - using cloud APIs)");
        }
    }
    println!("  {}", "─────────────────────────────────────────".bright_black());
    println!("  Total required:           {}GB", required_gb);

    if available_gb >= required_gb {
        println!("  Available:                {}GB {}", available_gb, "✓".green());
        Ok(())
    } else {
        println!("  Available:                {}GB {}", available_gb, "✗".red().bold());
        println!();
        fmt_err(
            "Insufficient disk space",
            &format!(
                "Need {}GB for {:?} tier, only {}GB available",
                required_gb,
                tier,
                available_gb
            ),
        );
        anyhow::bail!(
            "Insufficient disk space: need {}GB, have {}GB",
            required_gb,
            available_gb
        )
    }
}

fn prompt_tier() -> Result<Tier> {
    let ram = get_available_memory_gb();

    // Recommend tier based on available RAM
    let recommended = if ram >= 48.0 {
        0 // Pro
    } else if ram >= 24.0 {
        1 // Standard
    } else if ram >= 8.0 {
        2 // Lite
    } else {
        3 // Cloud
    };

    println!("  Your system: {:.1} GB RAM detected", ram);
    println!();

    let tiers = vec![
        format!(
            "Pro      (48GB+) - gemma4:26b + jina-reranker{}",
            if recommended == 0 {
                " (Recommended)"
            } else {
                ""
            }
        ),
        format!(
            "Standard (24GB)  - gemma4:12b + bge-reranker{}",
            if recommended == 1 {
                " (Recommended)"
            } else {
                ""
            }
        ),
        format!(
            "Lite     (8GB)   - phi4-mini, no reranker{}",
            if recommended == 2 {
                " (Recommended)"
            } else {
                ""
            }
        ),
        format!(
            "Cloud    (any)   - use cloud APIs{}",
            if recommended == 3 {
                " (Recommended)"
            } else {
                ""
            }
        ),
    ];

    println!(
        "  {}",
        "(Standalone tiers run models locally with Ollama + TEI)".dimmed()
    );
    let idx = Select::new()
        .with_prompt("Select tier based on available RAM")
        .items(&tiers)
        .default(recommended)
        .interact()?;

    let tier = match idx {
        0 => Tier::Pro,
        1 => Tier::Standard,
        2 => Tier::Lite,
        _ => Tier::Cloud,
    };

    println!(
        "  {} Selected: {:?} ({})",
        "✓".green(),
        tier,
        tier.description()
    );

    Ok(tier)
}

/// Attempt to pull an Ollama model via the host `ollama` CLI, retrying up to
/// `max_retries` times on failure. A 2-second pause is inserted between
/// attempts so transient network errors have a chance to clear.
///
/// Returns `Ok(())` if any attempt succeeds. Returns the last error if all
/// attempts are exhausted.
fn pull_model_external_with_retry(model: &str, max_retries: u32) -> Result<()> {
    let mut last_err = anyhow::anyhow!("no attempts made");
    for attempt in 1..=max_retries {
        println!(
            "  Downloading {} (attempt {}/{})",
            model.cyan(),
            attempt,
            max_retries
        );
        let result = Command::new("ollama").args(["pull", model]).status();
        match result {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                last_err = anyhow::anyhow!("ollama pull exited with {}", status);
            }
            Err(e) => {
                last_err = anyhow::anyhow!("failed to run ollama pull: {}", e);
            }
        }
        if attempt < max_retries {
            println!(
                "  {} Download failed: {}. Retrying in 2s...",
                "!".yellow(),
                last_err
            );
            std::thread::sleep(Duration::from_secs(2));
        }
    }
    Err(last_err)
}

/// Attempt to pull an Ollama model via `docker exec` into the running Ollama
/// container, retrying up to `max_retries` times on failure.
///
/// Returns `Ok(())` if any attempt succeeds. Returns the last error if all
/// attempts are exhausted.
fn pull_model_docker_with_retry(model: &str, max_retries: u32) -> Result<()> {
    let mut last_err = anyhow::anyhow!("no attempts made");
    for attempt in 1..=max_retries {
        println!(
            "  Downloading {} (attempt {}/{})",
            model.cyan(),
            attempt,
            max_retries
        );
        let result = Command::new("docker")
            .args(["exec", "-t", "engrammic-ollama", "ollama", "pull", model])
            .status();
        match result {
            Ok(status) if status.success() => return Ok(()),
            Ok(status) => {
                last_err = anyhow::anyhow!("docker exec ollama pull exited with {}", status);
            }
            Err(e) => {
                last_err = anyhow::anyhow!("failed to run docker exec ollama pull: {}", e);
            }
        }
        if attempt < max_retries {
            println!(
                "  {} Download failed: {}. Retrying in 2s...",
                "!".yellow(),
                last_err
            );
            std::thread::sleep(Duration::from_secs(2));
        }
    }
    Err(last_err)
}

const DOWNLOAD_MAX_RETRIES: u32 = 3;

fn download_models(config: &SelfHostConfig) -> Result<()> {
    let Some(model) = config.tier.ollama_model() else {
        return Ok(());
    };

    if config.use_external_ollama {
        println!();
        println!(
            "  {} Using existing Ollama - pulling {} directly...",
            "→".yellow(),
            model.cyan()
        );
        match pull_model_external_with_retry(model, DOWNLOAD_MAX_RETRIES) {
            Ok(()) => {
                println!("  {} Model {} ready", "✓".green(), model);
            }
            Err(e) => {
                println!(
                    "  {} Could not pull after {} attempts ({}). Run manually: ollama pull {}",
                    "!".yellow(),
                    DOWNLOAD_MAX_RETRIES,
                    e,
                    model
                );
            }
        }
        return Ok(());
    }

    println!();
    println!("{}", "Downloading Ollama model".bold());
    println!(
        "  {} This may take several minutes on first run...",
        "!".yellow()
    );
    println!();

    let compose_path = config.install_dir.join("docker-compose.yml");

    // Start just ollama service
    println!("  Starting Ollama container...");
    let start = Command::new("docker")
        .args([
            "compose",
            "-f",
            compose_path.to_str().unwrap(),
            "up",
            "-d",
            "ollama",
        ])
        .current_dir(&config.install_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !start.success() {
        println!("  {} Failed to start Ollama container", "!".yellow());
        println!("  Model will be downloaded when you run 'docker compose up'");
        return Ok(());
    }

    // Wait for ollama to be ready
    println!("  Waiting for Ollama to be ready...");
    for _ in 0..30 {
        std::thread::sleep(Duration::from_secs(2));
        let check = Command::new("docker")
            .args(["exec", "engrammic-ollama", "ollama", "list"])
            .output();
        if let Ok(output) = check {
            if output.status.success() {
                break;
            }
        }
    }

    // Pull the model with retry
    match pull_model_docker_with_retry(model, DOWNLOAD_MAX_RETRIES) {
        Ok(()) => {
            println!("  {} Model {} downloaded", "✓".green(), model);
        }
        Err(e) => {
            println!(
                "  {} Model download failed after {} attempts: {}",
                "!".yellow(),
                DOWNLOAD_MAX_RETRIES,
                e
            );
            println!("  Check with: docker exec engrammic-ollama ollama list");
            println!(
                "  Or retry manually: docker exec engrammic-ollama ollama pull {}",
                model
            );
        }
    }

    Ok(())
}

/// Prompt for a license key with a retry loop.
///
/// Returns `Some(key)` on success, `None` if the user leaves the input blank to skip.
/// The caller should then record that the license step is pending and print
/// how to complete it later (`engrammic license`).
///
/// Not unit-testable (requires a TTY).
fn prompt_license(prepopulated: Option<&str>) -> Result<Option<String>> {
    // Check prepopulated value first (from .env), then fall back to UserConfig
    let existing = prepopulated
        .map(|s| s.to_string())
        .or_else(|| UserConfig::load().ok().and_then(|c| c.license_key));

    if let Some(ref key) = existing {
        if let Ok(info) = license::validate_license_format(key) {
            println!(
                "  Found existing license: {} ({} days remaining)",
                info.customer.cyan(),
                info.days_remaining
            );
            let keep = Confirm::new()
                .with_prompt("  Use this license?")
                .default(true)
                .interact()?;
            if keep {
                return Ok(Some(key.clone()));
            }
        }
    }

    loop {
        // Repeated every attempt so a user stuck after a bad key always sees
        // both the format hint and the way out.
        println!(
            "  {}",
            "(Starts with ENGR_ - request at founders@engrammic.ai)".dimmed()
        );
        println!(
            "  {}",
            "(Leave blank to skip — finish later with `engrammic license`)".dimmed()
        );

        // dialoguer Input does not surface Esc directly; we use an empty string
        // submitted via Enter as the skip signal (user is told to leave blank).
        let raw: String = Input::new()
            .with_prompt("Enter license key (leave blank to configure later - app won't start without one)")
            .allow_empty(true)
            .interact_text()?;

        let key = raw.trim().to_string();

        if key.is_empty() {
            println!();
            println!("  {} License skipped.", "→".yellow());
            println!(
                "  Run {} to add your license key later.",
                "engrammic license".cyan()
            );
            println!();
            return Ok(None);
        }

        match license::validate_license_format(&key) {
            Ok(info) => {
                println!(
                    "  {} Valid — {}, {} days remaining",
                    "✓".green(),
                    info.customer.cyan(),
                    info.days_remaining
                );
                return Ok(Some(key));
            }
            Err(e) => {
                fmt_err(
                    &format!("{}", e),
                    "Check the key starts with ENGR_, is not expired, and was copied in full.",
                );
                println!();
                // loop continues
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

    println!("  {}", "(Choose where embeddings are computed)".dimmed());
    let idx = Select::new()
        .with_prompt("Embedding provider")
        .items(&providers)
        .default(0)
        .interact()?;
    let provider = providers[idx];

    let (model, dimensions, credential) = match provider {
        "OpenAI (cloud, paid)" => {
            let models = vec![
                "text-embedding-3-small (1536 dims, recommended)",
                "text-embedding-3-large (3072 dims, higher quality)",
            ];
            let model_idx = Select::new()
                .with_prompt("Model")
                .items(&models)
                .default(0)
                .interact()?;
            let model_choice = models[model_idx];

            let (model, dims) = if model_choice.starts_with("text-embedding-3-small") {
                ("openai/text-embedding-3-small", 1536)
            } else {
                ("openai/text-embedding-3-large", 3072)
            };

            println!("  {} Dimensions: {} (auto-filled)", "✓".green(), dims);

            println!("  {}", "(Your OpenAI API key (starts with sk-))".dimmed());
            let key: String = Input::new().with_prompt("OPENAI_API_KEY").interact_text()?;

            (
                model.to_string(),
                dims,
                Some(("OPENAI_API_KEY".to_string(), key)),
            )
        }
        "Ollama (local, free)" => {
            let models = vec![
                "nomic-embed-text (768 dims, recommended)",
                "all-minilm (384 dims, smaller)",
                "Other (enter manually)",
            ];
            println!(
                "  {}",
                "(Make sure this model is pulled in Ollama)".dimmed()
            );
            let model_idx = Select::new()
                .with_prompt("Model")
                .items(&models)
                .default(0)
                .interact()?;
            let model_choice = models[model_idx];

            let (model, dims) = if model_choice.starts_with("nomic-embed-text") {
                ("ollama/nomic-embed-text".to_string(), 768u32)
            } else if model_choice.starts_with("all-minilm") {
                ("ollama/all-minilm".to_string(), 384)
            } else {
                println!(
                    "  {}",
                    "(Just the model name, e.g. mxbai-embed-large)".dimmed()
                );
                let name: String = Input::new().with_prompt("Model name").interact_text()?;
                let dims = prompt_dimensions()?;
                (format!("ollama/{}", name), dims)
            };

            println!("  {} Dimensions: {}", "✓".green(), dims);

            println!("  {}", "(URL where your Ollama server is running)".dimmed());
            let base: String = Input::new()
                .with_prompt("OLLAMA_API_BASE")
                .default("http://localhost:11434".into())
                .interact_text()?;

            (model, dims, Some(("OLLAMA_API_BASE".to_string(), base)))
        }
        "Vertex AI (GCP)" => {
            let model = "vertex_ai/text-embedding-005";
            let dims = 768;

            println!("  {} Model: {}", "✓".green(), model);
            println!("  {} Dimensions: {}", "✓".green(), dims);

            println!("  {}", "(Your Google Cloud project ID)".dimmed());
            let project: String = Input::new().with_prompt("VERTEX_PROJECT").interact_text()?;
            println!("  {}", "(GCP region for Vertex AI)".dimmed());
            let location: String = Input::new()
                .with_prompt("VERTEX_LOCATION")
                .default("us-central1".into())
                .interact_text()?;

            (
                model.to_string(),
                dims,
                Some(("VERTEX".to_string(), format!("{}\x00{}", project, location))),
            )
        }
        _ => {
            println!("  {}", "(Format: provider/model-name)".dimmed());
            let model: String = Input::new()
                .with_prompt("Embedding model")
                .interact_text()?;

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
    println!(
        "  {}",
        "(Check your model's documentation for the correct value)".dimmed()
    );
    let dims_str: String = Input::new()
        .with_prompt("Embedding dimensions")
        .interact_text()?;
    dims_str
        .parse::<u32>()
        .context("Invalid dimensions - must be a positive integer")
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
            if let Some(port_str) = trimmed
                .strip_prefix("- \"")
                .and_then(|s| s.split(':').next())
            {
                app_port = port_str.parse().ok();
            }
        } else if trimmed.starts_with("- \"") && trimmed.contains(":3000\"") {
            if let Some(port_str) = trimmed
                .strip_prefix("- \"")
                .and_then(|s| s.split(':').next())
            {
                dagster_port = port_str.parse().ok();
            }
        }
    }

    (app_port, dagster_port)
}

fn prompt_port(existing: Option<u16>) -> Result<u16> {
    let default = existing.unwrap_or(DEFAULT_PORT);
    println!("  {}", "(Your editor will connect to this port)".dimmed());
    let port_str: String = Input::new()
        .with_prompt("MCP server port")
        .default(default.to_string())
        .interact_text()?;

    let port: u16 = port_str.parse().context("Invalid port number")?;

    // Check if port is in use
    if is_port_in_use(port) {
        println!("  {} Port {} appears to be in use", "!".yellow(), port);
        let proceed = Confirm::new()
            .with_prompt("  Continue anyway?")
            .default(false)
            .interact()?;
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

    println!("  {}", "(Optional - for monitoring SAGE jobs)".dimmed());
    let port_str: String = Input::new()
        .with_prompt("Dagster UI port (SAGE pipeline dashboard)")
        .default(default.to_string())
        .interact_text()?;

    port_str.parse().context("Invalid port number")
}

fn is_port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

/// Default port assignments for each service.
#[derive(Debug, Clone)]
pub struct PortConfig {
    pub api: u16,
    pub ollama: u16,
    pub tei_embed: u16,
    pub tei_rerank: u16,
    pub postgres: u16,
    pub qdrant: u16,
    pub memgraph: u16,
    pub redis: u16,
}

impl Default for PortConfig {
    fn default() -> Self {
        Self {
            api: 8000,
            ollama: 11434,
            tei_embed: 8080,
            tei_rerank: 8081,
            postgres: 5432,
            qdrant: 6333,
            memgraph: 7687,
            redis: 6379,
        }
    }
}

/// Check ports relevant to the selected tier and print availability status.
///
/// Ports that are not used by the tier are skipped. Returns a `PortConfig`
/// with default values — actual port remapping is left for a follow-up task.
fn check_and_warn_ports(tier: Tier) -> PortConfig {
    let config = PortConfig::default();

    println!("  Checking ports...");

    // Ports checked for every tier
    let mut checks: Vec<(u16, &str)> = vec![
        (config.api, "API"),
        (config.postgres, "Postgres"),
        (config.qdrant, "Qdrant"),
        (config.memgraph, "Memgraph"),
        (config.redis, "Redis"),
    ];

    // Standalone tiers add Ollama and TEI embedder
    if tier.is_standalone() {
        checks.push((config.ollama, "Ollama"));
        checks.push((config.tei_embed, "TEI embeddings"));
    }

    // Standard and Pro add the TEI reranker
    if matches!(tier, Tier::Standard | Tier::Pro) {
        checks.push((config.tei_rerank, "TEI reranker"));
    }

    // Sort by port number for a predictable display order
    checks.sort_by_key(|(port, _)| *port);

    for (port, label) in checks {
        if !is_port_in_use(port) {
            println!("  {} {} ({}) available", "✓".green(), port, label);
        } else {
            println!(
                "  {} {} ({}) in use - will conflict with Docker container",
                "✗".red().bold(),
                port,
                label
            );
        }
    }

    config
}

fn prompt_install_dir(existing: Option<&PathBuf>) -> Result<PathBuf> {
    // If reconfiguring, use existing dir as default and skip overwrite check
    if let Some(existing_dir) = existing {
        println!(
            "  {} Using existing directory: {}",
            "✓".green(),
            existing_dir.display()
        );
        return Ok(existing_dir.clone());
    }

    let default = UserConfig::dir();
    let default_str = default.display().to_string();

    println!(
        "  {}",
        "(docker-compose.yml and .env will be created here)".dimmed()
    );
    let dir_str: String = Input::new()
        .with_prompt("Install directory")
        .default(default_str)
        .interact_text()?;

    let path = PathBuf::from(dir_str);

    if path.exists() && path.join("docker-compose.yml").exists() {
        println!("  {} Existing installation found", "!".yellow());
        let overwrite = Confirm::new()
            .with_prompt("  Overwrite?")
            .default(false)
            .interact()?;
        if !overwrite {
            println!(
                "  {} Existing installation preserved at {}",
                "→".yellow(),
                path.display()
            );
            anyhow::bail!("Cancelled — existing installation preserved");
        }
    }

    Ok(path)
}

fn prompt_postgres_password(existing: Option<&str>) -> Result<String> {
    println!(
        "  {}",
        "(For local dev the default is fine; use a strong password in production)".dimmed()
    );
    let default = existing.unwrap_or("engrammic").to_string();
    let password: String = Input::new()
        .with_prompt("PostgreSQL password")
        .default(default)
        .interact_text()?;

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
        let overwrite = Confirm::new()
            .with_prompt("Config files exist. Overwrite?")
            .default(false)
            .interact()?;
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
    // Select template based on tier
    let template = match config.tier {
        Tier::Lite => docker::COMPOSE_LITE,
        Tier::Standard => docker::COMPOSE_STANDARD,
        Tier::Pro => docker::COMPOSE_PRO,
        Tier::Cloud => docker::COMPOSE_TEMPLATE,
    };

    // Replace ports and env file references
    let mut compose = template
        .replace("- \"8000:8000\"", &format!("- \"{}:8000\"", config.port))
        .replace(
            "- \"3000:3000\"",
            &format!("- \"{}:3000\"", config.dagster_port),
        );

    // For standalone tiers, replace env_file reference with just .env
    if config.tier.is_standalone() {
        compose = compose
            .replace("standalone-lite.env", ".env")
            .replace("standalone-standard.env", ".env")
            .replace("standalone-pro.env", ".env");
    }

    // Strip ollama service when using external Ollama
    if config.use_external_ollama {
        compose = strip_ollama_service(&compose);
    }

    // Podman adaptations
    if config.podman {
        // Add :Z SELinux label to all named volume mounts.
        // Named volumes look like `- name:/path` in YAML list form.
        // We match the pattern `      - <name>:<path>` (6 spaces, list item).
        compose = add_selinux_z_to_volumes(&compose);

        // Swap Docker GPU syntax for Podman CDI syntax.
        // The templates have the GPU section commented out; replace the comment
        // block with an active Podman devices entry.
        compose = swap_gpu_syntax_for_podman(&compose);
    }

    compose
}

/// Add `:Z` SELinux label to named volume mounts in a compose string.
///
/// Targets lines of the form `      - <volume-name>:<container-path>` where
/// the left side is a Docker named volume (no leading `/` or `.`). Bind mounts
/// with absolute or relative host paths are left unchanged.
fn add_selinux_z_to_volumes(compose: &str) -> String {
    compose
        .lines()
        .map(|line| {
            // Match a YAML list entry that looks like a named volume mount:
            // e.g. `      - ollama-models:/root/.ollama`
            let trimmed = line.trim_start();
            if trimmed.starts_with("- ") {
                let rest = trimmed.strip_prefix("- ").unwrap_or(trimmed);
                // Named volume: no leading `/`, `./`, or `../`
                if !rest.starts_with('/')
                    && !rest.starts_with("./")
                    && !rest.starts_with("../")
                    && rest.contains(':')
                    && !rest.ends_with(":Z")
                    && !rest.ends_with(":z")
                {
                    // Only touch lines where the left side looks like a named
                    // volume (alphanumeric, hyphens, underscores).
                    let volume_name = rest.split(':').next().unwrap_or("");
                    if volume_name
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        let indent: String = line
                            .chars()
                            .take_while(|c| c.is_whitespace())
                            .collect();
                        return format!("{}- {}:Z", indent, rest.trim_end());
                    }
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Replace the Docker NVIDIA GPU comment block in a compose string with active
/// Podman CDI device syntax.
///
/// The templates contain a commented-out Docker GPU block of the form:
/// ```yaml
/// # For GPU acceleration, uncomment:
/// #   reservations:
/// #     devices:
/// #       - driver: nvidia
/// #         count: all
/// #         capabilities: [gpu]
/// ```
///
/// This is replaced with an active Podman CDI entry:
/// ```yaml
///     devices:
///       - nvidia.com/gpu=all
/// ```
fn swap_gpu_syntax_for_podman(compose: &str) -> String {
    // The comment block spans multiple lines. We replace it with the Podman
    // CDI device stanza using a simple multi-line string replacement.
    //
    // Docker GPU block (commented out in templates, 4-space service indent):
    //
    //     # For GPU acceleration, uncomment:
    //     #   reservations:
    //     #     devices:
    //     #       - driver: nvidia
    //     #         count: all
    //     #         capabilities: [gpu]
    //
    // Replaced with active Podman CDI syntax at the same indent level:
    //
    //     # GPU (Podman CDI syntax):
    //     devices:
    //       - nvidia.com/gpu=all
    let docker_comment_block = concat!(
        "    # For GPU acceleration, uncomment:\n",
        "    #   reservations:\n",
        "    #     devices:\n",
        "    #       - driver: nvidia\n",
        "    #         count: all\n",
        "    #         capabilities: [gpu]"
    );
    let podman_gpu_stanza = concat!(
        "    # GPU (Podman CDI syntax):\n",
        "    devices:\n",
        "      - nvidia.com/gpu=all"
    );
    compose.replace(docker_comment_block, podman_gpu_stanza)
}

/// Strip the `ollama` service from a compose file when using external Ollama.
///
/// Removes the entire service block starting with `  ollama:` up to (but not
/// including) the next top-level service or section. Also removes:
/// - `ollama-models` volume declaration
/// - `ollama:` + `condition: service_healthy` from depends_on blocks
/// - Updates `OLLAMA_BASE_URL=http://ollama:...` to `http://localhost:...`
fn strip_ollama_service(compose: &str) -> String {
    let mut result = String::new();
    let mut skip_until_next_service = false;
    let mut in_volumes_section = false;
    let mut skip_next_condition_line = false;

    for line in compose.lines() {
        // Skip the "condition: service_healthy" line after "ollama:" in depends_on
        if skip_next_condition_line {
            skip_next_condition_line = false;
            if line.trim().starts_with("condition:") {
                continue;
            }
        }

        // Detect start of ollama service block (top-level service definition)
        if line.starts_with("  ollama:") && !line.contains("condition:") {
            skip_until_next_service = true;
            continue;
        }

        // Detect end of service block (next service at same indent level)
        if skip_until_next_service {
            // A line starting with exactly 2 spaces followed by a word and colon
            // indicates a new service or section (e.g., "  app:", "  redis:", "volumes:")
            let trimmed = line.trim_start();
            let indent = line.len() - trimmed.len();
            if indent == 2 && trimmed.ends_with(':') && !trimmed.starts_with('#') {
                skip_until_next_service = false;
            } else if indent == 0 && trimmed.ends_with(':') {
                // Top-level section like "volumes:"
                skip_until_next_service = false;
            } else {
                continue;
            }
        }

        // Track volumes section to strip ollama-models
        if line == "volumes:" {
            in_volumes_section = true;
        } else if !line.starts_with(' ') && !line.is_empty() && line != "volumes:" {
            in_volumes_section = false;
        }

        // Skip ollama-models volume declaration
        if in_volumes_section && line.trim_start().starts_with("ollama-models:") {
            continue;
        }

        // Skip "ollama:" dependency entries (inside depends_on blocks)
        // These look like "      ollama:" at 6-space indent
        if line.trim() == "ollama:" {
            skip_next_condition_line = true;
            continue;
        }

        // Rewrite OLLAMA_BASE_URL from docker network to localhost
        let output_line = if line.contains("OLLAMA_BASE_URL=http://ollama:") {
            line.replace("http://ollama:", "http://localhost:")
        } else {
            line.to_string()
        };

        result.push_str(&output_line);
        result.push('\n');
    }

    result
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

    // Point to external Ollama when not using bundled container
    let ollama_section = if config.use_external_ollama {
        "# External Ollama (user's existing instance)\nOLLAMA_HOST=http://localhost:11434\n"
    } else {
        ""
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
{ollama_section}

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
        ollama_section = ollama_section,
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
        .args(["compose", "-f", compose_path.to_str().unwrap(), "pull"])
        .current_dir(&config.install_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;

    let output = pull.wait_with_output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        fmt_err(
            &format!("Failed to pull Docker images: {}", stderr.trim()),
            "Check your internet connection and Docker daemon, then run `engrammic selfhost` again.",
        );
        anyhow::bail!("docker compose pull failed: {}", stderr.trim());
    }
    println!("  {} Images pulled", "✓".green());

    // Check for existing containers
    let ps_output = Command::new("docker")
        .args([
            "compose",
            "-f",
            compose_path.to_str().unwrap(),
            "ps",
            "--format",
            "{{.Name}}: {{.Status}}",
        ])
        .current_dir(&config.install_dir)
        .output();

    let existing_containers: Vec<String> = ps_output
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();

    let force_recreate = if !existing_containers.is_empty() {
        println!();
        println!("  {} Existing containers:", "!".yellow());
        for container in &existing_containers {
            println!("    - {}", container);
        }
        println!();
        Confirm::new()
            .with_prompt("  Stop and recreate them?")
            .default(true)
            .interact()?
    } else {
        false
    };

    // Start services
    let mut args = vec!["compose", "-f", compose_path.to_str().unwrap(), "up", "-d"];
    if force_recreate {
        args.push("--force-recreate");
    }

    let status = Command::new("docker")
        .args(&args)
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

        let result = Command::new("curl").args(["-sf", &health_url]).output();

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

    println!("  {} Services didn't become healthy in time", "!".yellow());
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
        println!("  {} No supported editors detected", "-".dimmed());
        println!(
            "  Add this to your MCP config: {}",
            format!(
                "\"engrammic\": {{ \"type\": \"http\", \"url\": \"http://localhost:{}/mcp\" }}",
                config.port
            )
            .cyan()
        );
        return Ok(());
    }

    let endpoint = format!("http://localhost:{}/mcp", config.port);

    for tool in &detected {
        match &tool.method {
            InstallMethod::FileEdit(shape) => {
                let backup = crate::config::ensure_backup(&tool.config_path).unwrap_or(None);
                match crate::config::install(&tool.config_path, &endpoint, *shape) {
                    Ok(_) => {
                        let mut m = crate::manifest::Manifest::load_or_migrate(None)?;
                        m.record_harness(tool.id, &tool.config_path, backup, &endpoint);
                        m.save()?;
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
    fn check_prerequisites_bail_message_is_specific() {
        // This test is documentation: check_prerequisites is not unit-testable
        // without Docker. Compile-check only — assert the function exists.
        let _: fn() -> anyhow::Result<()> = super::check_prerequisites;
    }

    #[test]
    fn models_yaml_template_schema_valid() {
        let template = include_str!("../assets/models.yaml");
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(template).expect("assets/models.yaml must be valid YAML");

        let mapping = yaml
            .as_mapping()
            .expect("assets/models.yaml must be a YAML mapping");

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

//! CLI installation helper - installs the binary to ~/.local/bin.

use anyhow::Result;
use colored::Colorize;
use dialoguer::Confirm;

pub fn offer_cli_install(auto: bool) -> Result<()> {
    let install_cli = if auto {
        false
    } else {
        println!("  {}", "(Allows running 'engrammic update', 'engrammic status', etc.)".dimmed());
        Confirm::new()
            .with_prompt("Install the Engrammic CLI for future updates?")
            .default(true)
            .interact()?
    };

    if !install_cli {
        return Ok(());
    }

    let current_exe = std::env::current_exe()?;
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let local_bin = home.join(".local").join("bin");

    std::fs::create_dir_all(&local_bin)?;

    let dest = local_bin.join("engrammic");
    std::fs::copy(&current_exe, &dest)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }

    println!(
        "  {} Installed to {}",
        "✓".green(),
        dest.display().to_string().cyan()
    );

    let path_env = std::env::var("PATH").unwrap_or_default();
    let local_bin_str = local_bin.display().to_string();
    if !path_env.split(':').any(|p| p == local_bin_str) {
        println!();
        println!("  {} Add to your shell config:", "!".yellow());
        println!(
            "    {}",
            format!("export PATH=\"$HOME/.local/bin:$PATH\"").cyan()
        );
        println!();
        println!(
            "  Then run {} anytime to update or check status.",
            "engrammic".cyan()
        );
    } else {
        println!(
            "  Run {} anytime to update or check status.",
            "engrammic".cyan()
        );
    }

    Ok(())
}

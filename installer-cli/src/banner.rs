use colored::Colorize;

// Oxide red border, bone white text. Hex values are tunable.
const OXIDE: (u8, u8, u8) = (0xA3, 0x3B, 0x2A);
const BONE: (u8, u8, u8) = (0xE9, 0xE2, 0xD2);
const INNER_WIDTH: usize = 45;

/// The text content of the banner, one entry per content line.
pub fn banner_lines() -> Vec<String> {
    vec![
        "engrammic   MCP Installer".to_string(),
        "epistemic memory for AI agents".to_string(),
        "engrammic.ai".to_string(),
    ]
}

pub fn print_banner() {
    let (o0, o1, o2) = OXIDE;
    let (b0, b1, b2) = BONE;
    let edge = |s: &str| s.truecolor(o0, o1, o2);

    let border = "─".repeat(INNER_WIDTH);
    let blank = " ".repeat(INNER_WIDTH);

    println!();
    println!("  {}", edge(&format!("╭{}╮", border)));
    println!("  {}{}{}", edge("│"), blank, edge("│"));

    for (i, line) in banner_lines().iter().enumerate() {
        let padded = format!("   {:<width$}", line, width = INNER_WIDTH - 3);
        let colored = if i == 0 {
            // Bold the product name at the start of the first line.
            let rest = padded.replacen("engrammic", "", 1);
            format!(
                "   {}{}",
                "engrammic".truecolor(b0, b1, b2).bold(),
                format!("{:<width$}", rest.trim_start(), width = INNER_WIDTH - 12)
                    .truecolor(b0, b1, b2)
            )
        } else if i == 2 {
            format!(
                "   {}{}",
                "→ ".truecolor(o0, o1, o2),
                format!("{:<width$}", line, width = INNER_WIDTH - 5)
                    .truecolor(b0, b1, b2)
            )
        } else {
            padded.truecolor(b0, b1, b2).to_string()
        };
        println!("  {}{}{}", edge("│"), colored, edge("│"));
    }

    println!("  {}{}{}", edge("│"), blank, edge("│"));
    println!("  {}", edge(&format!("╰{}╯", border)));
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn banner_lines_include_name_and_link() {
        let lines = banner_lines();
        assert!(lines.iter().any(|l| l.contains("engrammic")));
        assert!(lines.iter().any(|l| l.contains("engrammic.ai")));
    }
}

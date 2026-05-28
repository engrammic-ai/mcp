use colored::Colorize;

const VIOLET: (u8, u8, u8) = (0x7E, 0x57, 0xC2);
const INNER_WIDTH: usize = 45;

/// The text content of the banner, one entry per content line.
pub fn banner_lines() -> Vec<String> {
    let version = env!("CARGO_PKG_VERSION");
    vec![
        format!("engrammic   MCP Setup v{}", version),
        "epistemic memory for AI agents".to_string(),
        "engrammic.ai".to_string(),
    ]
}

pub fn print_banner() {
    let (v0, v1, v2) = VIOLET;
    let edge = |s: &str| s.truecolor(v0, v1, v2);

    let border = "─".repeat(INNER_WIDTH);
    let blank = " ".repeat(INNER_WIDTH);

    println!();
    println!("  {}", edge(&format!("╭{}╮", border)));
    println!("  {}{}{}", edge("│"), blank, edge("│"));

    for (i, line) in banner_lines().iter().enumerate() {
        let padded = format!("   {:<width$}", line, width = INNER_WIDTH - 3);
        let content = if i == 0 {
            let name = "engrammic";
            let rest = &line[name.len()..];
            format!(
                "   {}{}",
                name.truecolor(v0, v1, v2).bold(),
                format!("{:<width$}", rest, width = INNER_WIDTH - 3 - name.len())
            )
        } else if i == 2 {
            format!(
                "   {}{}",
                "→ ".truecolor(v0, v1, v2),
                format!("{:<width$}", line, width = INNER_WIDTH - 5)
            )
        } else {
            padded
        };
        println!("  {}{}{}", edge("│"), content, edge("│"));
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

    #[test]
    fn print_banner_does_not_panic() {
        print_banner();
    }
}

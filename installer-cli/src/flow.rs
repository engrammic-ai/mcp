//! Interview → plan → execute flow types and pure helpers.
//!
//! Interview functions ask everything up front with zero side effects; the
//! execute engine runs steps skip-and-continue and reports per-step results.
//! Pure helpers live here so they are unit-testable without a TTY.

use crate::tools::{SkillDest, Tool};

/// Everything the wizard needs to know, collected before any mutation.
// Note: Tool and SkillDest do not derive Debug, so Answers cannot either.
pub struct Answers {
    pub endpoint: String,
    pub to_install: Vec<Tool>,
    pub to_remove: Vec<Tool>,
    pub skill_dests: Vec<SkillDest>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Done,
    Failed(String),
    /// Needs a user action we cannot perform (deep-link approval, GUI config).
    Manual(String),
}

#[derive(Debug)]
pub struct StepResult {
    pub label: String,
    pub outcome: Outcome,
}

/// Display label for a harness in the selection prompt. Detection is shown as
/// information, never as a pre-checked default (spec: "nothing is pre-checked").
pub fn harness_label(tool: &Tool, detected: bool, configured: bool) -> String {
    if configured {
        format!("{}  (already configured)", tool.name)
    } else if detected {
        format!("{}  (detected)", tool.name)
    } else {
        tool.name.to_string()
    }
}

/// Human-readable recap of everything about to happen, shown before execution.
pub fn render_plan(answers: &Answers) -> String {
    let mut out = String::from("About to:\n");
    if answers.to_install.is_empty() {
        out.push_str("  • configure no editors\n");
    } else {
        let names: Vec<&str> = answers.to_install.iter().map(|t| t.name).collect();
        out.push_str(&format!("  • configure: {}\n", names.join(", ")));
    }
    for t in &answers.to_remove {
        out.push_str(&format!("  • remove Engrammic from: {}\n", t.name));
    }
    if answers.skill_dests.is_empty() {
        out.push_str("  • install no skills\n");
    } else {
        let names: Vec<&str> = answers.skill_dests.iter().map(|d| d.name).collect();
        out.push_str(&format!("  • install skills to: {}\n", names.join(", ")));
    }
    out.push_str(&format!("  • endpoint: {}\n", answers.endpoint));
    out
}

/// (done, failed, manual) counts for the final summary line.
pub fn summarize_results(results: &[StepResult]) -> (usize, usize, usize) {
    let mut done = 0;
    let mut failed = 0;
    let mut manual = 0;
    for r in results {
        match r.outcome {
            Outcome::Done => done += 1,
            Outcome::Failed(_) => failed += 1,
            Outcome::Manual(_) => manual += 1,
        }
    }
    (done, failed, manual)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    fn tool(id: &str) -> Tool {
        Tool::from_id(id).expect("known tool id")
    }

    #[test]
    fn harness_label_marks_detected_and_configured() {
        let t = tool("claude");
        assert_eq!(harness_label(&t, true, false), "Claude Code  (detected)");
        assert_eq!(harness_label(&t, true, true), "Claude Code  (already configured)");
        assert_eq!(harness_label(&t, false, false), "Claude Code");
    }

    #[test]
    fn render_plan_lists_all_decisions() {
        let answers = Answers {
            endpoint: "https://beta.engrammic.ai/mcp/".to_string(),
            to_install: vec![tool("claude"), tool("windsurf")],
            to_remove: vec![],
            skill_dests: vec![],
        };
        let plan = render_plan(&answers);
        assert!(plan.contains("Claude Code"));
        assert!(plan.contains("Windsurf"));
        assert!(plan.contains("beta.engrammic.ai"));
        assert!(plan.contains("no skills"), "empty skill dests must be stated, not omitted");
    }

    #[test]
    fn summarize_results_counts_outcomes() {
        let results = vec![
            StepResult { label: "Claude Code".into(), outcome: Outcome::Done },
            StepResult { label: "Windsurf".into(), outcome: Outcome::Failed("permission denied".into()) },
            StepResult { label: "Cursor".into(), outcome: Outcome::Manual("open this link".into()) },
        ];
        let (ok, failed, manual) = summarize_results(&results);
        assert_eq!((ok, failed, manual), (1, 1, 1));
    }
}

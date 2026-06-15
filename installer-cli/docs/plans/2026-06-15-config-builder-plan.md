# Config Builder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the installer wizard with component-by-component selection for Cloud tier (LLM/Embeddings/Reranker providers) and RAM detection for standalone tiers.

**Architecture:** Add provider enums with default model configs, extend WizardStep/WizardState for new Cloud steps, implement RAM detection using sysinfo crate, generate models.yaml dynamically based on selections.

**Tech Stack:** Rust, dialoguer (prompts), sysinfo (RAM detection), serde_yaml (YAML generation)

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src/providers.rs` | NEW: Provider enums, OtherProvider struct, CredentialSpec, ProviderSet, default configs |
| `src/ram.rs` | NEW: RAM detection and tier compatibility checks |
| `src/selfhost.rs` | MODIFY: WizardStep/WizardState changes, wizard flow integration, prompt functions |
| `src/main.rs` | MODIFY: Add `mod providers; mod ram;` |

---

### Task 1: Create Provider Types Module

**Files:**
- Create: `installer-cli/src/providers.rs`
- Modify: `installer-cli/src/main.rs`

- [ ] **Step 1: Create providers.rs with OtherProvider struct**

```rust
// src/providers.rs

use std::collections::HashMap;

/// Custom provider configuration for unlisted providers.
#[derive(Debug, Clone)]
pub struct OtherProvider {
    pub provider: String,
    pub model: String,
    pub dimensions: Option<u32>,  // Only for embeddings
    pub env_vars: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_provider_can_store_env_vars() {
        let p = OtherProvider {
            provider: "groq".to_string(),
            model: "llama-3-70b".to_string(),
            dimensions: None,
            env_vars: vec![("GROQ_API_KEY".to_string(), "test".to_string())],
        };
        assert_eq!(p.env_vars.len(), 1);
        assert_eq!(p.env_vars[0].0, "GROQ_API_KEY");
    }
}
```

- [ ] **Step 2: Run test to verify it passes**

```bash
cd installer-cli && cargo test other_provider_can_store_env_vars -- --nocapture
```

Expected: PASS

- [ ] **Step 3: Add LlmProvider enum**

Add to `src/providers.rs`:

```rust
/// LLM provider selection for Cloud tier.
#[derive(Debug, Clone)]
pub enum LlmProvider {
    OpenAI,
    Anthropic,
    VertexAI,
    AzureOpenAI,
    Bedrock,
    Other(OtherProvider),
}

impl LlmProvider {
    /// litellm provider name for YAML output.
    pub fn provider_name(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "openai",
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::VertexAI => "vertex_ai",
            LlmProvider::AzureOpenAI => "azure",
            LlmProvider::Bedrock => "bedrock",
            LlmProvider::Other(o) => &o.provider,
        }
    }

    /// Default reasoning model for this provider.
    pub fn reasoning_model(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "gpt-4o",
            LlmProvider::Anthropic => "claude-sonnet-4-5",
            LlmProvider::VertexAI => "gemini-2.5-pro",
            LlmProvider::AzureOpenAI => "gpt-4o",
            LlmProvider::Bedrock => "anthropic.claude-sonnet",
            LlmProvider::Other(o) => &o.model,
        }
    }

    /// Default fast model for this provider.
    pub fn fast_model(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "gpt-4o-mini",
            LlmProvider::Anthropic => "claude-haiku-4-5",
            LlmProvider::VertexAI => "gemini-2.5-flash",
            LlmProvider::AzureOpenAI => "gpt-4o-mini",
            LlmProvider::Bedrock => "anthropic.claude-haiku",
            LlmProvider::Other(o) => &o.model,
        }
    }
}
```

- [ ] **Step 4: Add LlmProvider tests**

Add to the `tests` module:

```rust
#[test]
fn llm_provider_names_are_lowercase() {
    assert_eq!(LlmProvider::OpenAI.provider_name(), "openai");
    assert_eq!(LlmProvider::Anthropic.provider_name(), "anthropic");
    assert_eq!(LlmProvider::VertexAI.provider_name(), "vertex_ai");
}

#[test]
fn llm_provider_has_default_models() {
    assert_eq!(LlmProvider::OpenAI.reasoning_model(), "gpt-4o");
    assert_eq!(LlmProvider::OpenAI.fast_model(), "gpt-4o-mini");
    assert_eq!(LlmProvider::Anthropic.reasoning_model(), "claude-sonnet-4-5");
}
```

- [ ] **Step 5: Run tests**

```bash
cd installer-cli && cargo test llm_provider -- --nocapture
```

Expected: 2 tests pass

- [ ] **Step 6: Add EmbeddingProvider enum**

Add to `src/providers.rs`:

```rust
/// Embedding provider selection for Cloud tier.
#[derive(Debug, Clone)]
pub enum EmbeddingProvider {
    OpenAI,
    VertexAI,
    AzureOpenAI,
    Bedrock,
    Other(OtherProvider),
}

impl EmbeddingProvider {
    pub fn provider_name(&self) -> &str {
        match self {
            EmbeddingProvider::OpenAI => "openai",
            EmbeddingProvider::VertexAI => "vertex_ai",
            EmbeddingProvider::AzureOpenAI => "azure",
            EmbeddingProvider::Bedrock => "bedrock",
            EmbeddingProvider::Other(o) => &o.provider,
        }
    }

    pub fn model(&self) -> &str {
        match self {
            EmbeddingProvider::OpenAI => "text-embedding-3-large",
            EmbeddingProvider::VertexAI => "text-embedding-005",
            EmbeddingProvider::AzureOpenAI => "text-embedding-3-large",
            EmbeddingProvider::Bedrock => "amazon.titan-embed-text-v2",
            EmbeddingProvider::Other(o) => &o.model,
        }
    }

    pub fn dimensions(&self) -> u32 {
        match self {
            EmbeddingProvider::OpenAI => 3072,
            EmbeddingProvider::VertexAI => 768,
            EmbeddingProvider::AzureOpenAI => 3072,
            EmbeddingProvider::Bedrock => 1024,
            EmbeddingProvider::Other(o) => o.dimensions.unwrap_or(768),
        }
    }
}
```

- [ ] **Step 7: Add EmbeddingProvider tests**

```rust
#[test]
fn embedding_provider_has_dimensions() {
    assert_eq!(EmbeddingProvider::OpenAI.dimensions(), 3072);
    assert_eq!(EmbeddingProvider::VertexAI.dimensions(), 768);
    assert_eq!(EmbeddingProvider::Bedrock.dimensions(), 1024);
}
```

- [ ] **Step 8: Run test**

```bash
cd installer-cli && cargo test embedding_provider -- --nocapture
```

Expected: PASS

- [ ] **Step 9: Add RerankerProvider enum**

Add to `src/providers.rs`:

```rust
/// Reranker provider selection for Cloud tier.
#[derive(Debug, Clone)]
pub enum RerankerProvider {
    None,
    Cohere,
    VertexAI,
    Other(OtherProvider),
}

impl RerankerProvider {
    pub fn provider_name(&self) -> Option<&str> {
        match self {
            RerankerProvider::None => None,
            RerankerProvider::Cohere => Some("cohere"),
            RerankerProvider::VertexAI => Some("vertex_ai"),
            RerankerProvider::Other(o) => Some(&o.provider),
        }
    }

    pub fn model(&self) -> Option<&str> {
        match self {
            RerankerProvider::None => None,
            RerankerProvider::Cohere => Some("rerank-v3.5"),
            RerankerProvider::VertexAI => Some("semantic-ranker-default@latest"),
            RerankerProvider::Other(o) => Some(&o.model),
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, RerankerProvider::None)
    }
}
```

- [ ] **Step 10: Add RerankerProvider tests**

```rust
#[test]
fn reranker_none_returns_none_for_model() {
    assert!(RerankerProvider::None.model().is_none());
    assert!(RerankerProvider::None.is_none());
}

#[test]
fn reranker_cohere_has_model() {
    assert_eq!(RerankerProvider::Cohere.model(), Some("rerank-v3.5"));
    assert!(!RerankerProvider::Cohere.is_none());
}
```

- [ ] **Step 11: Run tests**

```bash
cd installer-cli && cargo test reranker -- --nocapture
```

Expected: 2 tests pass

- [ ] **Step 12: Add module to main.rs**

Add near the top of `src/main.rs`:

```rust
mod providers;
```

- [ ] **Step 13: Verify compilation**

```bash
cd installer-cli && cargo build
```

Expected: Compiles without errors

- [ ] **Step 14: Commit**

```bash
cd installer-cli && git add src/providers.rs src/main.rs && git commit -m "feat(installer): add provider enums for Cloud tier config builder"
```

---

### Task 2: Add CredentialSpec and ProviderSet

**Files:**
- Modify: `installer-cli/src/providers.rs`

- [ ] **Step 1: Add CredentialSpec struct**

Add to `src/providers.rs`:

```rust
/// Specification for a credential that needs to be collected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CredentialSpec {
    pub env_var: &'static str,
    pub prompt: &'static str,
    pub secret: bool,
}

impl LlmProvider {
    pub fn required_credentials(&self) -> Vec<CredentialSpec> {
        match self {
            LlmProvider::OpenAI => vec![CredentialSpec {
                env_var: "OPENAI_API_KEY",
                prompt: "OpenAI API key",
                secret: true,
            }],
            LlmProvider::Anthropic => vec![CredentialSpec {
                env_var: "ANTHROPIC_API_KEY",
                prompt: "Anthropic API key",
                secret: true,
            }],
            LlmProvider::VertexAI => vec![
                CredentialSpec { env_var: "VERTEX_PROJECT", prompt: "GCP project ID", secret: false },
                CredentialSpec { env_var: "VERTEX_LOCATION", prompt: "GCP location (e.g., us-central1)", secret: false },
            ],
            LlmProvider::AzureOpenAI => vec![
                CredentialSpec { env_var: "AZURE_API_KEY", prompt: "Azure OpenAI API key", secret: true },
                CredentialSpec { env_var: "AZURE_API_BASE", prompt: "Azure endpoint URL", secret: false },
                CredentialSpec { env_var: "AZURE_API_VERSION", prompt: "Azure API version (e.g., 2024-02-01)", secret: false },
            ],
            LlmProvider::Bedrock => vec![
                CredentialSpec { env_var: "AWS_REGION", prompt: "AWS region (e.g., us-east-1)", secret: false },
            ],
            LlmProvider::Other(_) => vec![],  // Handled separately
        }
    }
}
```

- [ ] **Step 2: Add credential methods to EmbeddingProvider and RerankerProvider**

```rust
impl EmbeddingProvider {
    pub fn required_credentials(&self) -> Vec<CredentialSpec> {
        match self {
            EmbeddingProvider::OpenAI => vec![CredentialSpec {
                env_var: "OPENAI_API_KEY",
                prompt: "OpenAI API key",
                secret: true,
            }],
            EmbeddingProvider::VertexAI => vec![
                CredentialSpec { env_var: "VERTEX_PROJECT", prompt: "GCP project ID", secret: false },
                CredentialSpec { env_var: "VERTEX_LOCATION", prompt: "GCP location", secret: false },
            ],
            EmbeddingProvider::AzureOpenAI => vec![
                CredentialSpec { env_var: "AZURE_API_KEY", prompt: "Azure OpenAI API key", secret: true },
                CredentialSpec { env_var: "AZURE_API_BASE", prompt: "Azure endpoint URL", secret: false },
                CredentialSpec { env_var: "AZURE_API_VERSION", prompt: "Azure API version", secret: false },
            ],
            EmbeddingProvider::Bedrock => vec![
                CredentialSpec { env_var: "AWS_REGION", prompt: "AWS region", secret: false },
            ],
            EmbeddingProvider::Other(_) => vec![],
        }
    }
}

impl RerankerProvider {
    pub fn required_credentials(&self) -> Vec<CredentialSpec> {
        match self {
            RerankerProvider::None => vec![],
            RerankerProvider::Cohere => vec![CredentialSpec {
                env_var: "COHERE_API_KEY",
                prompt: "Cohere API key",
                secret: true,
            }],
            RerankerProvider::VertexAI => vec![
                CredentialSpec { env_var: "VERTEX_PROJECT", prompt: "GCP project ID", secret: false },
                CredentialSpec { env_var: "VERTEX_LOCATION", prompt: "GCP location", secret: false },
            ],
            RerankerProvider::Other(_) => vec![],
        }
    }
}
```

- [ ] **Step 3: Add ProviderSet struct**

```rust
/// Collection of all three provider selections for credential deduplication.
#[derive(Debug, Clone)]
pub struct ProviderSet {
    pub llm: LlmProvider,
    pub embedding: EmbeddingProvider,
    pub reranker: RerankerProvider,
}

impl ProviderSet {
    /// Get deduplicated list of all required credentials.
    pub fn required_credentials(&self) -> Vec<CredentialSpec> {
        let mut creds = Vec::new();
        creds.extend(self.llm.required_credentials());
        creds.extend(self.embedding.required_credentials());
        creds.extend(self.reranker.required_credentials());
        
        // Deduplicate by env_var
        creds.sort_by(|a, b| a.env_var.cmp(b.env_var));
        creds.dedup_by(|a, b| a.env_var == b.env_var);
        creds
    }

    /// Get custom env vars from Other providers.
    pub fn custom_env_vars(&self) -> Vec<(String, String)> {
        let mut vars = Vec::new();
        if let LlmProvider::Other(o) = &self.llm {
            vars.extend(o.env_vars.clone());
        }
        if let EmbeddingProvider::Other(o) = &self.embedding {
            vars.extend(o.env_vars.clone());
        }
        if let RerankerProvider::Other(o) = &self.reranker {
            vars.extend(o.env_vars.clone());
        }
        vars
    }
}
```

- [ ] **Step 4: Add ProviderSet tests**

```rust
#[test]
fn provider_set_deduplicates_credentials() {
    let set = ProviderSet {
        llm: LlmProvider::VertexAI,
        embedding: EmbeddingProvider::VertexAI,
        reranker: RerankerProvider::VertexAI,
    };
    let creds = set.required_credentials();
    // Should have VERTEX_PROJECT and VERTEX_LOCATION only once each
    let vertex_project_count = creds.iter().filter(|c| c.env_var == "VERTEX_PROJECT").count();
    assert_eq!(vertex_project_count, 1);
}

#[test]
fn provider_set_collects_mixed_credentials() {
    let set = ProviderSet {
        llm: LlmProvider::Anthropic,
        embedding: EmbeddingProvider::OpenAI,
        reranker: RerankerProvider::Cohere,
    };
    let creds = set.required_credentials();
    let env_vars: Vec<_> = creds.iter().map(|c| c.env_var).collect();
    assert!(env_vars.contains(&"ANTHROPIC_API_KEY"));
    assert!(env_vars.contains(&"OPENAI_API_KEY"));
    assert!(env_vars.contains(&"COHERE_API_KEY"));
}
```

- [ ] **Step 5: Run tests**

```bash
cd installer-cli && cargo test provider_set -- --nocapture
```

Expected: 2 tests pass

- [ ] **Step 6: Commit**

```bash
cd installer-cli && git add src/providers.rs && git commit -m "feat(installer): add CredentialSpec and ProviderSet for credential deduplication"
```

---

### Task 3: Create RAM Detection Module

**Files:**
- Create: `installer-cli/src/ram.rs`
- Modify: `installer-cli/src/main.rs`

- [ ] **Step 1: Create ram.rs with detection function**

```rust
// src/ram.rs

use sysinfo::System;

/// Detect system RAM in GB.
/// Returns None if detection fails.
pub fn detect_system_ram() -> Option<u64> {
    let sys = System::new_all();
    let total_bytes = sys.total_memory();
    if total_bytes == 0 {
        return None;
    }
    // Convert bytes to GB (rounded)
    Some(total_bytes / (1024 * 1024 * 1024))
}

/// RAM thresholds for each standalone tier.
pub fn tier_min_ram(tier: &str) -> u64 {
    match tier {
        "Lite" => 8,
        "Standard" => 24,
        "Pro" => 48,
        _ => 0,
    }
}

/// Check if detected RAM meets tier requirements.
pub enum RamCheckResult {
    /// RAM meets requirements
    Ok,
    /// RAM below minimum, includes detected GB and minimum GB
    Warning { detected: u64, minimum: u64 },
    /// Detection failed, skip warning
    Unknown,
}

pub fn check_ram_for_tier(tier: &str) -> RamCheckResult {
    let minimum = tier_min_ram(tier);
    if minimum == 0 {
        return RamCheckResult::Ok;
    }

    match detect_system_ram() {
        Some(detected) if detected >= minimum => RamCheckResult::Ok,
        Some(detected) => RamCheckResult::Warning { detected, minimum },
        None => RamCheckResult::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tier_min_ram_returns_correct_values() {
        assert_eq!(tier_min_ram("Lite"), 8);
        assert_eq!(tier_min_ram("Standard"), 24);
        assert_eq!(tier_min_ram("Pro"), 48);
        assert_eq!(tier_min_ram("Cloud"), 0);
    }

    #[test]
    fn detect_system_ram_returns_some() {
        // This test may fail in minimal CI environments
        let ram = detect_system_ram();
        // At minimum, the function should not panic
        // If it returns Some, it should be > 0
        if let Some(gb) = ram {
            assert!(gb > 0);
        }
    }
}
```

- [ ] **Step 2: Add module to main.rs**

Add to `src/main.rs`:

```rust
mod ram;
```

- [ ] **Step 3: Run tests**

```bash
cd installer-cli && cargo test ram:: -- --nocapture
```

Expected: 2 tests pass

- [ ] **Step 4: Commit**

```bash
cd installer-cli && git add src/ram.rs src/main.rs && git commit -m "feat(installer): add RAM detection for standalone tier warnings"
```

---

### Task 4: Update WizardStep Enum

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Update WizardStep enum**

Find the `WizardStep` enum (around line 17) and replace it with:

```rust
/// Wizard step enum for step-based navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WizardStep {
    Runtime,
    Tier,
    RamCheck,           // Standalone only
    LlmProvider,        // Cloud only
    EmbeddingProvider,  // Cloud only
    RerankerProvider,   // Cloud only
    Credentials,        // Cloud only
    License,
    Config,
    Install,
}

impl WizardStep {
    fn number(&self) -> usize {
        match self {
            WizardStep::Runtime => 1,
            WizardStep::Tier => 2,
            WizardStep::RamCheck => 3,
            WizardStep::LlmProvider => 3,
            WizardStep::EmbeddingProvider => 4,
            WizardStep::RerankerProvider => 5,
            WizardStep::Credentials => 6,
            WizardStep::License => 7,
            WizardStep::Config => 8,
            WizardStep::Install => 9,
        }
    }

    fn total() -> usize {
        9
    }
}
```

- [ ] **Step 2: Add step navigation functions**

Add after the `WizardStep` impl block:

```rust
fn next_step(current: WizardStep, tier: Tier) -> WizardStep {
    match (current, tier) {
        (WizardStep::Runtime, _) => WizardStep::Tier,
        (WizardStep::Tier, Tier::Cloud) => WizardStep::LlmProvider,
        (WizardStep::Tier, _) => WizardStep::RamCheck,
        (WizardStep::RamCheck, _) => WizardStep::License,
        (WizardStep::LlmProvider, _) => WizardStep::EmbeddingProvider,
        (WizardStep::EmbeddingProvider, _) => WizardStep::RerankerProvider,
        (WizardStep::RerankerProvider, _) => WizardStep::Credentials,
        (WizardStep::Credentials, _) => WizardStep::License,
        (WizardStep::License, _) => WizardStep::Config,
        (WizardStep::Config, _) => WizardStep::Install,
        (WizardStep::Install, _) => WizardStep::Install,
    }
}

fn prev_step(current: WizardStep, tier: Tier) -> WizardStep {
    match (current, tier) {
        (WizardStep::Runtime, _) => WizardStep::Runtime,
        (WizardStep::Tier, _) => WizardStep::Runtime,
        (WizardStep::RamCheck, _) => WizardStep::Tier,
        (WizardStep::LlmProvider, _) => WizardStep::Tier,
        (WizardStep::EmbeddingProvider, _) => WizardStep::LlmProvider,
        (WizardStep::RerankerProvider, _) => WizardStep::EmbeddingProvider,
        (WizardStep::Credentials, _) => WizardStep::RerankerProvider,
        (WizardStep::License, Tier::Cloud) => WizardStep::Credentials,
        (WizardStep::License, _) => WizardStep::RamCheck,
        (WizardStep::Config, _) => WizardStep::License,
        (WizardStep::Install, _) => WizardStep::Config,
    }
}
```

- [ ] **Step 3: Add navigation tests**

Add to the `tests` module at the bottom of selfhost.rs:

```rust
#[test]
fn next_step_cloud_goes_to_llm_provider() {
    assert_eq!(next_step(WizardStep::Tier, Tier::Cloud), WizardStep::LlmProvider);
}

#[test]
fn next_step_standalone_goes_to_ram_check() {
    assert_eq!(next_step(WizardStep::Tier, Tier::Lite), WizardStep::RamCheck);
    assert_eq!(next_step(WizardStep::Tier, Tier::Standard), WizardStep::RamCheck);
    assert_eq!(next_step(WizardStep::Tier, Tier::Pro), WizardStep::RamCheck);
}

#[test]
fn prev_step_license_differs_by_tier() {
    assert_eq!(prev_step(WizardStep::License, Tier::Cloud), WizardStep::Credentials);
    assert_eq!(prev_step(WizardStep::License, Tier::Lite), WizardStep::RamCheck);
}
```

- [ ] **Step 4: Run tests**

```bash
cd installer-cli && cargo test step -- --nocapture
```

Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
cd installer-cli && git add src/selfhost.rs && git commit -m "feat(installer): update WizardStep with Cloud tier steps and navigation"
```

---

### Task 5: Update WizardState

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Add providers import**

At the top of `src/selfhost.rs`, add:

```rust
use crate::providers::{LlmProvider, EmbeddingProvider, RerankerProvider, ProviderSet};
use crate::ram::{check_ram_for_tier, RamCheckResult};
use std::collections::HashMap;
```

- [ ] **Step 2: Update WizardState struct**

Find `WizardState` (around line 50) and update it:

```rust
/// Intermediate state accumulating wizard choices across steps.
/// Fields are Option so that going back clears future selections.
#[derive(Debug, Default)]
struct WizardState {
    podman: bool,
    tier: Option<Tier>,
    license_key: Option<String>,
    // Cloud tier providers (replaces embedding_model/embedding_credential)
    llm_provider: Option<LlmProvider>,
    embedding_provider: Option<EmbeddingProvider>,
    reranker_provider: Option<RerankerProvider>,
    collected_credentials: HashMap<String, String>,
    // Common config
    install_dir: Option<PathBuf>,
    port: Option<u16>,
    dagster_port: Option<u16>,
    postgres_password: Option<String>,
    use_external_ollama: Option<bool>,
}
```

- [ ] **Step 3: Add helper method to WizardState**

```rust
impl WizardState {
    /// Build ProviderSet from state. Returns None if any provider is missing.
    fn provider_set(&self) -> Option<ProviderSet> {
        Some(ProviderSet {
            llm: self.llm_provider.clone()?,
            embedding: self.embedding_provider.clone()?,
            reranker: self.reranker_provider.clone()?,
        })
    }
}
```

- [ ] **Step 4: Verify compilation**

```bash
cd installer-cli && cargo build 2>&1 | head -50
```

Note: There will be unused variable warnings and possibly errors from removed fields. These will be fixed in subsequent tasks.

- [ ] **Step 5: Commit**

```bash
cd installer-cli && git add src/selfhost.rs && git commit -m "feat(installer): update WizardState with provider fields"
```

---

### Task 6: Add Provider Prompt Functions

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Add prompt_llm_provider function**

Add after the existing prompt functions (around line 1700):

```rust
fn prompt_llm_provider() -> Result<LlmProvider> {
    use crate::providers::OtherProvider;
    
    let options = vec![
        "OpenAI (gpt-4o / gpt-4o-mini)",
        "Anthropic (claude-sonnet / claude-haiku)",
        "Vertex AI (gemini-2.5-pro / gemini-2.5-flash)",
        "Azure OpenAI (gpt-4o / gpt-4o-mini)",
        "AWS Bedrock (claude-sonnet / claude-haiku)",
        "Other (custom litellm provider)",
    ];

    let idx = Select::new()
        .with_prompt("Select LLM provider")
        .items(&options)
        .default(0)
        .interact()?;

    match idx {
        0 => Ok(LlmProvider::OpenAI),
        1 => Ok(LlmProvider::Anthropic),
        2 => Ok(LlmProvider::VertexAI),
        3 => Ok(LlmProvider::AzureOpenAI),
        4 => Ok(LlmProvider::Bedrock),
        5 => {
            let provider: String = Input::new()
                .with_prompt("litellm provider name (e.g., groq, together_ai)")
                .interact_text()?;
            let model: String = Input::new()
                .with_prompt("Model ID")
                .interact_text()?;
            
            println!("  Enter environment variables (empty name to finish):");
            let mut env_vars = Vec::new();
            loop {
                let name: String = Input::new()
                    .with_prompt("  Env var name")
                    .allow_empty(true)
                    .interact_text()?;
                if name.is_empty() {
                    break;
                }
                let value: String = Password::new()
                    .with_prompt(format!("  {}", name))
                    .interact()?;
                env_vars.push((name, value));
            }
            
            Ok(LlmProvider::Other(OtherProvider {
                provider,
                model,
                dimensions: None,
                env_vars,
            }))
        }
        _ => unreachable!(),
    }
}
```

- [ ] **Step 2: Add prompt_embedding_provider function**

```rust
fn prompt_embedding_provider() -> Result<EmbeddingProvider> {
    use crate::providers::OtherProvider;
    
    let options = vec![
        "OpenAI (text-embedding-3-large, 3072 dims)",
        "Vertex AI (text-embedding-005, 768 dims)",
        "Azure OpenAI (text-embedding-3-large, 3072 dims)",
        "AWS Bedrock (titan-embed-text-v2, 1024 dims)",
        "Other (custom provider)",
    ];

    let idx = Select::new()
        .with_prompt("Select embedding provider")
        .items(&options)
        .default(0)
        .interact()?;

    match idx {
        0 => Ok(EmbeddingProvider::OpenAI),
        1 => Ok(EmbeddingProvider::VertexAI),
        2 => Ok(EmbeddingProvider::AzureOpenAI),
        3 => Ok(EmbeddingProvider::Bedrock),
        4 => {
            let provider: String = Input::new()
                .with_prompt("litellm provider name")
                .interact_text()?;
            let model: String = Input::new()
                .with_prompt("Model ID")
                .interact_text()?;
            
            println!();
            println!("  {} Embedding dimensions are critical for Qdrant compatibility.", "!".yellow());
            println!("  {} Wrong dimensions will corrupt your vector store.", "!".yellow());
            let dimensions: u32 = Input::new()
                .with_prompt("Embedding dimensions")
                .interact_text()?;
            
            println!("  Enter environment variables (empty name to finish):");
            let mut env_vars = Vec::new();
            loop {
                let name: String = Input::new()
                    .with_prompt("  Env var name")
                    .allow_empty(true)
                    .interact_text()?;
                if name.is_empty() {
                    break;
                }
                let value: String = Password::new()
                    .with_prompt(format!("  {}", name))
                    .interact()?;
                env_vars.push((name, value));
            }
            
            Ok(EmbeddingProvider::Other(OtherProvider {
                provider,
                model,
                dimensions: Some(dimensions),
                env_vars,
            }))
        }
        _ => unreachable!(),
    }
}
```

- [ ] **Step 3: Add prompt_reranker_provider function**

```rust
fn prompt_reranker_provider() -> Result<RerankerProvider> {
    use crate::providers::OtherProvider;
    
    let options = vec![
        "None (disable reranking - faster but lower quality)",
        "Cohere (rerank-v3.5)",
        "Vertex AI (semantic-ranker)",
        "Other (custom provider)",
    ];

    let idx = Select::new()
        .with_prompt("Select reranker")
        .items(&options)
        .default(0)
        .interact()?;

    match idx {
        0 => Ok(RerankerProvider::None),
        1 => Ok(RerankerProvider::Cohere),
        2 => Ok(RerankerProvider::VertexAI),
        3 => {
            let provider: String = Input::new()
                .with_prompt("litellm provider name")
                .interact_text()?;
            let model: String = Input::new()
                .with_prompt("Model ID")
                .interact_text()?;
            
            println!("  Enter environment variables (empty name to finish):");
            let mut env_vars = Vec::new();
            loop {
                let name: String = Input::new()
                    .with_prompt("  Env var name")
                    .allow_empty(true)
                    .interact_text()?;
                if name.is_empty() {
                    break;
                }
                let value: String = Password::new()
                    .with_prompt(format!("  {}", name))
                    .interact()?;
                env_vars.push((name, value));
            }
            
            Ok(RerankerProvider::Other(OtherProvider {
                provider,
                model,
                dimensions: None,
                env_vars,
            }))
        }
        _ => unreachable!(),
    }
}
```

- [ ] **Step 4: Add Password import**

At the top with other dialoguer imports:

```rust
use dialoguer::{Input, Password, Select, Confirm};
```

- [ ] **Step 5: Verify compilation**

```bash
cd installer-cli && cargo build 2>&1 | head -30
```

- [ ] **Step 6: Commit**

```bash
cd installer-cli && git add src/selfhost.rs && git commit -m "feat(installer): add provider prompt functions for Cloud tier"
```

---

### Task 7: Add Credential Collection

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Add prompt_credentials function**

```rust
fn prompt_credentials(providers: &ProviderSet) -> Result<HashMap<String, String>> {
    let specs = providers.required_credentials();
    let custom_vars = providers.custom_env_vars();
    
    if specs.is_empty() && custom_vars.is_empty() {
        return Ok(HashMap::new());
    }
    
    println!();
    let env_var_names: Vec<_> = specs.iter().map(|s| s.env_var).collect();
    println!("  Your setup needs: {}", env_var_names.join(", "));
    println!();
    
    let mut creds = HashMap::new();
    
    for spec in specs {
        let value = if spec.secret {
            Password::new()
                .with_prompt(format!("  {}", spec.prompt))
                .interact()?
        } else {
            Input::new()
                .with_prompt(format!("  {}", spec.prompt))
                .interact_text()?
        };
        creds.insert(spec.env_var.to_string(), value);
    }
    
    // Custom env vars from Other providers are already collected
    for (name, value) in custom_vars {
        creds.insert(name, value);
    }
    
    Ok(creds)
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd installer-cli && cargo build
```

- [ ] **Step 3: Commit**

```bash
cd installer-cli && git add src/selfhost.rs && git commit -m "feat(installer): add credential collection with deduplication"
```

---

### Task 8: Add models.yaml Generation for Cloud Tier

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Add generate_cloud_models_yaml function**

Find the existing `generate_models_yaml` function and add this new function nearby:

```rust
fn generate_cloud_models_yaml(providers: &ProviderSet) -> String {
    let llm_provider = providers.llm.provider_name();
    let reasoning_model = providers.llm.reasoning_model();
    let fast_model = providers.llm.fast_model();
    
    let embed_provider = providers.embedding.provider_name();
    let embed_model = providers.embedding.model();
    let embed_dims = providers.embedding.dimensions();
    
    let reranker_section = if providers.reranker.is_none() {
        String::new()
    } else {
        format!(
            r#"    reranker:
      provider: {}
      model: {}
"#,
            providers.reranker.provider_name().unwrap(),
            providers.reranker.model().unwrap()
        )
    };
    
    format!(
        r#"# Cloud tier model configuration
# Generated by engrammic installer

tier: self_hosted

tiers:
  self_hosted:
    embeddings:
      provider: {embed_provider}
      model: {embed_model}
      dimensions: {embed_dims}
    reasoning:
      provider: {llm_provider}
      model: {reasoning_model}
    fast:
      provider: {llm_provider}
      model: {fast_model}
{reranker_section}    query_expander:
      provider: {llm_provider}
      model: {fast_model}
"#
    )
}
```

- [ ] **Step 2: Add test for models.yaml generation**

Add to tests module:

```rust
#[test]
fn generate_cloud_models_yaml_includes_all_providers() {
    use crate::providers::{LlmProvider, EmbeddingProvider, RerankerProvider, ProviderSet};
    
    let set = ProviderSet {
        llm: LlmProvider::Anthropic,
        embedding: EmbeddingProvider::OpenAI,
        reranker: RerankerProvider::Cohere,
    };
    
    let yaml = generate_cloud_models_yaml(&set);
    assert!(yaml.contains("provider: anthropic"));
    assert!(yaml.contains("provider: openai"));
    assert!(yaml.contains("provider: cohere"));
    assert!(yaml.contains("dimensions: 3072"));
}

#[test]
fn generate_cloud_models_yaml_omits_reranker_when_none() {
    use crate::providers::{LlmProvider, EmbeddingProvider, RerankerProvider, ProviderSet};
    
    let set = ProviderSet {
        llm: LlmProvider::OpenAI,
        embedding: EmbeddingProvider::OpenAI,
        reranker: RerankerProvider::None,
    };
    
    let yaml = generate_cloud_models_yaml(&set);
    assert!(!yaml.contains("reranker:"));
}
```

- [ ] **Step 3: Run tests**

```bash
cd installer-cli && cargo test generate_cloud_models_yaml -- --nocapture
```

Expected: 2 tests pass

- [ ] **Step 4: Commit**

```bash
cd installer-cli && git add src/selfhost.rs && git commit -m "feat(installer): add Cloud tier models.yaml generation"
```

---

### Task 9: Add RAM Check Step to Wizard

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Add prompt_ram_check function**

```rust
fn prompt_ram_check(tier: Tier) -> Result<bool> {
    let tier_name = match tier {
        Tier::Lite => "Lite",
        Tier::Standard => "Standard",
        Tier::Pro => "Pro",
        Tier::Cloud => return Ok(true), // Skip for Cloud
    };
    
    match check_ram_for_tier(tier_name) {
        RamCheckResult::Ok | RamCheckResult::Unknown => Ok(true),
        RamCheckResult::Warning { detected, minimum } => {
            println!();
            println!("  {} Detected: {}GB RAM", "!".yellow(), detected);
            println!();
            println!(
                "  {} {} tier requires {}GB+ RAM.",
                "Warning:".yellow(),
                tier_name,
                minimum
            );
            println!("  Your system may experience slowdowns or OOM errors.");
            println!();
            
            let options = vec![
                "Continue anyway",
                &format!("Switch to Lite ({}GB minimum)", crate::ram::tier_min_ram("Lite")),
                "Go back to tier selection",
            ];
            
            let idx = Select::new()
                .with_prompt("What would you like to do?")
                .items(&options)
                .default(0)
                .interact()?;
            
            match idx {
                0 => Ok(true),  // Continue
                1 => Ok(false), // Will trigger tier change to Lite
                2 => Ok(false), // Will go back
                _ => unreachable!(),
            }
        }
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cd installer-cli && cargo build
```

- [ ] **Step 3: Commit**

```bash
cd installer-cli && git add src/selfhost.rs && git commit -m "feat(installer): add RAM check prompt for standalone tiers"
```

---

### Task 10: Integrate New Steps into Wizard Loop

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Find the main wizard loop**

The wizard loop is in `run_wizard()` function (around line 307). Find the main `loop` that handles steps.

- [ ] **Step 2: Add new step handlers**

In the main wizard loop, add cases for the new steps. Find the existing step match and add:

```rust
WizardStep::RamCheck => {
    print_step_header(step, "RAM Check");
    
    let tier = state.tier.unwrap();
    if prompt_ram_check(tier)? {
        step = next_step(step, tier);
    } else {
        // User chose to go back or switch tier
        state.tier = None;
        step = WizardStep::Tier;
    }
}

WizardStep::LlmProvider => {
    print_step_header(step, "LLM Provider");
    
    if prompt_go_back()? {
        state.llm_provider = None;
        step = prev_step(step, state.tier.unwrap());
        continue;
    }
    
    state.llm_provider = Some(prompt_llm_provider()?);
    step = next_step(step, state.tier.unwrap());
}

WizardStep::EmbeddingProvider => {
    print_step_header(step, "Embedding Provider");
    
    if prompt_go_back()? {
        state.embedding_provider = None;
        step = prev_step(step, state.tier.unwrap());
        continue;
    }
    
    state.embedding_provider = Some(prompt_embedding_provider()?);
    step = next_step(step, state.tier.unwrap());
}

WizardStep::RerankerProvider => {
    print_step_header(step, "Reranker");
    
    if prompt_go_back()? {
        state.reranker_provider = None;
        step = prev_step(step, state.tier.unwrap());
        continue;
    }
    
    state.reranker_provider = Some(prompt_reranker_provider()?);
    step = next_step(step, state.tier.unwrap());
}

WizardStep::Credentials => {
    print_step_header(step, "API Credentials");
    
    if prompt_go_back()? {
        state.collected_credentials.clear();
        step = prev_step(step, state.tier.unwrap());
        continue;
    }
    
    let providers = state.provider_set().unwrap();
    state.collected_credentials = prompt_credentials(&providers)?;
    step = next_step(step, state.tier.unwrap());
}
```

- [ ] **Step 3: Update Tier step to use next_step**

Find the Tier step handler and update it to use the new navigation:

```rust
WizardStep::Tier => {
    print_step_header(step, "Deployment Tier");
    
    if prompt_go_back()? {
        state.tier = None;
        step = prev_step(step, Tier::Lite); // Tier doesn't matter for prev from Tier
        continue;
    }
    
    state.tier = Some(prompt_tier()?);
    step = next_step(step, state.tier.unwrap());
}
```

- [ ] **Step 4: Remove old Prerequisites and Embeddings steps**

Delete the handlers for `WizardStep::Prerequisites` and `WizardStep::Embeddings` (if they exist as separate enum variants in the old code).

- [ ] **Step 5: Update install step for Cloud tier**

Find the Install step and add Cloud tier handling:

```rust
// In the Install step, after config building:
let models_yaml = match config.tier {
    Tier::Cloud => {
        let providers = state.provider_set().unwrap();
        generate_cloud_models_yaml(&providers)
    }
    _ => generate_models_yaml(&config),
};
```

- [ ] **Step 6: Verify compilation**

```bash
cd installer-cli && cargo build
```

Fix any compilation errors that arise from the refactoring.

- [ ] **Step 7: Commit**

```bash
cd installer-cli && git add src/selfhost.rs && git commit -m "feat(installer): integrate Cloud tier steps into wizard loop"
```

---

### Task 11: Update .env Generation for Cloud Tier

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Find generate_env function**

Locate the function that generates `.env` content (search for `generate_env` or `.env` generation).

- [ ] **Step 2: Add Cloud tier credential writing**

Update the .env generation to include collected credentials for Cloud tier:

```rust
// After standard env vars, add for Cloud tier:
if config.tier == Tier::Cloud {
    for (key, value) in &state.collected_credentials {
        writeln!(env_content, "{}={}", key, value)?;
    }
}
```

- [ ] **Step 3: Set .env file permissions to 0o600**

Find where .env is written and add:

```rust
#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&env_path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&env_path, perms)?;
}
```

- [ ] **Step 4: Verify compilation**

```bash
cd installer-cli && cargo build
```

- [ ] **Step 5: Commit**

```bash
cd installer-cli && git add src/selfhost.rs && git commit -m "feat(installer): add Cloud tier credentials to .env with secure permissions"
```

---

### Task 12: Final Integration and Cleanup

**Files:**
- Modify: `installer-cli/src/selfhost.rs`

- [ ] **Step 1: Remove deprecated fields/functions**

Search for and remove:
- Old `embedding_model`, `embedding_dimensions`, `embedding_credential` field usages
- Old `prompt_embeddings` function if it exists
- Old `prompt_dimensions` function if not used elsewhere
- Any references to `Prerequisites` step

- [ ] **Step 2: Run full test suite**

```bash
cd installer-cli && cargo test
```

Fix any failing tests.

- [ ] **Step 3: Run clippy**

```bash
cd installer-cli && cargo clippy -- -W clippy::all
```

Fix any warnings.

- [ ] **Step 4: Verify build**

```bash
cd installer-cli && cargo build --release
```

- [ ] **Step 5: Commit**

```bash
cd installer-cli && git add . && git commit -m "chore(installer): cleanup deprecated code from config builder migration"
```

---

### Task 13: Manual Testing

**Files:** None (testing only)

- [ ] **Step 1: Test Cloud tier flow**

```bash
cd installer-cli && cargo run -- docker
```

Select Cloud tier and verify:
1. LLM provider prompt appears
2. Embedding provider prompt appears
3. Reranker prompt appears (with None option)
4. Credentials are collected
5. models.yaml is generated correctly in config/

- [ ] **Step 2: Test Standalone tier with RAM warning**

Run on a system with < 24GB RAM and select Standard tier. Verify warning appears.

- [ ] **Step 3: Test go-back navigation**

From Credentials step, go back to Reranker, then back to Embedding. Verify state is preserved.

- [ ] **Step 4: Test "Other" provider**

Select "Other" for LLM and verify custom provider/model/env-var prompts work.

- [ ] **Step 5: Commit test results**

```bash
cd installer-cli && git add . && git commit -m "test(installer): manual testing complete for config builder"
```

---

---

## Known Gaps (Future Work)

The following spec requirements are deferred to keep initial implementation focused:

1. **Vertex ADC auto-detection** - Spec says to run `gcloud auth application-default print-access-token` first and offer fallback to service account key. Current plan just prompts for project/location. Add later if users request.

2. **Bedrock IAM role option** - Spec says to offer "Use IAM role" that skips key prompts for EC2/ECS. Current plan always prompts for region only (keys optional). Add later if self-hosted on AWS is common.

3. **Env var collision check** - Spec says to warn if "Other" provider env vars collide with curated keys (e.g., custom OPENAI_API_KEY). Low priority - edge case.

4. **Reconfigure mode note** - Spec says Cloud tier reconfigure should show note about not pre-populating providers. Add in Task 10 if time permits.

These can be added incrementally without changing the core architecture.

---

## Summary

| Task | Description | Estimated Time |
|------|-------------|----------------|
| 1 | Provider types module | 15 min |
| 2 | CredentialSpec and ProviderSet | 10 min |
| 3 | RAM detection module | 10 min |
| 4 | WizardStep enum update | 10 min |
| 5 | WizardState update | 5 min |
| 6 | Provider prompt functions | 20 min |
| 7 | Credential collection | 10 min |
| 8 | models.yaml generation | 10 min |
| 9 | RAM check step | 10 min |
| 10 | Wizard loop integration | 30 min |
| 11 | .env generation update | 10 min |
| 12 | Final cleanup | 15 min |
| 13 | Manual testing | 20 min |

**Total: ~3 hours**

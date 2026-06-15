# Installer Config Builder Design

**Date:** 2026-06-15
**Status:** Approved (Rev 2 - addressed review findings)
**Scope:** Cloud tier component selection, RAM detection for standalone

## Problem

The current installer has a rigid tier-based model:
- Standalone tiers (Lite/Standard/Pro) work well for local setups
- Cloud tier only asks about embedding provider, not LLM or reranker
- No way to use cloud LLM + cloud embeddings + cloud reranker (Cohere)
- No RAM detection to warn users about tier requirements

## Solution

Extend the wizard with component selection for Cloud tier, add RAM detection for standalone tiers.

## Wizard Flow

### Standalone (Lite/Standard/Pro)

```
Runtime -> Tier -> RAM Check -> License -> Config -> Install
```

RAM check detects system memory and warns if selected tier exceeds available RAM.

### Cloud

```
Runtime -> Tier -> LLM Provider -> Embedding Provider -> Reranker -> Credentials -> License -> Config -> Install
```

Component selection with curated provider list + "Other" escape hatch.

### Step Navigation Logic

```rust
fn next_step(current: WizardStep, tier: Tier) -> WizardStep {
    match (current, tier) {
        (WizardStep::Runtime, _) => WizardStep::Tier,
        (WizardStep::Tier, Tier::Cloud) => WizardStep::LlmProvider,
        (WizardStep::Tier, _) => WizardStep::RamCheck,  // Standalone tiers
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

### Removed/Replaced Steps

| Old Step | New Step | Reason |
|----------|----------|--------|
| `Prerequisites` | Removed | Ollama/TEI checks now happen at Install based on tier |
| `Embeddings` | `EmbeddingProvider` (Cloud) or removed (Standalone) | Standalone uses bundled TEI, no prompt needed |

## Provider Support

### Curated Providers

| Provider | LLM | Embeddings | Reranker | Required Credentials |
|----------|-----|------------|----------|---------------------|
| OpenAI | Y | Y | - | `OPENAI_API_KEY` |
| Anthropic | Y | - | - | `ANTHROPIC_API_KEY` |
| Vertex AI | Y | Y | Y | `VERTEX_PROJECT`, `VERTEX_LOCATION`, ADC or SA key |
| Azure OpenAI | Y | Y | - | `AZURE_API_KEY`, `AZURE_API_BASE`, `AZURE_API_VERSION` |
| Bedrock | Y | Y | - | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION` (or IAM role) |
| Cohere | - | - | Y | `COHERE_API_KEY` |

### Default Models

| Provider | Reasoning | Fast | Embedding | Embedding Dims | Reranker |
|----------|-----------|------|-----------|----------------|----------|
| OpenAI | gpt-4o | gpt-4o-mini | text-embedding-3-large | 3072 | - |
| Anthropic | claude-sonnet-4-5 | claude-haiku-4-5 | - | - | - |
| Vertex | gemini-2.5-pro | gemini-2.5-flash | text-embedding-005 | 768 | semantic-ranker-default@latest |
| Azure | gpt-4o | gpt-4o-mini | text-embedding-3-large | 3072 | - |
| Bedrock | anthropic.claude-sonnet | anthropic.claude-haiku | amazon.titan-embed-text-v2 | 1024 | - |
| Cohere | - | - | - | - | rerank-v3.5 |

### "Other" Provider

For unlisted providers, prompt for:
1. litellm provider name (e.g., `groq`, `together_ai`)
2. Model ID
3. **For embeddings only:** Embedding dimensions (with warning about Qdrant compatibility)
4. Required env vars (name=value pairs)

**Env var collision check:** If user enters an env var that matches a curated provider key (e.g., `OPENAI_API_KEY`), warn and require confirmation.

**Embedding dimension warning:**
```
Embedding dimensions are critical for Qdrant compatibility.
Wrong dimensions will corrupt your vector store.
Enter embedding dimensions for your model: ____
```

### Query Expander Handling

`query_expander` is not prompted separately. It copies from the LLM provider's `fast` model automatically:

```rust
fn query_expander_config(llm: &LlmProvider) -> (String, String) {
    // Returns (provider_name, fast_model)
    llm.fast_model_config()
}
```

## Credential Collection

After component selection, deduplicate required credentials across all three components:

```rust
struct ProviderSet {
    llm: LlmProvider,
    embedding: EmbeddingProvider,
    reranker: RerankerProvider,
}

fn required_credentials(providers: &ProviderSet) -> Vec<CredentialSpec> {
    let mut creds = Vec::new();
    creds.extend(providers.llm.required_credentials());
    creds.extend(providers.embedding.required_credentials());
    creds.extend(providers.reranker.required_credentials());
    // Deduplicate by env var name
    creds.sort_by(|a, b| a.env_var.cmp(&b.env_var));
    creds.dedup_by(|a, b| a.env_var == b.env_var);
    creds
}

struct CredentialSpec {
    env_var: &'static str,
    prompt: &'static str,
    secret: bool,  // mask input
}
```

### Vertex AI ADC Handling

```
Checking Vertex AI authentication...

[1] Found: Application Default Credentials via gcloud
    Project: my-project, Location: us-central1
    [Use these]  [Enter different project/location]  [Use service account key]

[2] Not found: gcloud auth application-default login not configured
    [Configure now (opens browser)]  [Use service account key]
```

If service account key selected, prompt for JSON path and set `GOOGLE_APPLICATION_CREDENTIALS`.

### Bedrock IAM Role Handling

```
AWS credentials for Bedrock:

[1] Enter access key (recommended for local dev)
[2] Use IAM role (for EC2/ECS with attached role)

If using IAM role, ensure the instance has bedrock:InvokeModel permissions.
```

If IAM role selected, skip `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` prompts, only collect `AWS_REGION`.

## RAM Detection

### Detection Method

```rust
fn detect_system_ram() -> Option<u64> {
    #[cfg(target_os = "linux")]
    { /* parse /proc/meminfo for MemTotal */ }
    
    #[cfg(target_os = "macos")]
    { /* sysctl -n hw.memsize */ }
    
    #[cfg(target_os = "windows")]
    { /* GlobalMemoryStatusEx */ }
    
    // Returns RAM in GB, None if detection fails
}
```

### RAM Thresholds (exact values)

| Tier | Minimum GB | Warning Threshold |
|------|------------|-------------------|
| Lite | 8 | `detected < 8` |
| Standard | 24 | `detected < 24` |
| Pro | 48 | `detected < 48` |

### Behavior

If detected RAM < tier minimum:

```
Detected: 16GB RAM

Warning: Standard tier requires 24GB+ RAM.
Your system may experience slowdowns or OOM errors.

[1] Continue anyway
[2] Switch to Lite (8GB minimum)
[3] Go back to tier selection
```

If detection fails (returns `None`), skip warning and proceed.

## Reranker "None" Option

The reranker prompt explicitly includes "None" as the first option:

```
Select reranker provider:

[1] None (disable reranking - faster but lower quality)
[2] Cohere (rerank-v3.5)
[3] Vertex AI (semantic-ranker)
[4] Other (custom provider)
```

When "None" is selected:
- `RerankerProvider::None` stored in state
- No reranker credentials collected
- Generated `models.yaml` omits the `reranker:` section entirely
- App falls back to non-reranked retrieval

## Generated Files

### models.yaml (Cloud tier)

Written to `{install_dir}/config/models.yaml`:

```yaml
tier: self_hosted

tiers:
  self_hosted:
    embeddings:
      provider: openai
      model: text-embedding-3-large
      dimensions: 3072
    reasoning:
      provider: anthropic
      model: claude-sonnet-4-5
    fast:
      provider: anthropic
      model: claude-haiku-4-5
    # reranker omitted if None selected
    reranker:
      provider: cohere
      model: rerank-v3.5
    query_expander:
      provider: anthropic      # copied from LLM provider
      model: claude-haiku-4-5  # copied from LLM fast model
```

### .env (Cloud tier)

Written to `{install_dir}/.env` with **0o600 permissions** (user read/write only):

```bash
ENGRAMMIC_LICENSE_KEY=...
POSTGRES_PASSWORD=...
ENGRAMMIC_CONFIG_DIR=/app/config-override

# Collected credentials (only those needed)
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
COHERE_API_KEY=...
# GOOGLE_APPLICATION_CREDENTIALS=... (if SA key used)
```

### docker-compose.yml

Cloud tier uses existing `COMPOSE_TEMPLATE` (no Ollama/TEI containers).

## Data Structures

### Provider Config (shared structure)

```rust
struct ProviderConfig {
    provider: String,      // litellm provider name
    model: String,         // model ID
    dimensions: Option<u32>, // only for embeddings
}

impl LlmProvider {
    fn to_config(&self) -> ProviderConfig { ... }
    fn reasoning_model(&self) -> &str { ... }
    fn fast_model(&self) -> &str { ... }
    fn required_credentials(&self) -> Vec<CredentialSpec> { ... }
}
// Similar impls for EmbeddingProvider, RerankerProvider
```

### Provider Enums

```rust
#[derive(Debug, Clone)]
enum LlmProvider {
    OpenAI,
    Anthropic,
    VertexAI,
    AzureOpenAI,
    Bedrock,
    Other(OtherProvider),
}

#[derive(Debug, Clone)]
enum EmbeddingProvider {
    OpenAI,
    VertexAI,
    AzureOpenAI,
    Bedrock,
    Other(OtherProvider),
}

#[derive(Debug, Clone)]
enum RerankerProvider {
    None,
    Cohere,
    VertexAI,
    Other(OtherProvider),
}

#[derive(Debug, Clone)]
struct OtherProvider {
    provider: String,
    model: String,
    dimensions: Option<u32>,  // only for embeddings
    env_vars: Vec<(String, String)>,
}
```

### Enum to YAML String Conversion

```rust
impl LlmProvider {
    fn provider_name(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "openai",
            LlmProvider::Anthropic => "anthropic",
            LlmProvider::VertexAI => "vertex_ai",
            LlmProvider::AzureOpenAI => "azure",
            LlmProvider::Bedrock => "bedrock",
            LlmProvider::Other(o) => &o.provider,
        }
    }
}
// Similar for EmbeddingProvider, RerankerProvider
```

### WizardStep Enum

```rust
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
```

### WizardState (field changes)

```rust
#[derive(Debug, Default)]
struct WizardState {
    podman: bool,
    tier: Option<Tier>,
    license_key: Option<String>,
    
    // REMOVED: embedding_model, embedding_dimensions, embedding_credential
    // REPLACED BY:
    llm_provider: Option<LlmProvider>,
    embedding_provider: Option<EmbeddingProvider>,
    reranker_provider: Option<RerankerProvider>,
    collected_credentials: HashMap<String, String>,
    
    install_dir: Option<PathBuf>,
    port: Option<u16>,
    dagster_port: Option<u16>,
    postgres_password: Option<String>,
    use_external_ollama: Option<bool>,
}
```

### ProviderSet (for credential collection)

```rust
struct ProviderSet {
    llm: LlmProvider,
    embedding: EmbeddingProvider,
    reranker: RerankerProvider,
}

impl ProviderSet {
    fn from_state(state: &WizardState) -> Option<Self> {
        Some(ProviderSet {
            llm: state.llm_provider.clone()?,
            embedding: state.embedding_provider.clone()?,
            reranker: state.reranker_provider.clone()?,
        })
    }
}
```

## Reconfigure Mode

### Standalone Tiers

Reconfigure works as before - pre-populate from existing `.env`:
- `license_key`, `postgres_password`, `port`, `dagster_port`

### Cloud Tier

For Cloud tier reconfigure, **do not pre-populate provider selections** from existing `models.yaml`. Reasons:
1. Parsing YAML to reconstruct enum variants is error-prone
2. Provider defaults may have changed between versions
3. Users can see current config in `models.yaml` and re-enter if needed

Behavior:
```
Existing Cloud installation found.

Note: Provider selections will not be pre-filled.
Your current config is in config/models.yaml if you need to reference it.

[Continue with reconfigure]  [Cancel]
```

Future enhancement: Add `models.yaml` parsing to pre-populate provider selections.

## New Functions

```rust
// RAM detection
fn detect_system_ram() -> Option<u64>
fn check_ram_for_tier(tier: Tier, detected_ram: Option<u64>) -> RamCheckResult

// Provider prompts
fn prompt_llm_provider() -> Result<LlmProvider>
fn prompt_embedding_provider() -> Result<EmbeddingProvider>
fn prompt_reranker_provider() -> Result<RerankerProvider>
fn prompt_other_provider(component: &str) -> Result<OtherProvider>

// Credential handling
fn prompt_credentials(providers: &ProviderSet) -> Result<HashMap<String, String>>
fn required_credentials(providers: &ProviderSet) -> Vec<CredentialSpec>
fn check_vertex_adc() -> Result<Option<(String, String)>>  // (project, location)
fn prompt_bedrock_auth() -> Result<BedrockAuth>

// Generation
fn generate_cloud_models_yaml(providers: &ProviderSet) -> String
fn generate_cloud_env(providers: &ProviderSet, creds: &HashMap<String, String>) -> String

// Step navigation
fn next_step(current: WizardStep, tier: Tier) -> WizardStep
fn prev_step(current: WizardStep, tier: Tier) -> WizardStep
```

## File Changes

| File | Change |
|------|--------|
| `selfhost.rs` | New enums, wizard steps, provider prompts, RAM detection, step navigation |
| `docker.rs` | No change |
| `assets/models.yaml` | No change (used for standalone, Cloud generates dynamically) |

## Out of Scope

- Hybrid configs (cloud LLM + local TEI)
- New standalone presets (Medium tier for 16GB)
- Per-model selection within providers (use defaults)
- Cloud tier reconfigure pre-population from models.yaml

## Testing

1. Cloud: OpenAI LLM + Vertex embeddings + Cohere reranker
2. Cloud: Anthropic LLM + OpenAI embeddings + None reranker
3. Cloud: "Other" provider for all three components
4. Cloud: "Other" with env var collision (should warn)
5. Cloud: Vertex with ADC present
6. Cloud: Vertex with service account key fallback
7. Cloud: Bedrock with IAM role (skip key prompts)
8. Standalone Lite on 6GB system (warning shown)
9. Standalone Standard on 32GB system (no warning)
10. Standalone Pro on 48GB system (no warning, borderline)
11. RAM detection failure (warning skipped)
12. Go-back navigation: LlmProvider -> Tier
13. Go-back navigation: Credentials -> RerankerProvider
14. Reconfigure Cloud tier (no pre-population, shows note)
15. Verify .env has 0o600 permissions

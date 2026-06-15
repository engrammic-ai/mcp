# Installer Config Builder Design

**Date:** 2026-06-15
**Status:** Approved
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

## Provider Support

### Curated Providers

| Provider | LLM | Embeddings | Reranker | Required Credentials |
|----------|-----|------------|----------|---------------------|
| OpenAI | Y | Y | - | `OPENAI_API_KEY` |
| Anthropic | Y | - | - | `ANTHROPIC_API_KEY` |
| Vertex AI | Y | Y | Y | `VERTEX_PROJECT`, `VERTEX_LOCATION`, ADC |
| Azure OpenAI | Y | Y | - | `AZURE_API_KEY`, `AZURE_API_BASE`, `AZURE_API_VERSION` |
| Bedrock | Y | Y | - | `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION` |
| Cohere | - | - | Y | `COHERE_API_KEY` |

### Default Models

| Provider | Reasoning | Fast | Embedding | Reranker |
|----------|-----------|------|-----------|----------|
| OpenAI | gpt-4o | gpt-4o-mini | text-embedding-3-large | - |
| Anthropic | claude-sonnet-4-5 | claude-haiku-4-5 | - | - |
| Vertex | gemini-2.5-pro | gemini-2.5-flash | text-embedding-005 | semantic-ranker |
| Azure | gpt-4o | gpt-4o-mini | text-embedding-3-large | - |
| Bedrock | anthropic.claude-sonnet | anthropic.claude-haiku | amazon.titan-embed-text-v2 | - |
| Cohere | - | - | - | rerank-v3.5 |

### "Other" Provider

For unlisted providers, prompt for:
1. litellm provider name (e.g., `groq`, `together_ai`)
2. Model ID
3. Required env vars (name=value pairs)

## Credential Collection

After component selection, deduplicate required credentials:

```
Your setup needs: OPENAI_API_KEY, COHERE_API_KEY

Enter OPENAI_API_KEY: ********
Enter COHERE_API_KEY: ********
```

For Vertex AI, check `gcloud auth application-default print-access-token` first.

## RAM Detection

### Detection Method

- Linux: `/proc/meminfo`
- macOS: `sysctl hw.memsize`
- Windows: `GlobalMemoryStatusEx`

### Requirements

| Tier | Min RAM | Recommended |
|------|---------|-------------|
| Lite | 8GB | 12GB |
| Standard | 24GB | 32GB |
| Pro | 48GB | 64GB |

### Behavior

If detected RAM < tier minimum:

```
Detected: 16GB RAM

Warning: Standard tier recommends 24-32GB RAM.
Your system may experience slowdowns or OOM errors.

[Continue anyway]  [Switch to Lite]  [Go back]
```

If detection fails, skip warning silently.

## Generated Files

### models.yaml (Cloud tier)

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
    reranker:
      provider: cohere
      model: rerank-v3.5
    query_expander:
      provider: anthropic
      model: claude-haiku-4-5
```

If reranker is None, omit the reranker section entirely.

### .env (Cloud tier)

```bash
ENGRAMMIC_LICENSE_KEY=...
POSTGRES_PASSWORD=...

# Collected credentials (only those needed)
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
COHERE_API_KEY=...
```

### docker-compose.yml

Cloud tier uses existing `COMPOSE_TEMPLATE` (no Ollama/TEI containers).

## Data Structures

### New Enums

```rust
enum LlmProvider {
    OpenAI,
    Anthropic,
    VertexAI,
    AzureOpenAI,
    Bedrock,
    Other { provider: String, model: String, env_vars: Vec<(String, String)> },
}

enum EmbeddingProvider {
    OpenAI,
    VertexAI,
    AzureOpenAI,
    Bedrock,
    Other { provider: String, model: String, env_vars: Vec<(String, String)> },
}

enum RerankerProvider {
    Cohere,
    VertexAI,
    None,
    Other { provider: String, model: String, env_vars: Vec<(String, String)> },
}
```

### Extended WizardStep

```rust
enum WizardStep {
    Runtime,
    Tier,
    RamCheck,           // NEW: after Tier for Standalone
    LlmProvider,        // NEW: after Tier for Cloud
    EmbeddingProvider,  // NEW
    RerankerProvider,   // NEW
    Credentials,        // NEW: collect API keys
    License,
    Config,
    Install,
}
```

### Extended WizardState

```rust
struct WizardState {
    // existing fields...
    llm_provider: Option<LlmProvider>,
    embedding_provider: Option<EmbeddingProvider>,
    reranker_provider: Option<RerankerProvider>,
    collected_credentials: HashMap<String, String>,
}
```

## New Functions

```rust
fn detect_system_ram() -> Option<u64>
fn prompt_llm_provider() -> Result<LlmProvider>
fn prompt_embedding_provider() -> Result<EmbeddingProvider>
fn prompt_reranker_provider() -> Result<RerankerProvider>
fn prompt_credentials(providers: &ProviderSet) -> Result<HashMap<String, String>>
fn required_credentials(providers: &ProviderSet) -> Vec<&'static str>
fn generate_cloud_models_yaml(config: &SelfHostConfig) -> String
```

## File Changes

| File | Change |
|------|--------|
| `selfhost.rs` | New enums, wizard steps, prompt functions, RAM detection |
| `docker.rs` | No change |
| `assets/models.yaml` | No change |

## Out of Scope

- Hybrid configs (cloud LLM + local TEI)
- New standalone presets (Medium tier for 16GB)
- Per-model selection within providers (use defaults)

## Testing

1. Cloud tier with OpenAI LLM + Vertex embeddings + Cohere reranker
2. Cloud tier with "Other" provider
3. Cloud tier with reranker = None
4. Standalone Lite on 6GB system (warning shown)
5. Standalone Standard on 32GB system (no warning)
6. RAM detection failure (warning skipped)
7. Go-back navigation from LLM step to Tier

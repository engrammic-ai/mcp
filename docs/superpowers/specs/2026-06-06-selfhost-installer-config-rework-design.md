# Self-Host Installer Config Rework

Status: Draft
Author: Claude
Date: 2026-06-06
Updated: 2026-06-06

## Problem

The `engrammic selfhost` wizard (`installer-cli/src/selfhost.rs`) generates a
`.env` that does not match what the self-hosted image actually reads:

- `generate_env` writes `TELEMETRY_ENABLED`, but the service reads
  `TELEMETRY__ENABLED`. The flat form is ignored.
- It writes `LLM_PROVIDER`, `LLM_API_KEY`, and `LLM_MODEL`. None are read. The
  SAGE provider/model come from `config/models.yaml`, credentialed by
  per-provider keys (`OPENAI_API_KEY`, etc.).
- It never writes `EMBEDDING_MODEL` or any embedding credential. Embeddings are
  required; without them `remember`/`learn`/`recall` fail silently.

Net effect: wizard's LLM step is wired to dead variables, and the one required
piece (embeddings) is never configured.

## Design principles

Self-hosted must be **platform and infra agnostic**. No provider lock-in, no
opinionated defaults. The installer exposes all config knobs; users configure
what they have.

- **Hyperconfigurable**: every knob documented and overridable
- **Minimal wizard prompts**: only what's required to boot (embeddings)
- **No tiering**: self-hosted uses a single flat model config, not
  economy/balanced/premium tiers
- **Same config story**: compose and standalone image use identical env vars and
  config mounts

## Already done (context-service + compose)

- Host config override: `paths.resolve_config_file(filename, default)` prefers
  `$ENGRAMMIC_CONFIG_DIR/<filename>` when present, else baked-in default.
- Both compose files mount `./config` to `/app/config-override` (read-only) and
  set `ENGRAMMIC_CONFIG_DIR` on all services.

## Config surface (source of truth)

### Infrastructure

| Env var | Description | Default |
|---------|-------------|---------|
| `QDRANT_HOST` | Qdrant server host | `qdrant` (compose service name) |
| `QDRANT_PORT` | Qdrant server port | `6333` |
| `QDRANT_API_KEY` | API key for remote/cloud Qdrant | (none) |
| `REDIS_URL` | Redis connection string | `redis://redis:6379` |
| `MEMGRAPH_HOST` | Memgraph server host | `memgraph` |
| `MEMGRAPH_PORT` | Memgraph bolt port | `7687` |
| `ENGRAMMIC_CONFIG_DIR` | Config override directory | `/app/config-override` |

### Embeddings (required)

| Env var | Description |
|---------|-------------|
| `EMBEDDING_MODEL` | LiteLLM model string (e.g. `openai/text-embedding-3-small`, `ollama/nomic-embed-text`) |
| `EMBEDDING_DIMENSIONS` | Must match model: 1536 (openai-3-small), 768 (nomic, vertex-005), 384 (MiniLM) |

Credentials by prefix:
- OpenAI: `OPENAI_API_KEY`
- Azure: `AZURE_API_KEY`, `AZURE_API_BASE`, `AZURE_API_VERSION`, `AZURE_DEPLOYMENT`
- AWS Bedrock: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_REGION_NAME`
- Vertex: `VERTEX_PROJECT`, `VERTEX_LOCATION` (requires gcloud credentials mounted at `~/.config/gcloud` or ADC)
- Ollama: `OLLAMA_API_BASE`

### Reranking (optional, improves recall quality)

| Env var | Description |
|---------|-------------|
| `RERANKING__ENABLED` | `true`/`false` (default: `false` to avoid boot failures without credentials) |
| `RERANKING__PROVIDER` | `cohere`, `vertex`, `cross_encoder` |
| `RERANKING__MODEL` | Provider-specific model ID |

Credentials by provider:
- Cohere: `COHERE_API_KEY`
- Vertex: `VERTEX_PROJECT`, `VERTEX_LOCATION`
- Cross-encoder: `RERANKING__ENDPOINT` (custom endpoint URL)

### LLM (optional, enables SAGE synthesis)

Configured via `config/models.yaml` override. Self-hosted uses a single flat
structure (no tiers):

```yaml
default_tier: self_hosted

tiers:
  self_hosted:
    reasoning:
      provider: openai  # or anthropic, ollama, vertex, etc.
      model: gpt-4o
    fast:
      provider: openai
      model: gpt-4o-mini
    query_expander:
      provider: openai
      model: gpt-4o-mini
```

Credentials by provider:
- OpenAI: `OPENAI_API_KEY`
- Anthropic: `ANTHROPIC_API_KEY`
- Gemini: `GEMINI_API_KEY`
- Vertex: `VERTEX_PROJECT`, `VERTEX_LOCATION`
- Ollama: `OLLAMA_BASE_URL` (note: distinct from embeddings' `OLLAMA_API_BASE`)

### Other

| Env var | Description |
|---------|-------------|
| `TELEMETRY__ENABLED` | `true`/`false` (double underscore) |

## Wizard changes (`selfhost.rs`)

### Minimal prompts

The wizard only prompts for what's required to boot:

1. **Embedding model** (required): prompt for model string, auto-fill dimensions
   for known models, prompt for dimensions if unknown (with warning about Qdrant
   collection corruption on mismatch)
2. **Embedding credential**: prompt for the relevant API key based on model prefix

Everything else: generate a well-commented `.env` template with all knobs
documented inline. User edits as needed.

### Generated files

1. **`.env`**: all config knobs with inline comments explaining each. Only
   embeddings filled in from prompts; rest are commented examples.

2. **`config/models.yaml`**: flat self-hosted structure (no tiers). Generated
   as a template with placeholder values, commented out by default.

3. **`README.md`**: brief getting-started, points to docs for full config
   reference.

### Implementation

```rust
fn generate_env(config: &Config) -> String {
    // Required (filled from wizard)
    format!(r#"
# =============================================================================
# EMBEDDINGS (required)
# =============================================================================
EMBEDDING_MODEL={embedding_model}
EMBEDDING_DIMENSIONS={embedding_dimensions}
{embedding_credential}

# =============================================================================
# INFRASTRUCTURE (defaults work with bundled compose)
# =============================================================================
# QDRANT_HOST=qdrant
# QDRANT_PORT=6333
# QDRANT_API_KEY=
# REDIS_URL=redis://redis:6379
# MEMGRAPH_HOST=memgraph
# MEMGRAPH_PORT=7687
ENGRAMMIC_CONFIG_DIR=/app/config-override

# =============================================================================
# RERANKING (optional, improves recall quality)
# See: https://docs.engrammic.ai/self-hosted/reranking
# =============================================================================
# RERANKING__ENABLED=true
# RERANKING__PROVIDER=cohere
# RERANKING__MODEL=rerank-english-v3.0
# COHERE_API_KEY=your-key

# =============================================================================
# LLM CREDENTIALS (for SAGE synthesis, configure models in config/models.yaml)
# =============================================================================
# OPENAI_API_KEY=your-key
# ANTHROPIC_API_KEY=your-key
# GEMINI_API_KEY=your-key
# OLLAMA_BASE_URL=http://localhost:11434

# =============================================================================
# TELEMETRY
# =============================================================================
TELEMETRY__ENABLED=false
"#, ...)
}
```

### Remove dead code

Drop from `generate_env`:
- `LLM_PROVIDER`
- `LLM_API_KEY`
- `LLM_MODEL`
- `TELEMETRY_ENABLED` (flat form)

### Embedding dimensions handling

Known models auto-fill:
- `openai/text-embedding-3-small` -> 1536
- `openai/text-embedding-3-large` -> 3072
- `ollama/nomic-embed-text` -> 768
- `vertex_ai/text-embedding-005` -> 768
- `ollama/all-minilm` -> 384

Unknown models: prompt for dimensions with warning:
```
Unknown embedding model. Enter dimensions manually.
WARNING: Wrong dimensions will corrupt your Qdrant collection.
         Fixing requires wiping the collection and re-embedding all data.
Dimensions:
```

### models.yaml generation

Bundle `models.yaml` template as installer asset. CI check validates that the
bundled template's schema (required keys: `default_tier`, `tiers`, tier
structure with `reasoning`/`fast`/`query_expander`) matches the service's
`models.yaml`. Content (model names, providers) can differ. Hard failure on
schema mismatch.

Generated template uses flat self-hosted structure:
```yaml
# Self-hosted model configuration
# See: https://docs.engrammic.ai/self-hosted/models

default_tier: self_hosted

tiers:
  self_hosted:
    # Uncomment and configure your preferred provider
    # reasoning:
    #   provider: openai
    #   model: gpt-4o
    # fast:
    #   provider: openai
    #   model: gpt-4o-mini
    # query_expander:
    #   provider: openai
    #   model: gpt-4o-mini
```

### File permissions

- `create_dir_all(install_dir/config)` so mount source exists
- Write files as mode `644` so container's uid-1000 can read

## Documentation (../web/docs)

Add self-hosted config reference:

1. **`/self-hosted/configuration.md`**: full env var reference, all knobs
2. **`/self-hosted/reranking.md`**: reranker setup (Cohere, Vertex,
   cross-encoder, disabling), quality vs cost trade-offs
3. **`/self-hosted/models.md`**: LLM configuration, models.yaml structure,
   provider-specific setup
4. **`/self-hosted/examples.md`**: complete example configs:
   - Ollama-only (fully local)
   - OpenAI everything
   - Mixed (local embeddings + cloud LLM)
   - Vertex (GCP-native)

## Edge cases

- **Dimension change on re-run**: warn that changing embedding model after first
  boot requires wiping Qdrant collection and re-embedding
- **Idempotency**: if `.env` or `config/models.yaml` exists, prompt: "Config
  files exist. Overwrite? [y/N]". On 'n', skip file generation and print path
  for manual editing. On 'y', regenerate from wizard answers.
- **Ollama**: write `OLLAMA_API_BASE` for embeddings, `OLLAMA_BASE_URL` for LLM
  (distinct vars)

## Standalone image

Same config surface as compose. Pass env vars via `docker run -e` or
`--env-file`. Mount config override dir:

```bash
docker run -d \
  --env-file .env \
  -v ./config:/app/config-override:ro \
  -e ENGRAMMIC_CONFIG_DIR=/app/config-override \
  europe-north1-docker.pkg.dev/engrammic/releases/engrammic-api:latest
```

The generated `.env` and `config/` work for both compose and standalone.

## Out of scope

- Model weight downloads, ONNX runtime, air-gapped setup (separate track in
  `context-service/context/specs/local-model-setup.md`)
- Changes to service config resolution beyond what already landed
- Tiering for self-hosted (explicitly removed)

## Verification

- `cargo build` and `cargo test` pass in `installer-cli`
- Generated `.env` contains only live vars (no dead `LLM_*` vars, correct
  `TELEMETRY__ENABLED`)
- Generated `config/models.yaml` uses flat self-hosted structure
- Files created with mode `644`
- End-to-end: OpenAI embeddings + disabled reranking boots and
  `recall`/`remember` work
- Docs build and render correctly

## Resolved

- No tiering for self-hosted; single flat model config
- Reranker is configurable via env vars, documented in docs
- Wizard prompts only for embeddings; everything else via commented template
- Platform/infra agnostic: no provider lock-in

/// Custom provider configuration for unlisted providers.
#[derive(Debug, Clone)]
pub struct OtherProvider {
    pub provider: String,
    pub model: String,
    pub dimensions: Option<u32>, // Only for embeddings
    pub env_vars: Vec<(String, String)>,
}

/// Specification for a credential that needs to be collected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CredentialSpec {
    pub env_var: &'static str,
    pub prompt: &'static str,
    pub secret: bool,
}

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
            LlmProvider::Other(_) => vec![],
        }
    }

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

/// Reranker provider selection for Cloud tier.
#[derive(Debug, Clone)]
pub enum RerankerProvider {
    LocalTei,  // Bundled TEI reranker (bge-reranker-v2-m3)
    None,
    Cohere,
    VertexAI,
    Other(OtherProvider),
}

impl RerankerProvider {
    pub fn provider_name(&self) -> Option<&str> {
        match self {
            RerankerProvider::LocalTei => Some("tei"),
            RerankerProvider::None => Option::None,
            RerankerProvider::Cohere => Some("cohere"),
            RerankerProvider::VertexAI => Some("vertex_ai"),
            RerankerProvider::Other(o) => Some(&o.provider),
        }
    }

    pub fn model(&self) -> Option<&str> {
        match self {
            RerankerProvider::LocalTei => Some("BAAI/bge-reranker-v2-m3"),
            RerankerProvider::None => Option::None,
            RerankerProvider::Cohere => Some("rerank-v3.5"),
            RerankerProvider::VertexAI => Some("semantic-ranker-default@latest"),
            RerankerProvider::Other(o) => Some(&o.model),
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, RerankerProvider::None)
    }

    pub fn is_local(&self) -> bool {
        matches!(self, RerankerProvider::LocalTei)
    }

    pub fn required_credentials(&self) -> Vec<CredentialSpec> {
        match self {
            RerankerProvider::LocalTei => vec![], // No credentials needed
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

    #[test]
    fn embedding_provider_has_dimensions() {
        assert_eq!(EmbeddingProvider::OpenAI.dimensions(), 3072);
        assert_eq!(EmbeddingProvider::VertexAI.dimensions(), 768);
        assert_eq!(EmbeddingProvider::Bedrock.dimensions(), 1024);
    }

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
}

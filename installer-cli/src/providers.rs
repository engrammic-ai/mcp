/// Custom provider configuration for unlisted providers.
#[derive(Debug, Clone)]
pub struct OtherProvider {
    pub provider: String,
    pub model: String,
    pub dimensions: Option<u32>, // Only for embeddings
    pub env_vars: Vec<(String, String)>,
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
}

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
            RerankerProvider::None => Option::None,
            RerankerProvider::Cohere => Some("cohere"),
            RerankerProvider::VertexAI => Some("vertex_ai"),
            RerankerProvider::Other(o) => Some(&o.provider),
        }
    }

    pub fn model(&self) -> Option<&str> {
        match self {
            RerankerProvider::None => Option::None,
            RerankerProvider::Cohere => Some("rerank-v3.5"),
            RerankerProvider::VertexAI => Some("semantic-ranker-default@latest"),
            RerankerProvider::Other(o) => Some(&o.model),
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, RerankerProvider::None)
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
}

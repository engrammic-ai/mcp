package core

// EnvVar is a key-value pair for custom provider environment variables.
type EnvVar struct {
	Key   string
	Value string
}

// OtherProvider holds configuration for custom/unlisted providers.
type OtherProvider struct {
	Provider   string
	Model      string
	Dimensions *uint32
	EnvVars    []EnvVar
}

// CredentialSpec describes a credential that needs to be collected from the user.
type CredentialSpec struct {
	EnvVar string
	Prompt string
	Secret bool
}

// LlmProvider identifies an LLM provider.
type LlmProvider int

const (
	LlmOpenAI LlmProvider = iota
	LlmAnthropic
	LlmGeminiAPI
	LlmVertexAI
	LlmAzureOpenAI
	LlmBedrock
	LlmOther
)

// llmOtherData holds the OtherProvider payload when LlmProvider == LlmOther.
// Callers use NewLlmOther / LlmOtherData to work with it.
type llmProviderConfig struct {
	kind  LlmProvider
	other *OtherProvider
}

// LlmProviderConfig wraps an LlmProvider variant together with optional Other data.
type LlmProviderConfig struct {
	Kind  LlmProvider
	Other *OtherProvider
}

func NewLlmProvider(kind LlmProvider) LlmProviderConfig {
	return LlmProviderConfig{Kind: kind}
}

func NewLlmOther(o OtherProvider) LlmProviderConfig {
	return LlmProviderConfig{Kind: LlmOther, Other: &o}
}

func (p LlmProviderConfig) ProviderName() string {
	switch p.Kind {
	case LlmOpenAI:
		return "openai"
	case LlmAnthropic:
		return "anthropic"
	case LlmGeminiAPI:
		return "gemini"
	case LlmVertexAI:
		return "vertex_ai"
	case LlmAzureOpenAI:
		return "azure"
	case LlmBedrock:
		return "bedrock"
	case LlmOther:
		if p.Other != nil {
			return p.Other.Provider
		}
	}
	return ""
}

func (p LlmProviderConfig) ReasoningModel() string {
	switch p.Kind {
	case LlmOpenAI:
		return "gpt-4o"
	case LlmAnthropic:
		return "claude-sonnet-4-5"
	case LlmGeminiAPI:
		return "gemini-2.5-pro"
	case LlmVertexAI:
		return "gemini-2.5-pro"
	case LlmAzureOpenAI:
		return "gpt-4o"
	case LlmBedrock:
		return "anthropic.claude-sonnet"
	case LlmOther:
		if p.Other != nil {
			return p.Other.Model
		}
	}
	return ""
}

func (p LlmProviderConfig) FastModel() string {
	switch p.Kind {
	case LlmOpenAI:
		return "gpt-4o-mini"
	case LlmAnthropic:
		return "claude-haiku-4-5"
	case LlmGeminiAPI:
		return "gemini-2.5-flash"
	case LlmVertexAI:
		return "gemini-2.5-flash"
	case LlmAzureOpenAI:
		return "gpt-4o-mini"
	case LlmBedrock:
		return "anthropic.claude-haiku"
	case LlmOther:
		if p.Other != nil {
			return p.Other.Model
		}
	}
	return ""
}

func (p LlmProviderConfig) RequiredCredentials() []CredentialSpec {
	switch p.Kind {
	case LlmOpenAI:
		return []CredentialSpec{{EnvVar: "OPENAI_API_KEY", Prompt: "OpenAI API key", Secret: true}}
	case LlmAnthropic:
		return []CredentialSpec{{EnvVar: "ANTHROPIC_API_KEY", Prompt: "Anthropic API key", Secret: true}}
	case LlmGeminiAPI:
		return []CredentialSpec{{EnvVar: "GEMINI_API_KEY", Prompt: "Gemini API key (from ai.google.dev)", Secret: true}}
	case LlmVertexAI:
		return []CredentialSpec{
			{EnvVar: "VERTEX_PROJECT", Prompt: "GCP project ID", Secret: false},
			{EnvVar: "VERTEX_LOCATION", Prompt: "GCP location (e.g., us-central1)", Secret: false},
		}
	case LlmAzureOpenAI:
		return []CredentialSpec{
			{EnvVar: "AZURE_API_KEY", Prompt: "Azure OpenAI API key", Secret: true},
			{EnvVar: "AZURE_API_BASE", Prompt: "Azure endpoint URL", Secret: false},
			{EnvVar: "AZURE_API_VERSION", Prompt: "Azure API version (e.g., 2024-02-01)", Secret: false},
		}
	case LlmBedrock:
		return []CredentialSpec{{EnvVar: "AWS_REGION", Prompt: "AWS region (e.g., us-east-1)", Secret: false}}
	}
	return nil
}

// EmbeddingProvider identifies an embedding model provider.
type EmbeddingProvider int

const (
	EmbOpenAI EmbeddingProvider = iota
	EmbGeminiAPI
	EmbVertexAI
	EmbAzureOpenAI
	EmbBedrock
	EmbOther
)

// EmbeddingProviderConfig wraps an EmbeddingProvider variant with optional Other data.
type EmbeddingProviderConfig struct {
	Kind  EmbeddingProvider
	Other *OtherProvider
}

func NewEmbeddingProvider(kind EmbeddingProvider) EmbeddingProviderConfig {
	return EmbeddingProviderConfig{Kind: kind}
}

func NewEmbeddingOther(o OtherProvider) EmbeddingProviderConfig {
	return EmbeddingProviderConfig{Kind: EmbOther, Other: &o}
}

func (p EmbeddingProviderConfig) ProviderName() string {
	switch p.Kind {
	case EmbOpenAI:
		return "openai"
	case EmbGeminiAPI:
		return "gemini"
	case EmbVertexAI:
		return "vertex_ai"
	case EmbAzureOpenAI:
		return "azure"
	case EmbBedrock:
		return "bedrock"
	case EmbOther:
		if p.Other != nil {
			return p.Other.Provider
		}
	}
	return ""
}

func (p EmbeddingProviderConfig) Model() string {
	switch p.Kind {
	case EmbOpenAI:
		return "text-embedding-3-large"
	case EmbGeminiAPI:
		return "text-embedding-004"
	case EmbVertexAI:
		return "text-embedding-005"
	case EmbAzureOpenAI:
		return "text-embedding-3-large"
	case EmbBedrock:
		return "amazon.titan-embed-text-v2"
	case EmbOther:
		if p.Other != nil {
			return p.Other.Model
		}
	}
	return ""
}

func (p EmbeddingProviderConfig) Dimensions() uint32 {
	switch p.Kind {
	case EmbOpenAI:
		return 3072
	case EmbGeminiAPI:
		return 768
	case EmbVertexAI:
		return 768
	case EmbAzureOpenAI:
		return 3072
	case EmbBedrock:
		return 1024
	case EmbOther:
		if p.Other != nil && p.Other.Dimensions != nil {
			return *p.Other.Dimensions
		}
		return 768
	}
	return 768
}

func (p EmbeddingProviderConfig) RequiredCredentials() []CredentialSpec {
	switch p.Kind {
	case EmbOpenAI:
		return []CredentialSpec{{EnvVar: "OPENAI_API_KEY", Prompt: "OpenAI API key", Secret: true}}
	case EmbGeminiAPI:
		return []CredentialSpec{{EnvVar: "GEMINI_API_KEY", Prompt: "Gemini API key (from ai.google.dev)", Secret: true}}
	case EmbVertexAI:
		return []CredentialSpec{
			{EnvVar: "VERTEX_PROJECT", Prompt: "GCP project ID", Secret: false},
			{EnvVar: "VERTEX_LOCATION", Prompt: "GCP location", Secret: false},
		}
	case EmbAzureOpenAI:
		return []CredentialSpec{
			{EnvVar: "AZURE_API_KEY", Prompt: "Azure OpenAI API key", Secret: true},
			{EnvVar: "AZURE_API_BASE", Prompt: "Azure endpoint URL", Secret: false},
			{EnvVar: "AZURE_API_VERSION", Prompt: "Azure API version", Secret: false},
		}
	case EmbBedrock:
		return []CredentialSpec{{EnvVar: "AWS_REGION", Prompt: "AWS region", Secret: false}}
	}
	return nil
}

// RerankerProvider identifies a reranker provider.
type RerankerProvider int

const (
	RerankerLocalTeiMiniLM RerankerProvider = iota
	RerankerLocalTeiJina
	RerankerNone
	RerankerCohere
	RerankerVertexAI
	RerankerOther
)

// RerankerProviderConfig wraps a RerankerProvider variant with optional Other data.
type RerankerProviderConfig struct {
	Kind  RerankerProvider
	Other *OtherProvider
}

func NewRerankerProvider(kind RerankerProvider) RerankerProviderConfig {
	return RerankerProviderConfig{Kind: kind}
}

func NewRerankerOther(o OtherProvider) RerankerProviderConfig {
	return RerankerProviderConfig{Kind: RerankerOther, Other: &o}
}

func (p RerankerProviderConfig) ProviderName() (string, bool) {
	switch p.Kind {
	case RerankerLocalTeiMiniLM:
		return "tei", true
	case RerankerLocalTeiJina:
		return "tei", true
	case RerankerNone:
		return "", false
	case RerankerCohere:
		return "cohere", true
	case RerankerVertexAI:
		return "vertex_ai", true
	case RerankerOther:
		if p.Other != nil {
			return p.Other.Provider, true
		}
	}
	return "", false
}

func (p RerankerProviderConfig) Model() (string, bool) {
	switch p.Kind {
	case RerankerLocalTeiMiniLM:
		return "cross-encoder/ms-marco-MiniLM-L6-v2", true
	case RerankerLocalTeiJina:
		return "jinaai/jina-reranker-v2-base-multilingual", true
	case RerankerNone:
		return "", false
	case RerankerCohere:
		return "rerank-v3.5", true
	case RerankerVertexAI:
		return "semantic-ranker-default@latest", true
	case RerankerOther:
		if p.Other != nil {
			return p.Other.Model, true
		}
	}
	return "", false
}

func (p RerankerProviderConfig) IsNone() bool {
	return p.Kind == RerankerNone
}

func (p RerankerProviderConfig) IsLocal() bool {
	return p.Kind == RerankerLocalTeiMiniLM || p.Kind == RerankerLocalTeiJina
}

func (p RerankerProviderConfig) MemoryLimit() string {
	switch p.Kind {
	case RerankerLocalTeiMiniLM:
		return "1G"
	case RerankerLocalTeiJina:
		return "6G"
	}
	return "1G"
}

func (p RerankerProviderConfig) RequiredCredentials() []CredentialSpec {
	switch p.Kind {
	case RerankerCohere:
		return []CredentialSpec{{EnvVar: "COHERE_API_KEY", Prompt: "Cohere API key", Secret: true}}
	case RerankerVertexAI:
		return []CredentialSpec{
			{EnvVar: "VERTEX_PROJECT", Prompt: "GCP project ID", Secret: false},
			{EnvVar: "VERTEX_LOCATION", Prompt: "GCP location", Secret: false},
		}
	}
	return nil
}

// ProviderSet holds a complete selection of LLM, embedding, and reranker providers.
type ProviderSet struct {
	LLM       LlmProviderConfig
	Embedding EmbeddingProviderConfig
	Reranker  RerankerProviderConfig
}

// RequiredCredentials returns a deduplicated list of credentials needed by all providers.
func (ps ProviderSet) RequiredCredentials() []CredentialSpec {
	seen := make(map[string]bool)
	var result []CredentialSpec
	for _, cred := range append(append(
		ps.LLM.RequiredCredentials(),
		ps.Embedding.RequiredCredentials()...),
		ps.Reranker.RequiredCredentials()...) {
		if !seen[cred.EnvVar] {
			seen[cred.EnvVar] = true
			result = append(result, cred)
		}
	}
	return result
}

// CustomEnvVars returns environment variables from any Other providers in the set.
func (ps ProviderSet) CustomEnvVars() []EnvVar {
	var vars []EnvVar
	if ps.LLM.Kind == LlmOther && ps.LLM.Other != nil {
		vars = append(vars, ps.LLM.Other.EnvVars...)
	}
	if ps.Embedding.Kind == EmbOther && ps.Embedding.Other != nil {
		vars = append(vars, ps.Embedding.Other.EnvVars...)
	}
	if ps.Reranker.Kind == RerankerOther && ps.Reranker.Other != nil {
		vars = append(vars, ps.Reranker.Other.EnvVars...)
	}
	return vars
}

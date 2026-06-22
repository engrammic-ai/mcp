package core

import (
	"testing"
)

func TestOtherProviderStoresEnvVars(t *testing.T) {
	p := OtherProvider{
		Provider: "groq",
		Model:    "llama-3-70b",
		EnvVars:  []EnvVar{{Key: "GROQ_API_KEY", Value: "test"}},
	}
	if len(p.EnvVars) != 1 {
		t.Fatalf("expected 1 env var, got %d", len(p.EnvVars))
	}
	if p.EnvVars[0].Key != "GROQ_API_KEY" {
		t.Errorf("expected GROQ_API_KEY, got %s", p.EnvVars[0].Key)
	}
}

func TestLlmProviderNames(t *testing.T) {
	cases := []struct {
		p    LlmProviderConfig
		want string
	}{
		{NewLlmProvider(LlmOpenAI), "openai"},
		{NewLlmProvider(LlmAnthropic), "anthropic"},
		{NewLlmProvider(LlmGeminiAPI), "gemini"},
		{NewLlmProvider(LlmVertexAI), "vertex_ai"},
		{NewLlmProvider(LlmAzureOpenAI), "azure"},
		{NewLlmProvider(LlmBedrock), "bedrock"},
	}
	for _, c := range cases {
		if got := c.p.ProviderName(); got != c.want {
			t.Errorf("ProviderName() = %q, want %q", got, c.want)
		}
	}
}

func TestLlmProviderModels(t *testing.T) {
	if got := NewLlmProvider(LlmOpenAI).ReasoningModel(); got != "gpt-4o" {
		t.Errorf("OpenAI reasoning model = %q", got)
	}
	if got := NewLlmProvider(LlmOpenAI).FastModel(); got != "gpt-4o-mini" {
		t.Errorf("OpenAI fast model = %q", got)
	}
	if got := NewLlmProvider(LlmAnthropic).ReasoningModel(); got != "claude-sonnet-4-5" {
		t.Errorf("Anthropic reasoning model = %q", got)
	}
	if got := NewLlmProvider(LlmAnthropic).FastModel(); got != "claude-haiku-4-5" {
		t.Errorf("Anthropic fast model = %q", got)
	}
	if got := NewLlmProvider(LlmGeminiAPI).ReasoningModel(); got != "gemini-2.5-pro" {
		t.Errorf("GeminiAPI reasoning model = %q", got)
	}
	if got := NewLlmProvider(LlmBedrock).ReasoningModel(); got != "anthropic.claude-sonnet" {
		t.Errorf("Bedrock reasoning model = %q", got)
	}
}

func TestEmbeddingProviderDimensions(t *testing.T) {
	cases := []struct {
		p    EmbeddingProviderConfig
		want uint32
	}{
		{NewEmbeddingProvider(EmbOpenAI), 3072},
		{NewEmbeddingProvider(EmbGeminiAPI), 768},
		{NewEmbeddingProvider(EmbVertexAI), 768},
		{NewEmbeddingProvider(EmbAzureOpenAI), 3072},
		{NewEmbeddingProvider(EmbBedrock), 1024},
	}
	for _, c := range cases {
		if got := c.p.Dimensions(); got != c.want {
			t.Errorf("%v Dimensions() = %d, want %d", c.p.Kind, got, c.want)
		}
	}
}

func TestEmbeddingProviderModels(t *testing.T) {
	if got := NewEmbeddingProvider(EmbOpenAI).Model(); got != "text-embedding-3-large" {
		t.Errorf("OpenAI model = %q", got)
	}
	if got := NewEmbeddingProvider(EmbGeminiAPI).Model(); got != "text-embedding-004" {
		t.Errorf("GeminiAPI model = %q", got)
	}
	if got := NewEmbeddingProvider(EmbVertexAI).Model(); got != "text-embedding-005" {
		t.Errorf("VertexAI model = %q", got)
	}
	if got := NewEmbeddingProvider(EmbBedrock).Model(); got != "amazon.titan-embed-text-v2" {
		t.Errorf("Bedrock model = %q", got)
	}
}

func TestEmbeddingOtherDefaultDimensions(t *testing.T) {
	p := NewEmbeddingOther(OtherProvider{Provider: "custom", Model: "my-model"})
	if got := p.Dimensions(); got != 768 {
		t.Errorf("Other with nil dimensions = %d, want 768", got)
	}
}

func TestRerankerNone(t *testing.T) {
	p := NewRerankerProvider(RerankerNone)
	if !p.IsNone() {
		t.Error("RerankerNone.IsNone() should be true")
	}
	if p.IsLocal() {
		t.Error("RerankerNone.IsLocal() should be false")
	}
	if _, ok := p.Model(); ok {
		t.Error("RerankerNone.Model() should return false")
	}
}

func TestRerankerCohere(t *testing.T) {
	p := NewRerankerProvider(RerankerCohere)
	model, ok := p.Model()
	if !ok || model != "rerank-v3.5" {
		t.Errorf("Cohere model = %q, ok=%v", model, ok)
	}
	if p.IsNone() {
		t.Error("Cohere.IsNone() should be false")
	}
}

func TestRerankerLocalFlags(t *testing.T) {
	miniLM := NewRerankerProvider(RerankerLocalTeiMiniLM)
	jina := NewRerankerProvider(RerankerLocalTeiJina)
	if !miniLM.IsLocal() {
		t.Error("LocalTeiMiniLM should be local")
	}
	if !jina.IsLocal() {
		t.Error("LocalTeiJina should be local")
	}
	if miniLM.MemoryLimit() != "1G" {
		t.Errorf("MiniLM memory limit = %q, want 1G", miniLM.MemoryLimit())
	}
	if jina.MemoryLimit() != "6G" {
		t.Errorf("Jina memory limit = %q, want 6G", jina.MemoryLimit())
	}
}

func TestRerankerLocalModels(t *testing.T) {
	m, _ := NewRerankerProvider(RerankerLocalTeiMiniLM).Model()
	if m != "cross-encoder/ms-marco-MiniLM-L6-v2" {
		t.Errorf("MiniLM model = %q", m)
	}
	m, _ = NewRerankerProvider(RerankerLocalTeiJina).Model()
	if m != "jinaai/jina-reranker-v2-base-multilingual" {
		t.Errorf("Jina model = %q", m)
	}
}

func TestProviderSetDeduplicatesCredentials(t *testing.T) {
	ps := ProviderSet{
		LLM:       NewLlmProvider(LlmVertexAI),
		Embedding: NewEmbeddingProvider(EmbVertexAI),
		Reranker:  NewRerankerProvider(RerankerVertexAI),
	}
	creds := ps.RequiredCredentials()
	count := 0
	for _, c := range creds {
		if c.EnvVar == "VERTEX_PROJECT" {
			count++
		}
	}
	if count != 1 {
		t.Errorf("VERTEX_PROJECT appeared %d times, want 1", count)
	}
}

func TestProviderSetMixedCredentials(t *testing.T) {
	ps := ProviderSet{
		LLM:       NewLlmProvider(LlmAnthropic),
		Embedding: NewEmbeddingProvider(EmbOpenAI),
		Reranker:  NewRerankerProvider(RerankerCohere),
	}
	creds := ps.RequiredCredentials()
	has := func(envVar string) bool {
		for _, c := range creds {
			if c.EnvVar == envVar {
				return true
			}
		}
		return false
	}
	for _, want := range []string{"ANTHROPIC_API_KEY", "OPENAI_API_KEY", "COHERE_API_KEY"} {
		if !has(want) {
			t.Errorf("missing credential %s", want)
		}
	}
}

func TestProviderSetCustomEnvVars(t *testing.T) {
	ps := ProviderSet{
		LLM: NewLlmOther(OtherProvider{
			Provider: "groq",
			Model:    "llama-3-70b",
			EnvVars:  []EnvVar{{Key: "GROQ_API_KEY", Value: "test"}},
		}),
		Embedding: NewEmbeddingProvider(EmbOpenAI),
		Reranker:  NewRerankerProvider(RerankerNone),
	}
	vars := ps.CustomEnvVars()
	if len(vars) != 1 || vars[0].Key != "GROQ_API_KEY" {
		t.Errorf("unexpected custom env vars: %v", vars)
	}
}

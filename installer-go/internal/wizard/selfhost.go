// selfhost.go — SelfhostWizard: Runtime → Tier → Providers → Credentials → License → Config → Review → Deploy.
package wizard

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"text/template"

	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

// SelfhostWizard returns a Wizard wired with all selfhost steps.
func SelfhostWizard() *Wizard {
	return New("Engrammic Selfhost", []Step{
		{Name: "Runtime", Run: stepRuntime},
		{Name: "Tier", Run: stepTier},
		{Name: "Providers", Run: stepProviders},
		{Name: "Credentials", Run: stepCredentials},
		{Name: "License", Run: stepLicense},
		{Name: "Config", Run: stepConfig},
		{Name: "Review", Run: stepSelfhostReview},
		{Name: "Deploy", Run: stepDeploy},
	})
}

// ---------------------------------------------------------------------------
// Step 1: Runtime
// ---------------------------------------------------------------------------

func stepRuntime(w *Wizard) StepResult {
	hasDocker := exec.Command("docker", "info").Run() == nil
	hasPodman := exec.Command("podman", "info").Run() == nil

	if !hasDocker && !hasPodman {
		ui.Error("Docker or Podman is required to run a self-hosted server.")
		fmt.Println("\nInstall Docker from: https://docs.docker.com/get-docker/")
		return StepQuit
	}

	if hasDocker && !hasPodman {
		w.Runtime = "docker"
		ui.Info("Using Docker")
		return StepNext
	}
	if hasPodman && !hasDocker {
		w.Runtime = "podman"
		ui.Info("Using Podman")
		return StepNext
	}

	// Both available — ask the user.
	fmt.Println()
	ui.Title(w.StepHeader())
	idx := ui.PlainSelect("Select container runtime:", []string{"Docker", "Podman"}, 0)
	if idx == 0 {
		w.Runtime = "docker"
	} else {
		w.Runtime = "podman"
	}
	return StepNext
}

// ---------------------------------------------------------------------------
// Step 2: Tier
// ---------------------------------------------------------------------------

func stepTier(w *Wizard) StepResult {
	fmt.Println()
	ui.Title(w.StepHeader())
	idx := ui.PlainSelect(
		"Select deployment tier:",
		[]string{
			"Standalone — run local models (16 GB+ RAM required)",
			"Cloud Providers — use your own API keys",
		},
		1,
	)
	switch idx {
	case 0:
		w.Tier = "standalone"
	default:
		w.Tier = "cloud"
	}
	return StepNext
}

// ---------------------------------------------------------------------------
// Step 3: Providers
// ---------------------------------------------------------------------------

func stepProviders(w *Wizard) StepResult {
	if w.Tier == "standalone" {
		// Standalone uses Ollama for both LLM and embeddings — represented as
		// LlmOther / EmbOther so no API key prompts fire downstream.
		w.LLMProvider = core.NewLlmOther(core.OtherProvider{
			Provider: "ollama",
			Model:    "llama3.2",
		})
		w.EmbedProvider = core.NewEmbeddingOther(core.OtherProvider{
			Provider: "ollama",
			Model:    "nomic-embed-text",
		})
		w.Reranker = core.NewRerankerProvider(core.RerankerNone)
		return StepNext
	}

	fmt.Println()
	ui.Title(w.StepHeader())

	// LLM Provider
	llmNames := []string{
		"OpenAI (gpt-4o)",
		"Anthropic (claude-sonnet)",
		"Google Gemini",
		"Vertex AI",
		"Azure OpenAI",
		"AWS Bedrock",
	}
	llmKinds := []core.LlmProvider{
		core.LlmOpenAI,
		core.LlmAnthropic,
		core.LlmGeminiAPI,
		core.LlmVertexAI,
		core.LlmAzureOpenAI,
		core.LlmBedrock,
	}
	llmIdx := ui.PlainSelect("Select LLM provider:", llmNames, 0)
	w.LLMProvider = core.NewLlmProvider(llmKinds[llmIdx])

	// Embedding Provider
	embedNames := []string{
		"OpenAI (text-embedding-3-large)",
		"Google Gemini",
		"Vertex AI",
		"Azure OpenAI",
		"AWS Bedrock",
	}
	embedKinds := []core.EmbeddingProvider{
		core.EmbOpenAI,
		core.EmbGeminiAPI,
		core.EmbVertexAI,
		core.EmbAzureOpenAI,
		core.EmbBedrock,
	}
	embedIdx := ui.PlainSelect("Select embedding provider:", embedNames, 0)
	w.EmbedProvider = core.NewEmbeddingProvider(embedKinds[embedIdx])

	// Reranker
	rerankerNames := []string{
		"None",
		"Local TEI — MiniLM-L6 (1 GB RAM)",
		"Local TEI — Jina v2 (6 GB RAM)",
		"Cohere",
		"Vertex AI",
	}
	rerankerKinds := []core.RerankerProvider{
		core.RerankerNone,
		core.RerankerLocalTeiMiniLM,
		core.RerankerLocalTeiJina,
		core.RerankerCohere,
		core.RerankerVertexAI,
	}
	rerankerIdx := ui.PlainSelect("Select reranker (optional):", rerankerNames, 0)
	w.Reranker = core.NewRerankerProvider(rerankerKinds[rerankerIdx])

	return StepNext
}

// ---------------------------------------------------------------------------
// Step 4: Credentials
// ---------------------------------------------------------------------------

func stepCredentials(w *Wizard) StepResult {
	if w.Tier == "standalone" {
		return StepNext
	}

	ps := core.ProviderSet{
		LLM:       w.LLMProvider,
		Embedding: w.EmbedProvider,
		Reranker:  w.Reranker,
	}
	required := ps.RequiredCredentials()
	if len(required) == 0 {
		return StepNext
	}

	fmt.Println()
	ui.Title(w.StepHeader())
	ui.Info("Enter credentials for your selected providers.")
	fmt.Println()

	for _, cred := range required {
		// Skip if already collected (e.g. shared key between LLM and embeddings).
		if _, ok := w.Credentials[cred.EnvVar]; ok {
			continue
		}
		prompt := cred.Prompt
		if cred.Secret {
			prompt += " (hidden)"
		}
		val := ui.PlainInput(prompt, "")
		w.Credentials[cred.EnvVar] = val
	}

	return StepNext
}

// ---------------------------------------------------------------------------
// Step 5: License
// ---------------------------------------------------------------------------

func stepLicense(w *Wizard) StepResult {
	fmt.Println()
	ui.Title(w.StepHeader())
	license := ui.PlainInput("License key (leave blank for 14-day trial)", "")
	w.License = strings.TrimSpace(license)
	return StepNext
}

// ---------------------------------------------------------------------------
// Step 6: Config
// ---------------------------------------------------------------------------

func stepConfig(w *Wizard) StepResult {
	fmt.Println()
	ui.Title(w.StepHeader())

	if !core.IsPortAvailable(w.Port) {
		newPort, err := core.FindAvailablePort(w.Port)
		if err != nil {
			ui.Error("No available port found starting from %d", w.Port)
			return StepBack
		}
		who := core.WhoIsUsingPort(w.Port)
		if who != "" {
			ui.Warn("Port %d is in use (%s), suggesting %d instead", w.Port, who, newPort)
		} else {
			ui.Warn("Port %d is in use, suggesting %d instead", w.Port, newPort)
		}
		w.Port = newPort
	}

	fmt.Printf("\n  Port:           %d (available)\n", w.Port)
	fmt.Printf("  Data directory: %s/data\n\n", platform.UserConfigDir())

	if ui.PlainConfirm("Use these defaults?", true) {
		return StepNext
	}

	portStr := ui.PlainInput("Port", fmt.Sprintf("%d", w.Port))
	var port int
	if _, err := fmt.Sscanf(portStr, "%d", &port); err == nil && port > 0 {
		if !core.IsPortAvailable(port) {
			ui.Warn("Port %d is not available", port)
		} else {
			w.Port = port
		}
	}

	return StepNext
}

// ---------------------------------------------------------------------------
// Step 7: Review
// ---------------------------------------------------------------------------

func stepSelfhostReview(w *Wizard) StepResult {
	w.Endpoint = fmt.Sprintf("http://localhost:%d/mcp", w.Port)

	fmt.Println()
	ui.Title("Ready to deploy")
	fmt.Println()
	fmt.Printf("  Tier:       %s\n", w.Tier)
	fmt.Printf("  Runtime:    %s\n", w.Runtime)
	fmt.Printf("  LLM:        %s\n", w.LLMProvider.ProviderName())
	fmt.Printf("  Embedding:  %s (%s)\n", w.EmbedProvider.ProviderName(), w.EmbedProvider.Model())
	if !w.Reranker.IsNone() {
		model, _ := w.Reranker.Model()
		fmt.Printf("  Reranker:   %s (%s)\n", func() string { n, _ := w.Reranker.ProviderName(); return n }(), model)
	} else {
		fmt.Printf("  Reranker:   none\n")
	}
	fmt.Printf("  Port:       %d\n", w.Port)
	fmt.Printf("  Endpoint:   %s\n", w.Endpoint)
	fmt.Println()

	idx := ui.PlainSelect(
		"What would you like to do?",
		[]string{"Deploy now", "Go back", "Cancel"},
		0,
	)
	switch idx {
	case 0:
		return StepNext
	case 1:
		return StepBack
	default:
		return StepQuit
	}
}

// ---------------------------------------------------------------------------
// Step 8: Deploy
// ---------------------------------------------------------------------------

func stepDeploy(w *Wizard) StepResult {
	configDir := platform.UserConfigDir()
	if err := os.MkdirAll(configDir, 0755); err != nil {
		ui.Error("Failed to create config directory: %v", err)
		return StepQuit
	}

	// Generate docker-compose.yml
	composePath := filepath.Join(configDir, "docker-compose.yml")
	if err := generateCompose(w, composePath); err != nil {
		ui.Error("Failed to generate docker-compose.yml: %v", err)
		return StepQuit
	}
	ui.Success("Generated docker-compose.yml")

	// Generate .env
	envPath := filepath.Join(configDir, ".env")
	if err := generateEnv(w, envPath); err != nil {
		ui.Error("Failed to generate .env: %v", err)
		return StepQuit
	}
	ui.Success("Generated .env")

	// Pull images first so startup is fast.
	ui.Info("Pulling container images (this may take a few minutes)...")
	pull := exec.Command(w.Runtime, "compose", "-f", composePath, "pull")
	pull.Dir = configDir
	pull.Stdout = os.Stdout
	pull.Stderr = os.Stderr
	if err := pull.Run(); err != nil {
		ui.Warn("Image pull had warnings (continuing): %v", err)
	}

	// Start containers.
	ui.Info("Starting containers...")
	up := exec.Command(w.Runtime, "compose", "-f", composePath, "up", "-d")
	up.Dir = configDir
	if out, err := up.CombinedOutput(); err != nil {
		ui.Error("Failed to start containers: %v\n%s", err, string(out))
		return StepQuit
	}
	ui.Success("Containers started")

	ui.Success("Server is running at %s", w.Endpoint)
	fmt.Printf("\n  Manage with: engrammic selfhost [up|down|logs|upgrade]\n\n")

	return StepNext
}

// ---------------------------------------------------------------------------
// File generators
// ---------------------------------------------------------------------------

const composeTemplate = `version: '3.8'
services:
  engrammic:
    image: engrammic/server:latest
    ports:
      - "{{.Port}}:8000"
    env_file:
      - .env
    volumes:
      - ./data:/data
    restart: unless-stopped
`

func generateCompose(w *Wizard, path string) error {
	t, err := template.New("compose").Parse(composeTemplate)
	if err != nil {
		return err
	}
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()
	return t.Execute(f, w)
}

func generateEnv(w *Wizard, path string) error {
	var lines []string
	// Write collected credentials as env vars.
	for k, v := range w.Credentials {
		lines = append(lines, fmt.Sprintf("%s=%s", k, v))
	}
	if w.License != "" {
		lines = append(lines, fmt.Sprintf("ENGRAMMIC_LICENSE=%s", w.License))
	}
	lines = append(lines, fmt.Sprintf("ENGRAMMIC_PORT=%d", w.Port))
	content := strings.Join(lines, "\n")
	if len(lines) > 0 {
		content += "\n"
	}
	return os.WriteFile(path, []byte(content), 0600)
}

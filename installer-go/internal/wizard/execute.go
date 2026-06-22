// execute.go — config writing, deeplink launching, and state persistence.
package wizard

import (
	"fmt"
	"net/url"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"time"

	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

// ExecuteResult holds the outcome of installing a single harness.
type ExecuteResult struct {
	Harness core.Harness
	Method  string
	Success bool
	Error   error
	Detail  string
}

// executeSingleConfig handles a single harness install without progress display.
func executeSingleConfig(choice HarnessChoice, endpoint string) ExecuteResult {
	result := ExecuteResult{
		Harness: choice.Harness,
		Method:  choice.Method,
	}

	switch choice.Method {
	case "file":
		result.Error = writeFileConfig(choice.Harness, endpoint)
		result.Detail = "configured"
	case "deeplink":
		result.Error = openDeeplink(choice.Harness, endpoint)
		result.Detail = "opened"
	case "manual":
		result.Detail = "skipped (manual)"
	}

	result.Success = result.Error == nil
	return result
}

// ExecuteConfigs iterates through harness choices, writes configs or opens
// deeplinks, and updates a ProgressList in real time.
func ExecuteConfigs(choices []HarnessChoice, endpoint string, progress *ui.ProgressList) []ExecuteResult {
	var results []ExecuteResult

	for _, choice := range choices {
		progress.SetStatus(choice.Harness.Name, ui.StatusRunning, "")

		result := ExecuteResult{
			Harness: choice.Harness,
			Method:  choice.Method,
		}

		switch choice.Method {
		case "file":
			result.Error = writeFileConfig(choice.Harness, endpoint)
			result.Detail = "configured"
		case "deeplink":
			result.Error = openDeeplink(choice.Harness, endpoint)
			result.Detail = "opened"
		case "manual":
			result.Detail = "skipped (manual)"
		}

		if result.Error != nil {
			progress.SetStatus(choice.Harness.Name, ui.StatusFailed, result.Error.Error())
			result.Success = false
		} else if choice.Method == "manual" {
			progress.SetStatus(choice.Harness.Name, ui.StatusSkipped, result.Detail)
			result.Success = true
		} else {
			progress.SetStatus(choice.Harness.Name, ui.StatusDone, result.Detail)
			result.Success = true
		}

		results = append(results, result)
	}

	return results
}

// writeFileConfig merges the Engrammic MCP entry into the harness config file.
func writeFileConfig(h core.Harness, endpoint string) error {
	if h.Shape == nil {
		return fmt.Errorf("harness %q has no config shape for file edit", h.ID)
	}

	configPath := platform.ExpandPath(h.ConfigPath)

	// Best-effort backup of any existing file.
	if _, err := os.Stat(configPath); err == nil {
		_, _ = core.BackupConfig(configPath)
	}

	// Ensure parent directory exists.
	dir := filepath.Dir(configPath)
	if err := os.MkdirAll(dir, 0o755); err != nil {
		return fmt.Errorf("create dir: %w", err)
	}

	// Read existing content or start with an empty JSON object.
	existing, err := os.ReadFile(configPath)
	if err != nil {
		existing = []byte("{}")
	}

	// Merge our entry into the existing content.
	merged, err := core.MergeServerConfig(existing, *h.Shape, "engrammic", endpoint)
	if err != nil {
		return fmt.Errorf("merge config: %w", err)
	}

	// Atomic write via temp file + rename.
	tmpPath := configPath + ".tmp"
	if err := os.WriteFile(tmpPath, merged, 0o644); err != nil {
		return fmt.Errorf("write temp file: %w", err)
	}
	if err := os.Rename(tmpPath, configPath); err != nil {
		_ = os.Remove(tmpPath)
		return fmt.Errorf("rename config file: %w", err)
	}

	return nil
}

// openDeeplink launches the deeplink URI for a harness.
func openDeeplink(h core.Harness, endpoint string) error {
	if h.DeepLink == nil {
		return fmt.Errorf("harness %q has no deeplink", h.ID)
	}

	link, err := buildDeeplinkURL(*h.DeepLink, h.Name, endpoint)
	if err != nil {
		return err
	}

	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("open", link)
	case "linux":
		cmd = exec.Command("xdg-open", link)
	case "windows":
		cmd = exec.Command("cmd", "/c", "start", "", link)
	default:
		return fmt.Errorf("unsupported OS: %s", runtime.GOOS)
	}

	return cmd.Start()
}

// buildDeeplinkURL constructs the URI for a given DeepLinkKind.
func buildDeeplinkURL(kind core.DeepLinkKind, name, endpoint string) (string, error) {
	switch kind {
	case core.DeepLinkVSCode:
		// vscode:mcp/install?{urlencoded-json}
		params := url.Values{}
		params.Set("url", endpoint)
		params.Set("name", "engrammic")
		return "vscode:mcp/install?" + params.Encode(), nil

	case core.DeepLinkCursor:
		// cursor://anysphere.cursor-deeplink/mcp/install?name=X&url=Y
		params := url.Values{}
		params.Set("name", "engrammic")
		params.Set("url", endpoint)
		return "cursor://anysphere.cursor-deeplink/mcp/install?" + params.Encode(), nil

	case core.DeepLinkWindsurf:
		// windsurf://windsurf-mcp-registry?serverName=X&serverUrl=Y
		params := url.Values{}
		params.Set("serverName", "engrammic")
		params.Set("serverUrl", endpoint)
		return "windsurf://windsurf-mcp-registry?" + params.Encode(), nil

	case core.DeepLinkClaudeWeb:
		// https://claude.ai/install-mcp?url=Y
		params := url.Values{}
		params.Set("url", endpoint)
		params.Set("name", "engrammic")
		return "https://claude.ai/install-mcp?" + params.Encode(), nil

	default:
		return "", fmt.Errorf("unknown deeplink kind %d for %q", kind, name)
	}
}

// UpdateState persists installation results to ~/.engrammic/state.json.
func UpdateState(w *Wizard, results []ExecuteResult) error {
	state, err := core.LoadState()
	if err != nil {
		// Non-fatal: construct a fresh state rather than aborting.
		state = &core.State{
			Version:   1,
			Harnesses: make(map[string]core.HarnessState),
		}
	}

	now := time.Now()
	state.LastUpdated = now

	if w.Mode == "selfhost" && w.Port > 0 {
		state.Server = &core.ServerState{
			Port:      w.Port,
			Endpoint:  w.Endpoint,
			StartedAt: now,
		}
	}

	for _, r := range results {
		if !r.Success {
			continue
		}
		state.Harnesses[r.Harness.ID] = core.HarnessState{
			InstalledAt: now,
			ConfigPath:  platform.ExpandPath(r.Harness.ConfigPath),
			Endpoint:    w.Endpoint,
		}
	}

	return state.Save()
}

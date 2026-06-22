// doctor.go
package cli

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/spf13/cobra"
	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

var doctorCmd = &cobra.Command{
	Use:   "doctor",
	Short: "Run diagnostics (ports, docker, configs)",
	Run:   runDoctor,
}

func init() {
	RootCmd.AddCommand(doctorCmd)
}

func runDoctor(cmd *cobra.Command, args []string) {
	ui.PrintBanner()
	fmt.Println()

	var warnings, errors int

	state, _ := core.LoadState()

	// Connectivity
	ui.Title("Connectivity")
	endpoint := "https://beta.engrammic.ai/mcp/"
	if state != nil && state.Server != nil && state.Server.Endpoint != "" {
		endpoint = state.Server.Endpoint
	}

	if checkEndpoint(endpoint) {
		ui.Success("Endpoint reachable: %s", endpoint)
	} else {
		ui.Error("Endpoint unreachable: %s", endpoint)
		errors++
	}
	fmt.Println()

	// Docker (only if selfhosted — container ID recorded in state)
	if state != nil && state.Server != nil && state.Server.ContainerID != nil && *state.Server.ContainerID != "" {
		containerID := *state.Server.ContainerID
		ui.Title("Docker")
		if checkDocker() {
			ui.Success("Docker running")
			if checkContainer(containerID) {
				short := containerID
				if len(short) > 12 {
					short = short[:12]
				}
				ui.Success("Container healthy: %s", short)
			} else {
				short := containerID
				if len(short) > 12 {
					short = short[:12]
				}
				ui.Error("Container not running: %s", short)
				errors++
			}
		} else {
			ui.Error("Docker not running")
			errors++
		}
		fmt.Println()
	}

	// Ports
	ui.Title("Ports")
	port := core.DefaultPort
	if state != nil && state.Server != nil && state.Server.Port > 0 {
		port = state.Server.Port
	}

	if !core.IsPortAvailable(port) {
		who := core.WhoIsUsingPort(port)
		if state != nil && state.Server != nil && state.IsServerRunning() {
			ui.Success("Port %d in use by engrammic", port)
		} else if who != "" {
			ui.Warn("Port %d in use: %s", port, who)
			warnings++
		} else {
			ui.Warn("Port %d in use", port)
			warnings++
		}
	} else {
		ui.Info("Port %d available", port)
	}
	fmt.Println()

	// Configs
	ui.Title("Configs")
	if state != nil && len(state.Harnesses) > 0 {
		for id, hs := range state.Harnesses {
			h := core.FromID(id)
			name := id
			if h != nil {
				name = h.Name
			}

			if hs.ConfigPath == "" {
				// Harness configured via endpoint only (deeplink/manual)
				if hs.Endpoint != "" {
					ui.Info("%-18s via endpoint (cannot verify config)", name)
					warnings++
				} else {
					ui.Info("%-18s no config path recorded", name)
				}
				continue
			}

			valid, endpointMatch := checkConfig(hs.ConfigPath, endpoint)
			if valid && endpointMatch {
				ui.Success("%-18s valid, endpoint matches", name)
			} else if valid {
				ui.Warn("%-18s valid JSON, endpoint mismatch", name)
				warnings++
			} else {
				ui.Error("%-18s invalid or missing", name)
				errors++
			}
		}
	} else {
		ui.Info("No configs to check")
	}
	fmt.Println()

	// Summary
	if errors > 0 {
		ui.Error("%d error(s), %d warning(s)", errors, warnings)
		os.Exit(1)
	} else if warnings > 0 {
		ui.Warn("%d warning(s), 0 errors", warnings)
	} else {
		ui.Success("All checks passed")
	}
}

func checkEndpoint(url string) bool {
	client := &http.Client{Timeout: 5 * time.Second}
	resp, err := client.Get(url)
	if err != nil {
		return false
	}
	resp.Body.Close()
	return resp.StatusCode < 500
}

func checkDocker() bool {
	cmd := exec.Command("docker", "info")
	return cmd.Run() == nil
}

func checkContainer(id string) bool {
	cmd := exec.Command("docker", "ps", "-q", "--filter", "id="+id)
	out, err := cmd.Output()
	return err == nil && strings.TrimSpace(string(out)) != ""
}

func checkConfig(path, expectedEndpoint string) (valid, endpointMatch bool) {
	data, err := os.ReadFile(path)
	if err != nil {
		return false, false
	}

	var obj map[string]any
	if err := json.Unmarshal(data, &obj); err != nil {
		return false, false
	}

	valid = true

	// Check for engrammic entry with matching endpoint under mcpServers
	if servers, ok := obj["mcpServers"].(map[string]any); ok {
		if eng, ok := servers["engrammic"].(map[string]any); ok {
			if u, ok := eng["url"].(string); ok {
				endpointMatch = u == expectedEndpoint
			}
			if !endpointMatch {
				if u, ok := eng["serverUrl"].(string); ok {
					endpointMatch = u == expectedEndpoint
				}
			}
		}
	}

	return valid, endpointMatch
}

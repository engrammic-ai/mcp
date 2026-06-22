// status.go
package cli

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show installed harnesses, server state, endpoint",
	Run:   runStatus,
}

func init() {
	RootCmd.AddCommand(statusCmd)
}

func runStatus(cmd *cobra.Command, args []string) {
	ui.PrintBanner()
	fmt.Println()

	state, err := core.LoadState()
	if err != nil {
		ui.Warn("Could not load state: %v", err)
		state = &core.State{}
	}

	// Server status
	ui.Title("Server")
	if state.Server != nil {
		endpoint := state.Server.Endpoint
		if endpoint == "" {
			endpoint = fmt.Sprintf("http://localhost:%d/mcp", state.Server.Port)
		}
		if state.IsServerRunning() {
			ui.Success("%-15s %s", endpoint, "(running)")
		} else {
			ui.Warn("%-15s %s", endpoint, "(stopped)")
		}
	} else {
		ui.Info("Not configured")
	}
	fmt.Println()

	// Configured harnesses
	ui.Title("Configured Editors")
	if len(state.Harnesses) > 0 {
		for id, hs := range state.Harnesses {
			h := core.FromID(id)
			name := id
			if h != nil {
				name = h.Name
			}

			detail := hs.ConfigPath
			if detail == "" {
				detail = hs.Endpoint
			}

			if hs.ConfigPath != "" {
				if _, err := os.Stat(hs.ConfigPath); err == nil {
					ui.Success("%-18s %s", name, detail)
				} else {
					ui.Warn("%-18s %s (file missing)", name, detail)
				}
			} else if hs.Endpoint != "" {
				ui.Success("%-18s via endpoint: %s", name, hs.Endpoint)
			} else {
				ui.Info("%-18s (no config path recorded)", name)
			}
		}
	} else {
		ui.Info("No editors configured")
	}
	fmt.Println()

	// Detected but not configured
	detected := platform.DetectEditors()
	ui.Title("Detected (not configured)")
	unconfigured := 0
	for _, h := range detected {
		if _, ok := state.Harnesses[h.ID]; ok {
			continue
		}
		unconfigured++
		ui.Info("%-18s %s", h.Name, h.ConfigPath)
	}
	if unconfigured == 0 {
		ui.Info("All detected editors are configured")
	}
}

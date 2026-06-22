// license.go — "engrammic license" command.
package cli

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"
	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

var licenseCmd = &cobra.Command{
	Use:   "license [key]",
	Short: "Show or set license key",
	Args:  cobra.MaximumNArgs(1),
	Run:   runLicense,
}

func init() {
	RootCmd.AddCommand(licenseCmd)
}

func runLicense(cmd *cobra.Command, args []string) {
	envPath := filepath.Join(platform.UserConfigDir(), ".env")

	if len(args) == 0 {
		// Show current license
		data, err := os.ReadFile(envPath)
		if err != nil {
			ui.Info("No license configured")
			return
		}
		for _, line := range strings.Split(string(data), "\n") {
			if strings.HasPrefix(line, "ENGRAMMIC_LICENSE=") {
				key := strings.TrimPrefix(line, "ENGRAMMIC_LICENSE=")
				if len(key) > 8 {
					key = key[:4] + "..." + key[len(key)-4:]
				}
				ui.Success("License: %s", key)
				return
			}
		}
		ui.Info("No license configured")
		return
	}

	// Set license
	key := args[0]

	// Read existing env, preserving non-license lines
	var lines []string
	if data, err := os.ReadFile(envPath); err == nil {
		for _, line := range strings.Split(string(data), "\n") {
			if !strings.HasPrefix(line, "ENGRAMMIC_LICENSE=") && line != "" {
				lines = append(lines, line)
			}
		}
	}
	lines = append(lines, fmt.Sprintf("ENGRAMMIC_LICENSE=%s", key))

	// Ensure config dir exists
	if err := os.MkdirAll(platform.UserConfigDir(), 0755); err != nil {
		ui.Error("Failed to create config dir: %v", err)
		return
	}

	if err := os.WriteFile(envPath, []byte(strings.Join(lines, "\n")), 0600); err != nil {
		ui.Error("Failed to save: %v", err)
		return
	}
	ui.Success("License saved")
}

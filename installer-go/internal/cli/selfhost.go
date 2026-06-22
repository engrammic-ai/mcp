// selfhost.go — "engrammic selfhost" command group with up/down/logs/upgrade subcommands.
package cli

import (
	"os"
	"os/exec"
	"path/filepath"

	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
	"github.com/anthropics/engrammic/installer/internal/wizard"
	"github.com/spf13/cobra"
)

var selfhostCmd = &cobra.Command{
	Use:   "selfhost",
	Short: "Self-host Engrammic on your own infrastructure",
	Long:  "Run the selfhost setup wizard or manage a running Engrammic server.",
	Run:   runSelfhost,
}

var selfhostUpCmd = &cobra.Command{
	Use:   "up",
	Short: "Start the self-hosted Engrammic server",
	Run:   runSelfhostUp,
}

var selfhostDownCmd = &cobra.Command{
	Use:   "down",
	Short: "Stop the self-hosted Engrammic server",
	Run:   runSelfhostDown,
}

var selfhostLogsCmd = &cobra.Command{
	Use:   "logs",
	Short: "Tail container logs",
	Run:   runSelfhostLogs,
}

var selfhostUpgradeCmd = &cobra.Command{
	Use:   "upgrade",
	Short: "Pull the latest images and restart the server",
	Run:   runSelfhostUpgrade,
}

func init() {
	selfhostCmd.AddCommand(selfhostUpCmd)
	selfhostCmd.AddCommand(selfhostDownCmd)
	selfhostCmd.AddCommand(selfhostLogsCmd)
	selfhostCmd.AddCommand(selfhostUpgradeCmd)
	RootCmd.AddCommand(selfhostCmd)
}

// ---------------------------------------------------------------------------
// selfhost (root — runs the wizard)
// ---------------------------------------------------------------------------

func runSelfhost(cmd *cobra.Command, args []string) {
	ui.PrintBanner()
	w := wizard.SelfhostWizard()
	if err := w.Run(); err != nil {
		// "wizard cancelled" is a normal exit — no error message needed.
	}
}

// ---------------------------------------------------------------------------
// selfhost up
// ---------------------------------------------------------------------------

func runSelfhostUp(cmd *cobra.Command, args []string) {
	composePath := selfhostComposePath()
	if _, err := os.Stat(composePath); os.IsNotExist(err) {
		ui.Error("No selfhost configuration found. Run 'engrammic selfhost' first.")
		return
	}

	runtime := detectRuntime()
	ui.Info("Starting containers...")
	c := exec.Command(runtime, "compose", "-f", composePath, "up", "-d")
	c.Stdout = os.Stdout
	c.Stderr = os.Stderr
	if err := c.Run(); err != nil {
		ui.Error("Failed to start containers: %v", err)
		return
	}
	ui.Success("Server started")
}

// ---------------------------------------------------------------------------
// selfhost down
// ---------------------------------------------------------------------------

func runSelfhostDown(cmd *cobra.Command, args []string) {
	composePath := selfhostComposePath()
	if _, err := os.Stat(composePath); os.IsNotExist(err) {
		ui.Error("No selfhost configuration found. Run 'engrammic selfhost' first.")
		return
	}

	runtime := detectRuntime()
	ui.Info("Stopping containers...")
	c := exec.Command(runtime, "compose", "-f", composePath, "down")
	c.Stdout = os.Stdout
	c.Stderr = os.Stderr
	if err := c.Run(); err != nil {
		ui.Error("Failed to stop containers: %v", err)
		return
	}
	ui.Success("Server stopped")
}

// ---------------------------------------------------------------------------
// selfhost logs
// ---------------------------------------------------------------------------

func runSelfhostLogs(cmd *cobra.Command, args []string) {
	composePath := selfhostComposePath()
	if _, err := os.Stat(composePath); os.IsNotExist(err) {
		ui.Error("No selfhost configuration found. Run 'engrammic selfhost' first.")
		return
	}

	runtime := detectRuntime()
	c := exec.Command(runtime, "compose", "-f", composePath, "logs", "-f")
	c.Stdout = os.Stdout
	c.Stderr = os.Stderr
	c.Run() //nolint:errcheck — user may Ctrl-C to stop following
}

// ---------------------------------------------------------------------------
// selfhost upgrade
// ---------------------------------------------------------------------------

func runSelfhostUpgrade(cmd *cobra.Command, args []string) {
	composePath := selfhostComposePath()
	if _, err := os.Stat(composePath); os.IsNotExist(err) {
		ui.Error("No selfhost configuration found. Run 'engrammic selfhost' first.")
		return
	}

	runtime := detectRuntime()

	ui.Info("Pulling latest images...")
	pull := exec.Command(runtime, "compose", "-f", composePath, "pull")
	pull.Stdout = os.Stdout
	pull.Stderr = os.Stderr
	if err := pull.Run(); err != nil {
		ui.Error("Image pull failed: %v", err)
		return
	}

	ui.Info("Restarting containers...")
	up := exec.Command(runtime, "compose", "-f", composePath, "up", "-d")
	up.Stdout = os.Stdout
	up.Stderr = os.Stderr
	if err := up.Run(); err != nil {
		ui.Error("Restart failed: %v", err)
		return
	}

	ui.Success("Upgrade complete")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// selfhostComposePath returns the canonical path to the docker-compose.yml
// generated by the selfhost wizard.
func selfhostComposePath() string {
	return filepath.Join(platform.UserConfigDir(), "docker-compose.yml")
}

// detectRuntime returns "docker" or "podman" depending on which is available.
// Falls back to "docker" when neither (or both) can be detected, letting the
// error surface from the actual compose call.
func detectRuntime() string {
	if exec.Command("podman", "info").Run() == nil {
		return "podman"
	}
	return "docker"
}

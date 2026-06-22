// install.go — "engrammic install" command.
package cli

import (
	"github.com/spf13/cobra"
	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
	"github.com/anthropics/engrammic/installer/internal/wizard"
)

var installCmd = &cobra.Command{
	Use:   "install",
	Short: "Run the cloud install wizard",
	Long:  "Interactively configure editor integrations against the Engrammic MCP cloud service.",
	Run:   runInstall,
}

func init() {
	RootCmd.AddCommand(installCmd)
}

func runInstall(cmd *cobra.Command, args []string) {
	flags := GetGlobalFlags()

	ui.PrintBanner()

	if flags.Yes {
		runNonInteractiveInstall(flags)
		return
	}

	w := wizard.CloudInstallWizard()

	// Pre-fill endpoint from --endpoint flag if provided.
	if flags.Endpoint != "" {
		w.Endpoint = flags.Endpoint
	}

	if err := w.Run(); err != nil {
		// "wizard cancelled" is a normal exit — don't print an error.
		return
	}
}

// runNonInteractiveInstall handles the --yes flag path: detects editors,
// applies sensible defaults, and installs without any prompts.
func runNonInteractiveInstall(flags GlobalFlags) {
	ui.Info("Running non-interactive install...")

	endpoint := flags.Endpoint
	if endpoint == "" {
		endpoint = wizard.DefaultCloudEndpoint
	}

	detected := platform.DetectEditors()
	if len(detected) == 0 {
		ui.Warn("No editors detected — nothing to install.")
		return
	}

	ui.Info("Using endpoint: %s", endpoint)

	var choices []wizard.HarnessChoice
	for _, h := range detected {
		method := wizard.DefaultMethod(h)
		// --method flag overrides when supported.
		if flags.Method == "deeplink" && h.DeepLink != nil {
			method = "deeplink"
		} else if flags.Method == "file" && h.Shape != nil {
			method = "file"
		}

		choices = append(choices, wizard.HarnessChoice{
			Harness:  h,
			Method:   method,
			Selected: true,
		})
		ui.Info("Configuring %s (%s)...", h.Name, method)
	}

	names := make([]string, len(choices))
	for i, c := range choices {
		names[i] = c.Harness.Name
	}
	progress := ui.NewProgressList(names)

	results := wizard.ExecuteConfigs(choices, endpoint, progress)

	ui.Info("")
	ui.Info("%s", progress.Render())

	allOK := true
	for _, r := range results {
		if r.Success {
			ui.Success("%s", r.Harness.Name)
		} else {
			ui.Error("%s: %v", r.Harness.Name, r.Error)
			allOK = false
		}
	}

	if allOK {
		ui.Success("Done!")
	} else {
		ui.Warn("Some editors failed to configure — run 'engrammic doctor' for details.")
	}
}

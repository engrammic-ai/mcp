// remove.go — "engrammic remove" command.
package cli

import (
	"github.com/spf13/cobra"
	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/ui"
	"github.com/anthropics/engrammic/installer/internal/wizard"
)

var removeCmd = &cobra.Command{
	Use:   "remove",
	Short: "Uninstall wizard (select what to remove)",
	Long:  "Interactively remove editor integrations, the self-hosted server, or everything.",
	Run:   runRemove,
}

func init() {
	RootCmd.AddCommand(removeCmd)
}

func runRemove(cmd *cobra.Command, args []string) {
	flags := GetGlobalFlags()

	ui.PrintBanner()

	if flags.Yes {
		runNonInteractiveRemove()
		return
	}

	w := wizard.RemoveWizard()
	if err := w.Run(); err != nil {
		// "wizard cancelled" is a normal exit — don't print an error.
		return
	}
}

// runNonInteractiveRemove handles the --yes flag: removes everything without prompts.
func runNonInteractiveRemove() {
	ui.Info("Running non-interactive remove (everything)...")

	state, err := core.LoadState()
	if err != nil || state == nil {
		ui.Info("Nothing installed")
		return
	}

	if len(state.Harnesses) == 0 && state.Server == nil {
		ui.Info("Nothing to remove")
		return
	}

	// Delegate to the wizard's execute step by pre-setting mode and skipping
	// the Select/Confirm steps.
	w := wizard.RemoveWizard()
	w.Mode = "everything"
	// Advance past Select (0) and Confirm (1) to Execute (2).
	w.Current = 2
	if err := w.Run(); err != nil {
		// Cancelled is fine.
	}
}

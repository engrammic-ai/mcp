// root.go
package cli

import (
	"os"

	"github.com/spf13/cobra"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

var (
	flagYes     bool
	flagVerbose bool
	flagNoColor bool
	flagEndpoint string
	flagTools   []string
	flagMethod  string
)

// Version information, set via SetVersion.
var (
	Version = "dev"
	Commit  = "none"
	Date    = "unknown"
)

// SetVersion stores build-time version metadata.
func SetVersion(v, c, d string) {
	Version = v
	Commit = c
	Date = d
}

// GlobalFlags holds the parsed values of the persistent global flags.
type GlobalFlags struct {
	Yes      bool
	Verbose  bool
	NoColor  bool
	Endpoint string
	Tools    []string
	Method   string // "file" or "deeplink"
}

// GetGlobalFlags returns the current values of all global flags.
func GetGlobalFlags() GlobalFlags {
	return GlobalFlags{
		Yes:      flagYes,
		Verbose:  flagVerbose,
		NoColor:  flagNoColor,
		Endpoint: flagEndpoint,
		Tools:    flagTools,
		Method:   flagMethod,
	}
}

// RootCmd is the root Cobra command for the engrammic CLI.
var RootCmd = &cobra.Command{
	Use:   "engrammic",
	Short: "Engrammic installer and manager",
	Long:  "Install, configure, and manage Engrammic MCP servers and editor integrations.",
	PersistentPreRun: func(cmd *cobra.Command, args []string) {
		if flagNoColor {
			os.Setenv("NO_COLOR", "1")
		}
	},
	Run: func(cmd *cobra.Command, args []string) {
		// Default action when no sub-command is given: show help.
		// Sub-commands (install, uninstall, …) are registered separately.
		cmd.Help() //nolint:errcheck
	},
}

func init() {
	RootCmd.PersistentFlags().BoolVarP(&flagYes, "yes", "y", false, "Accept defaults, no prompts")
	RootCmd.PersistentFlags().BoolVarP(&flagVerbose, "verbose", "v", false, "Debug output")
	RootCmd.PersistentFlags().BoolVar(&flagNoColor, "no-color", false, "Disable colors")
	RootCmd.PersistentFlags().StringVar(&flagEndpoint, "endpoint", "", "Override endpoint URL")
	RootCmd.PersistentFlags().StringSliceVar(&flagTools, "tool", nil, "Pre-select specific tools")
	RootCmd.PersistentFlags().StringVar(&flagMethod, "method", "", "Prefer 'file' or 'deeplink'")
}

// Execute runs the root command and exits on error.
func Execute() {
	if err := RootCmd.Execute(); err != nil {
		ui.Fatal("%v", err)
	}
}

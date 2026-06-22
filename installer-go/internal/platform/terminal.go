package platform

import (
	"os"

	"golang.org/x/term"
)

// IsTTY reports whether os.Stdout is a terminal.
func IsTTY() bool {
	return term.IsTerminal(int(os.Stdout.Fd()))
}

// IsDumb reports whether the terminal is considered dumb (no color/rich output).
// Returns true when TERM=dumb or the NO_COLOR env var is set.
func IsDumb() bool {
	if os.Getenv("NO_COLOR") != "" {
		return true
	}
	if os.Getenv("TERM") == "dumb" {
		return true
	}
	return false
}

// UseRichUI reports whether rich terminal UI (colors, prompts) should be used.
// Requires a real TTY that is not dumb.
func UseRichUI() bool {
	return IsTTY() && !IsDumb()
}

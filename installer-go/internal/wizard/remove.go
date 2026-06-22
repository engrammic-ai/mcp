// remove.go — "engrammic remove" wizard: Select → Confirm → Execute.
package wizard

import (
	"fmt"
	"os"
	"strings"

	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

// RemoveWizard returns a Wizard wired with the three remove steps.
func RemoveWizard() *Wizard {
	return New("Engrammic Remove", []Step{
		{Name: "Select", Run: stepRemoveSelect},
		{Name: "Confirm", Run: stepRemoveConfirm},
		{Name: "Execute", Run: stepRemoveExecute},
	})
}

// ---------------------------------------------------------------------------
// Step 1: Select
// ---------------------------------------------------------------------------

func stepRemoveSelect(w *Wizard) StepResult {
	state, err := core.LoadState()
	if err != nil || state == nil {
		ui.Info("Nothing installed")
		return StepQuit
	}

	hasEditors := len(state.Harnesses) > 0
	hasServer := state.Server != nil

	if !hasEditors && !hasServer {
		ui.Info("Nothing to remove")
		return StepQuit
	}

	// Build option list based on what is actually installed.
	var options []string
	var keys []string // parallel slice of mode identifiers

	if hasEditors {
		var editorNames []string
		for id := range state.Harnesses {
			h := core.FromID(id)
			if h != nil {
				editorNames = append(editorNames, h.Name)
			} else {
				editorNames = append(editorNames, id)
			}
		}
		label := fmt.Sprintf("Editor configs (%s)", strings.Join(editorNames, ", "))
		options = append(options, label)
		keys = append(keys, "editors")
	}

	if hasServer {
		options = append(options, "Selfhost server (docker containers + data)")
		keys = append(keys, "server")
	}

	options = append(options, "Everything")
	keys = append(keys, "everything")

	fmt.Println()
	ui.Title(w.StepHeader())
	fmt.Println()

	idx := ui.PlainSelect("What do you want to remove?", options, 0)
	if idx < 0 || idx >= len(keys) {
		return StepQuit
	}

	w.Mode = keys[idx]

	// Warn about plain-UI non-interactive path when --yes is not set.
	if !platform.UseRichUI() {
		// Already in plain mode; PlainSelect already handled input.
	}

	return StepNext
}

// ---------------------------------------------------------------------------
// Step 2: Confirm
// ---------------------------------------------------------------------------

func stepRemoveConfirm(w *Wizard) StepResult {
	state, err := core.LoadState()
	if err != nil || state == nil {
		return StepQuit
	}

	fmt.Println()
	ui.Title("Confirm removal")
	fmt.Println()
	fmt.Println("  Will remove:")

	if w.Mode == "everything" || w.Mode == "editors" {
		for id, hs := range state.Harnesses {
			h := core.FromID(id)
			name := id
			if h != nil {
				name = h.Name
			}
			if h != nil && h.Method == core.InstallMethodPrintInstructions {
				fmt.Printf("    • %s (manual — cannot auto-remove)\n", name)
			} else {
				fmt.Printf("    • %s config entry (%s)\n", name, hs.ConfigPath)
			}
		}
	}

	if w.Mode == "everything" || w.Mode == "server" {
		if state.Server != nil {
			fmt.Println("    • Docker containers")
			fmt.Println("    • Server state entry")
		}
	}

	fmt.Println()

	idx := ui.PlainSelect(
		"What would you like to do?",
		[]string{"Remove now", "Go back", "Cancel"},
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
// Step 3: Execute
// ---------------------------------------------------------------------------

func stepRemoveExecute(w *Wizard) StepResult {
	state, err := core.LoadState()
	if err != nil || state == nil {
		ui.Error("Could not load state: %v", err)
		return StepQuit
	}

	fmt.Println()
	ui.Title("Removing...")
	fmt.Println()

	// Remove editor configs.
	if w.Mode == "everything" || w.Mode == "editors" {
		for id, hs := range state.Harnesses {
			h := core.FromID(id)
			name := id
			if h != nil {
				name = h.Name
			}

			// Manual-install harnesses cannot be auto-removed.
			if h != nil && h.Method == core.InstallMethodPrintInstructions {
				ui.Warn("%-22s cannot auto-remove (manual setup)", name)
				continue
			}

			if hs.ConfigPath == "" {
				ui.Warn("%-22s no config path recorded — skipping", name)
				continue
			}

			data, readErr := os.ReadFile(hs.ConfigPath)
			if readErr != nil {
				if os.IsNotExist(readErr) {
					// Config file already gone — just clean up state.
					delete(state.Harnesses, id)
					ui.Info("%-22s config file already absent", name)
					continue
				}
				ui.Warn("%-22s %v", name, readErr)
				continue
			}

			// Determine config shape: use the harness definition when available,
			// fall back to the default JSON-map shape.
			shape := core.ConfigShape{
				Kind:      core.ConfigShapeJsonMap,
				Container: "mcpServers",
				TypeField: core.TypeFieldNone,
				UrlField:  "url",
			}
			if h != nil && h.Shape != nil {
				shape = *h.Shape
			}

			updated, removeErr := core.RemoveServerConfig(data, shape, "engrammic")
			if removeErr != nil {
				ui.Error("%-22s %v", name, removeErr)
				continue
			}

			tmpPath := hs.ConfigPath + ".tmp"
			if writeErr := os.WriteFile(tmpPath, updated, 0o644); writeErr != nil {
				ui.Error("%-22s %v", name, writeErr)
				continue
			}
			if renameErr := os.Rename(tmpPath, hs.ConfigPath); renameErr != nil {
				_ = os.Remove(tmpPath)
				ui.Error("%-22s %v", name, renameErr)
				continue
			}

			delete(state.Harnesses, id)
			ui.Success("%-22s removed", name)
		}
	}

	// Remove server state.
	if w.Mode == "everything" || w.Mode == "server" {
		if state.Server != nil {
			state.Server = nil
			ui.Success("%-22s cleared", "Server state")
		}
	}

	// Persist updated state.
	if saveErr := state.Save(); saveErr != nil {
		ui.Warn("Could not save state: %v", saveErr)
	}

	fmt.Println()
	ui.Success("Done!")

	return StepNext
}

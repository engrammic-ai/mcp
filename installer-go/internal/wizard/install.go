// install.go — Cloud install wizard: Mode → Editors → Skills → Review → Execute.
package wizard

import (
	"fmt"

	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

// DefaultCloudEndpoint is the production Engrammic MCP endpoint.
const DefaultCloudEndpoint = "https://beta.engrammic.ai/mcp/"

// CloudInstallWizard returns a Wizard wired with all five cloud-install steps.
func CloudInstallWizard() *Wizard {
	return New("Engrammic Installer", []Step{
		{Name: "Mode", Run: stepMode},
		{Name: "Editors", Run: stepEditors},
		{Name: "Skills", Run: stepSkills},
		{Name: "Review", Run: stepReview},
		{Name: "Execute", Run: stepExecute},
	})
}

// ---------------------------------------------------------------------------
// Step 1: Mode
// ---------------------------------------------------------------------------

func stepMode(w *Wizard) StepResult {
	idx := ui.PlainSelect(
		"How do you want to connect to Engrammic?",
		[]string{
			"Cloud (recommended) — connect to engrammic.ai",
			"Self-hosted — run your own server with Docker",
		},
		0,
	)

	switch idx {
	case 0:
		w.Mode = "cloud"
		w.Endpoint = DefaultCloudEndpoint
	default:
		w.Mode = "selfhost"
	}
	return StepNext
}

// ---------------------------------------------------------------------------
// Step 2: Editors
// ---------------------------------------------------------------------------

func stepEditors(w *Wizard) StepResult {
	if w.Mode == "selfhost" {
		return StepNext // selfhost wizard handles editors differently
	}

	// Populate harness choices on first visit.
	if len(w.Harnesses) == 0 {
		detected := platform.DetectEditors()
		detectedIDs := make(map[string]bool, len(detected))
		for _, h := range detected {
			detectedIDs[h.ID] = true
		}

		for _, h := range core.AllHarnesses() {
			// Skip project-level (relative path) and PrintInstructions-only harnesses
			// for the primary cloud wizard — users can configure those manually.
			if !isAbsPath(h.ConfigPath) {
				continue
			}
			if h.Method == core.InstallMethodPrintInstructions {
				continue
			}
			w.Harnesses = append(w.Harnesses, HarnessChoice{
				Harness:  h,
				Method:   DefaultMethod(h),
				Selected: detectedIDs[h.ID],
			})
		}
	}

	// Build display names and current selection state.
	names := make([]string, len(w.Harnesses))
	selected := make([]bool, len(w.Harnesses))
	for i, hc := range w.Harnesses {
		names[i] = hc.Harness.Name
		selected[i] = hc.Selected
	}

	fmt.Println()
	ui.Title("Select editors to configure")
	selected = ui.PlainMultiSelect("", names, selected)

	for i := range w.Harnesses {
		w.Harnesses[i].Selected = selected[i]
	}

	// For editors that support both file and deeplink, ask which method to use.
	for i := range w.Harnesses {
		hc := &w.Harnesses[i]
		if !hc.Selected {
			continue
		}
		if hc.Harness.DeepLink != nil && hc.Harness.Shape != nil {
			method := askInstallMethod(hc.Harness)
			if method == "" {
				return StepBack
			}
			hc.Method = method
		}
	}

	return StepNext
}

// ---------------------------------------------------------------------------
// Step 3: Skills
// ---------------------------------------------------------------------------

func stepSkills(w *Wizard) StepResult {
	if w.Mode == "selfhost" {
		return StepNext
	}

	// Populate skill choices on first visit.
	if len(w.Skills) == 0 {
		for _, dest := range core.AllSkillDests() {
			w.Skills = append(w.Skills, SkillChoice{
				Dest:     dest,
				Selected: dest.Default,
			})
		}
	}

	names := make([]string, len(w.Skills))
	selected := make([]bool, len(w.Skills))
	for i, sc := range w.Skills {
		label := sc.Dest.Name
		if sc.Dest.Note != nil {
			label += fmt.Sprintf(" (%s)", *sc.Dest.Note)
		}
		names[i] = label
		selected[i] = sc.Selected
	}

	fmt.Println()
	ui.Title("Install Engrammic skills")
	selected = ui.PlainMultiSelect("", names, selected)

	for i := range w.Skills {
		w.Skills[i].Selected = selected[i]
	}

	return StepNext
}

// ---------------------------------------------------------------------------
// Step 4: Review
// ---------------------------------------------------------------------------

func stepReview(w *Wizard) StepResult {
	fmt.Println()
	ui.Title("Ready to install")
	fmt.Println()
	fmt.Printf("  Endpoint:  %s\n", w.Endpoint)
	fmt.Println()

	selectedHarnesses := w.SelectedHarnesses()
	if len(selectedHarnesses) == 0 {
		ui.Warn("No editors selected. Go back to select at least one editor.")
	} else {
		fmt.Println("  Editors:")
		for _, hc := range selectedHarnesses {
			fmt.Printf("    %-22s %s\n", hc.Harness.Name, hc.Method)
		}
	}
	fmt.Println()

	selectedSkills := w.SelectedSkills()
	if len(selectedSkills) > 0 {
		fmt.Print("  Skills:    ")
		for i, sc := range selectedSkills {
			if i > 0 {
				fmt.Print(", ")
			}
			fmt.Print(sc.Dest.Name)
		}
		fmt.Println()
		fmt.Println()
	}

	idx := ui.PlainSelect(
		"What would you like to do?",
		[]string{"Install now", "Go back", "Cancel"},
		0,
	)

	switch idx {
	case 0:
		if len(selectedHarnesses) == 0 {
			ui.Warn("Select at least one editor before installing.")
			return StepStay
		}
		return StepNext
	case 1:
		return StepBack
	default:
		return StepQuit
	}
}

// ---------------------------------------------------------------------------
// Step 5: Execute
// ---------------------------------------------------------------------------

func stepExecute(w *Wizard) StepResult {
	selected := w.SelectedHarnesses()
	if len(selected) == 0 {
		ui.Warn("No editors selected")
		return StepBack
	}

	// Build the name list for the progress display.
	names := make([]string, 0, len(selected))
	for _, hc := range selected {
		names = append(names, hc.Harness.Name)
	}
	skillsSelected := w.SelectedSkills()
	if len(skillsSelected) > 0 {
		names = append(names, "Skills")
	}

	progress := ui.NewProgressList(names)

	fmt.Println()
	ui.Title("Installing...")
	fmt.Println(progress.Render())

	// Run a background ticker to animate the spinner while installs run.
	lineCount := len(names) + 1
	stop := progress.StartTicker(func() {
		fmt.Printf("\033[%dA", lineCount)
		fmt.Println(progress.Render())
	})

	results := ExecuteConfigs(selected, w.Endpoint, progress)

	// Mark skills as done (actual file writing is handled separately elsewhere).
	if len(skillsSelected) > 0 {
		progress.SetStatus("Skills", ui.StatusDone, "installed")
	}

	stop()

	// Final render (move cursor up and redraw so the last frame is clean).
	fmt.Printf("\033[%dA", lineCount)
	fmt.Println(progress.Render())

	// Persist state.
	if err := UpdateState(w, results); err != nil {
		ui.Warn("Could not save state: %v", err)
	}

	fmt.Println()
	ui.Success("Done! Your editors are now connected to Engrammic.")

	return StepNext
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

// DefaultMethod returns the string method name based on a harness's primary
// InstallMethod.  Harnesses that support both file and deeplink (i.e. have
// both Shape and DeepLink set) default to deeplink.
// Exported so the CLI non-interactive path can reuse it.
func DefaultMethod(h core.Harness) string {
	switch h.Method {
	case core.InstallMethodDeepLink:
		return "deeplink"
	case core.InstallMethodPrintInstructions:
		return "manual"
	default:
		return "file"
	}
}

// askInstallMethod prompts the user to pick between file-edit and deeplink
// for a harness that supports both.  Returns "" if the user cancels.
func askInstallMethod(h core.Harness) string {
	idx := ui.PlainSelect(
		fmt.Sprintf("How do you want to configure %s?", h.Name),
		[]string{
			fmt.Sprintf("Edit config file (%s)", h.ConfigPath),
			"Open in editor via deeplink",
		},
		0,
	)
	switch idx {
	case 0:
		return "file"
	case 1:
		return "deeplink"
	default:
		return ""
	}
}

// isAbsPath reports whether a path starts with '/' (Unix) or a drive letter
// (Windows).  Used to filter out project-relative config paths.
func isAbsPath(p string) bool {
	if len(p) == 0 {
		return false
	}
	if p[0] == '/' || p[0] == '\\' {
		return true
	}
	// Windows: C:\...
	if len(p) >= 3 && p[1] == ':' {
		return true
	}
	return false
}

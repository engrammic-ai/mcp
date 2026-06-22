package wizard

import (
	"fmt"
	"os"
	"time"

	"github.com/charmbracelet/huh"

	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/platform"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

const DefaultCloudEndpoint = "https://beta.engrammic.ai/mcp/"

func CloudInstallWizard() *Wizard {
	return New("Engrammic Installer", []Step{
		{Name: "Mode", Run: stepMode},
		{Name: "Editors", Run: stepEditors},
		{Name: "Skills", Run: stepSkills},
		{Name: "Review", Run: stepReview},
		{Name: "Execute", Run: stepExecute},
	})
}

func stepMode(w *Wizard) StepResult {
	var mode string

	form := huh.NewForm(
		huh.NewGroup(
			huh.NewSelect[string]().
				Title("How do you want to connect to Engrammic?").
				Options(
					huh.NewOption("Cloud (recommended)", "cloud"),
					huh.NewOption("Self-hosted", "selfhost"),
				).
				Value(&mode),
		),
	)

	if err := form.Run(); err != nil {
		return StepQuit
	}

	w.Mode = mode
	if mode == "cloud" {
		w.Endpoint = DefaultCloudEndpoint
	}
	return StepNext
}

func stepEditors(w *Wizard) StepResult {
	if w.Mode == "selfhost" {
		return StepNext
	}

	if len(w.Harnesses) == 0 {
		// Load existing state to know what's already configured
		state, _ := core.LoadState()
		configuredIDs := make(map[string]bool)
		hasExistingConfig := false
		if state != nil && len(state.Harnesses) > 0 {
			hasExistingConfig = true
			for id := range state.Harnesses {
				configuredIDs[id] = true
			}
		}

		detected := platform.DetectEditors()
		detectedIDs := make(map[string]bool, len(detected))
		for _, h := range detected {
			detectedIDs[h.ID] = true
		}

		for _, h := range core.AllHarnesses() {
			if !isAbsPath(h.ConfigPath) {
				continue
			}
			if h.Method == core.InstallMethodPrintInstructions {
				continue
			}
			// If we have existing config, only pre-select those
			// Otherwise (fresh install), pre-select detected editors
			var preSelect bool
			if hasExistingConfig {
				preSelect = configuredIDs[h.ID]
			} else {
				preSelect = detectedIDs[h.ID]
			}
			w.Harnesses = append(w.Harnesses, HarnessChoice{
				Harness:  h,
				Method:   DefaultMethod(h),
				Selected: preSelect,
			})
		}
	}

	// Load state again to show which are configured
	state, _ := core.LoadState()
	configuredIDs := make(map[string]bool)
	if state != nil {
		for id := range state.Harnesses {
			configuredIDs[id] = true
		}
	}

	options := make([]huh.Option[string], len(w.Harnesses))
	for i, hc := range w.Harnesses {
		label := hc.Harness.Name
		if configuredIDs[hc.Harness.ID] {
			label += " (configured)"
		}
		options[i] = huh.NewOption(label, hc.Harness.ID)
	}

	var selected []string
	for _, hc := range w.Harnesses {
		if hc.Selected {
			selected = append(selected, hc.Harness.ID)
		}
	}

	form := huh.NewForm(
		huh.NewGroup(
			huh.NewMultiSelect[string]().
				Title("Select editors to configure").
				Options(options...).
				Value(&selected),
		),
	)

	if err := form.Run(); err != nil {
		return StepBack
	}

	selectedMap := make(map[string]bool)
	for _, id := range selected {
		selectedMap[id] = true
	}
	for i := range w.Harnesses {
		w.Harnesses[i].Selected = selectedMap[w.Harnesses[i].Harness.ID]
	}

	// For dual-method editors, ask which method
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

func stepSkills(w *Wizard) StepResult {
	if w.Mode == "selfhost" {
		return StepNext
	}

	if len(w.Skills) == 0 {
		for _, dest := range core.AllSkillDests() {
			w.Skills = append(w.Skills, SkillChoice{
				Dest:     dest,
				Selected: dest.Default,
			})
		}
	}

	options := make([]huh.Option[string], len(w.Skills))
	for i, sc := range w.Skills {
		label := sc.Dest.Name
		if sc.Dest.Note != nil {
			label += fmt.Sprintf(" (%s)", *sc.Dest.Note)
		}
		options[i] = huh.NewOption(label, sc.Dest.Path)
	}

	var selected []string
	for _, sc := range w.Skills {
		if sc.Selected {
			selected = append(selected, sc.Dest.Path)
		}
	}

	form := huh.NewForm(
		huh.NewGroup(
			huh.NewMultiSelect[string]().
				Title("Install Engrammic skills").
				Options(options...).
				Value(&selected),
		),
	)

	if err := form.Run(); err != nil {
		return StepBack
	}

	selectedMap := make(map[string]bool)
	for _, path := range selected {
		selectedMap[path] = true
	}
	for i := range w.Skills {
		w.Skills[i].Selected = selectedMap[w.Skills[i].Dest.Path]
	}

	return StepNext
}

func stepReview(w *Wizard) StepResult {
	fmt.Println()
	ui.Title("Ready to install")
	fmt.Println()
	fmt.Printf("  Endpoint:  %s\n", w.Endpoint)
	fmt.Println()

	selectedHarnesses := w.SelectedHarnesses()
	if len(selectedHarnesses) == 0 {
		ui.Warn("No editors selected.")
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

	var action string
	form := huh.NewForm(
		huh.NewGroup(
			huh.NewSelect[string]().
				Title("").
				Options(
					huh.NewOption("Install now", "install"),
					huh.NewOption("Go back", "back"),
					huh.NewOption("Cancel", "cancel"),
				).
				Value(&action),
		),
	)

	if err := form.Run(); err != nil {
		return StepQuit
	}

	switch action {
	case "install":
		if len(selectedHarnesses) == 0 {
			ui.Warn("Select at least one editor.")
			return StepStay
		}
		return StepNext
	case "back":
		return StepBack
	default:
		return StepQuit
	}
}

func stepExecute(w *Wizard) StepResult {
	selected := w.SelectedHarnesses()

	// Load current state to find what needs to be removed
	state, _ := core.LoadState()
	configuredIDs := make(map[string]bool)
	if state != nil {
		for id := range state.Harnesses {
			configuredIDs[id] = true
		}
	}

	// Build set of selected IDs
	selectedIDs := make(map[string]bool)
	for _, hc := range selected {
		selectedIDs[hc.Harness.ID] = true
	}

	fmt.Println()
	ui.Title("Configuring...")

	// Remove engrammic from deselected harnesses that were previously configured
	for _, hc := range w.Harnesses {
		if !hc.Selected && configuredIDs[hc.Harness.ID] {
			if err := removeConfig(hc.Harness); err != nil {
				ui.Warn("%-22s could not remove: %v", hc.Harness.Name, err)
			} else {
				ui.Info("%-22s removed", hc.Harness.Name)
			}
		}
	}

	// Configure selected editors
	for _, hc := range selected {
		result := executeSingleConfig(hc, w.Endpoint)
		if result.Error != nil {
			ui.Error("%-22s %v", hc.Harness.Name, result.Error)
		} else {
			ui.Success("%-22s %s", hc.Harness.Name, result.Detail)
		}
	}

	// Skills (placeholder - actual writing not implemented yet)
	skillsSelected := w.SelectedSkills()
	if len(skillsSelected) > 0 {
		ui.Success("%-22s installed", "Skills")
	}

	// Update state - only include selected harnesses
	newState := &core.State{
		Version:     1,
		LastUpdated: time.Now(),
		Harnesses:   make(map[string]core.HarnessState),
	}
	if state != nil && state.Server != nil {
		newState.Server = state.Server
	}
	for _, hc := range selected {
		newState.Harnesses[hc.Harness.ID] = core.HarnessState{
			InstalledAt: time.Now(),
			ConfigPath:  platform.ExpandPath(hc.Harness.ConfigPath),
			Endpoint:    w.Endpoint,
		}
	}
	if err := newState.Save(); err != nil {
		ui.Warn("Could not save state: %v", err)
	}

	fmt.Println()
	ui.Success("Done!")

	return StepNext
}

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

func askInstallMethod(h core.Harness) string {
	var method string

	form := huh.NewForm(
		huh.NewGroup(
			huh.NewSelect[string]().
				Title(fmt.Sprintf("How do you want to configure %s?", h.Name)).
				Options(
					huh.NewOption(fmt.Sprintf("Edit config file (%s)", h.ConfigPath), "file"),
					huh.NewOption("Open in editor via deeplink", "deeplink"),
				).
				Value(&method),
		),
	)

	if err := form.Run(); err != nil {
		return ""
	}
	return method
}

func isAbsPath(p string) bool {
	if len(p) == 0 {
		return false
	}
	if p[0] == '/' || p[0] == '\\' {
		return true
	}
	if len(p) >= 3 && p[1] == ':' {
		return true
	}
	return false
}

// removeConfig removes the engrammic entry from a harness config file.
func removeConfig(h core.Harness) error {
	if h.Shape == nil {
		return nil // Nothing to remove for non-file harnesses
	}

	configPath := platform.ExpandPath(h.ConfigPath)
	data, err := os.ReadFile(configPath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil // Config doesn't exist, nothing to remove
		}
		return err
	}

	updated, err := core.RemoveServerConfig(data, *h.Shape, "engrammic")
	if err != nil {
		return err
	}

	return os.WriteFile(configPath, updated, 0644)
}

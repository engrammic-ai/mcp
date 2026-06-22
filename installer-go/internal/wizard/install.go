package wizard

import (
	"fmt"

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
			w.Harnesses = append(w.Harnesses, HarnessChoice{
				Harness:  h,
				Method:   DefaultMethod(h),
				Selected: detectedIDs[h.ID],
			})
		}
	}

	options := make([]huh.Option[string], len(w.Harnesses))
	for i, hc := range w.Harnesses {
		options[i] = huh.NewOption(hc.Harness.Name, hc.Harness.ID)
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
	if len(selected) == 0 {
		ui.Warn("No editors selected")
		return StepBack
	}

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

	lineCount := len(names) + 1
	stop := progress.StartTicker(func() {
		fmt.Printf("\033[%dA", lineCount)
		fmt.Println(progress.Render())
	})

	results := ExecuteConfigs(selected, w.Endpoint, progress)

	if len(skillsSelected) > 0 {
		progress.SetStatus("Skills", ui.StatusDone, "installed")
	}

	stop()

	fmt.Printf("\033[%dA", lineCount)
	fmt.Println(progress.Render())

	if err := UpdateState(w, results); err != nil {
		ui.Warn("Could not save state: %v", err)
	}

	fmt.Println()
	ui.Success("Done! Your editors are now connected to Engrammic.")

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

// wizard.go — step machine for install/selfhost/remove flows.
package wizard

import (
	"fmt"

	"github.com/anthropics/engrammic/installer/internal/core"
)

// StepResult indicates what the wizard should do after a step runs.
type StepResult int

const (
	StepNext StepResult = iota // advance to next step
	StepBack                   // go back one step
	StepQuit                   // cancel wizard
	StepStay                   // re-render current step (validation failure, etc.)
)

// Step is a single wizard screen with a name and a run function.
type Step struct {
	Name string
	Run  func(w *Wizard) StepResult
}

// HarnessChoice holds a harness and the user's chosen installation method.
type HarnessChoice struct {
	Harness  core.Harness
	Method   string // "file", "deeplink", "manual"
	Selected bool
}

// SkillChoice holds a skill destination and whether it was selected.
type SkillChoice struct {
	Dest     core.SkillDest
	Selected bool
}

// Wizard is the shared state carrier and step machine for all wizard flows.
type Wizard struct {
	Steps   []Step
	Current int
	Title   string

	// Shared state
	Mode     string // "cloud" or "selfhost"
	Endpoint string
	Harnesses []HarnessChoice
	Skills    []SkillChoice

	// Selfhost-specific
	Runtime        string // "docker" or "podman"
	Tier           string // "standalone" or "cloud"
	LLMProvider    core.LlmProviderConfig
	EmbedProvider  core.EmbeddingProviderConfig
	Reranker       core.RerankerProviderConfig
	Port           int
	Credentials    map[string]string
	License        string
}

// New creates a Wizard with the given title and steps, with sensible defaults.
func New(title string, steps []Step) *Wizard {
	return &Wizard{
		Steps:       steps,
		Title:       title,
		Credentials: make(map[string]string),
		Port:        8000,
	}
}

// Run drives the step machine until all steps complete or the wizard is cancelled.
func (w *Wizard) Run() error {
	for w.Current < len(w.Steps) {
		step := w.Steps[w.Current]

		result := step.Run(w)
		switch result {
		case StepNext:
			w.Current++
		case StepBack:
			if w.Current > 0 {
				w.Current--
			}
		case StepQuit:
			return fmt.Errorf("wizard cancelled")
		case StepStay:
			// Loop without advancing
		}
	}
	return nil
}

// StepHeader returns a formatted header string for the current step.
func (w *Wizard) StepHeader() string {
	return fmt.Sprintf("%s  Step %d/%d", w.Title, w.Current+1, len(w.Steps))
}

// SelectedHarnesses returns only the harnesses the user chose to install.
func (w *Wizard) SelectedHarnesses() []HarnessChoice {
	var result []HarnessChoice
	for _, h := range w.Harnesses {
		if h.Selected {
			result = append(result, h)
		}
	}
	return result
}

// SelectedSkills returns only the skill destinations the user chose to install.
func (w *Wizard) SelectedSkills() []SkillChoice {
	var result []SkillChoice
	for _, s := range w.Skills {
		if s.Selected {
			result = append(result, s)
		}
	}
	return result
}

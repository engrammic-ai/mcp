// skills.go — "engrammic skills" command.
package cli

import (
	"fmt"

	"github.com/spf13/cobra"
	"github.com/anthropics/engrammic/installer/internal/core"
	"github.com/anthropics/engrammic/installer/internal/ui"
)

var skillsCmd = &cobra.Command{
	Use:   "skills",
	Short: "Install/manage skills",
	Run:   runSkills,
}

var skillsListCmd = &cobra.Command{
	Use:   "list",
	Short: "List available skill destinations",
	Run:   runSkillsList,
}

func init() {
	skillsCmd.AddCommand(skillsListCmd)
	RootCmd.AddCommand(skillsCmd)
}

func runSkills(cmd *cobra.Command, args []string) {
	ui.Info("Use 'engrammic skills list' to see available destinations")
	ui.Info("Skills are installed via the install wizard")
}

func runSkillsList(cmd *cobra.Command, args []string) {
	ui.Title("Skill Destinations")
	fmt.Println()

	fmt.Println("User-level (global):")
	for _, d := range core.AllSkillDests() {
		if d.Scope == core.SkillScopeUser {
			fmt.Printf("  %-18s %s\n", d.Name, d.Path)
		}
	}

	fmt.Println()
	fmt.Println("Project-level:")
	for _, d := range core.AllSkillDests() {
		if d.Scope == core.SkillScopeProject {
			fmt.Printf("  %-18s %s\n", d.Name, d.Path)
		}
	}
}

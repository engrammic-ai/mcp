package core

import (
	"path/filepath"
	"strings"
	"testing"
)

func TestAllSkillDestsNotEmpty(t *testing.T) {
	dests := AllSkillDests()
	if len(dests) == 0 {
		t.Fatal("AllSkillDests returned empty slice")
	}
}

func TestUserLevelDestsScope(t *testing.T) {
	user := UserLevelDests()
	if len(user) == 0 {
		t.Fatal("UserLevelDests returned empty slice")
	}
	for _, d := range user {
		if d.Scope != SkillScopeUser {
			t.Errorf("UserLevelDests: %q has scope %d, want SkillScopeUser", d.Name, d.Scope)
		}
	}
}

func TestProjectLevelDestsScope(t *testing.T) {
	project := ProjectLevelDests()
	if len(project) == 0 {
		t.Fatal("ProjectLevelDests returned empty slice")
	}
	for _, d := range project {
		if d.Scope != SkillScopeProject {
			t.Errorf("ProjectLevelDests: %q has scope %d, want SkillScopeProject", d.Name, d.Scope)
		}
	}
}

func TestProjectDestsAreRelative(t *testing.T) {
	for _, d := range ProjectLevelDests() {
		if filepath.IsAbs(d.Path) {
			t.Errorf("project dest %q has absolute path %q, want relative", d.Name, d.Path)
		}
	}
}

func TestUserDestsAreAbsolute(t *testing.T) {
	for _, d := range UserLevelDests() {
		if !filepath.IsAbs(d.Path) {
			t.Errorf("user dest %q has relative path %q, want absolute", d.Name, d.Path)
		}
	}
}

func TestClaudeDestPath(t *testing.T) {
	dests := AllSkillDests()
	var claude *SkillDest
	for i := range dests {
		if dests[i].Harness == "claude" {
			claude = &dests[i]
			break
		}
	}
	if claude == nil {
		t.Fatal("no claude harness dest found")
	}
	if !strings.HasSuffix(claude.Path, ".claude/skills") {
		t.Errorf("claude path %q should end with .claude/skills", claude.Path)
	}
}

func TestTotalDestCount(t *testing.T) {
	all := AllSkillDests()
	user := UserLevelDests()
	project := ProjectLevelDests()
	if len(user)+len(project) != len(all) {
		t.Errorf("user(%d) + project(%d) != all(%d)", len(user), len(project), len(all))
	}
}

func TestDefaultDestsSubsetOfAll(t *testing.T) {
	defaults := DefaultDests()
	all := AllSkillDests()
	allByHarness := make(map[string]bool)
	for _, d := range all {
		allByHarness[d.Harness+":"+d.Name] = true
	}
	for _, d := range defaults {
		key := d.Harness + ":" + d.Name
		if !allByHarness[key] {
			t.Errorf("default dest %q not found in AllSkillDests", d.Name)
		}
	}
}

func TestCrossHarnessNote(t *testing.T) {
	for _, d := range AllSkillDests() {
		if d.Harness == "cross" {
			if d.Note == nil {
				t.Fatal("cross-harness dest should have a note")
			}
			if !strings.Contains(*d.Note, "Pi Agents") {
				t.Errorf("cross-harness note %q should mention Pi Agents", *d.Note)
			}
			return
		}
	}
	t.Fatal("no cross-harness dest found")
}

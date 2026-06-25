package core

import (
	"strings"
	"testing"
)

func TestConvertToMdc(t *testing.T) {
	input := `---
name: engrammic:leap-guide
description: Test skill
---

# LeAP Guide

Some content here.`

	expected := `# LeAP Guide

Some content here.`

	result := convertToMdc(input)
	if result != expected {
		t.Errorf("convertToMdc mismatch:\ngot:\n%s\nwant:\n%s", result, expected)
	}
}

func TestRemoveEngrammicSections(t *testing.T) {
	input := `# My File

Some existing content.

<!-- engrammic:start -->
<!-- Auto-generated -->
Old skill content
<!-- engrammic:end -->

More content.`

	result := removeEngrammicSections(input)

	// Check that engrammic markers are removed
	if strings.Contains(result, "engrammic:start") {
		t.Error("result should not contain engrammic:start")
	}
	if strings.Contains(result, "Old skill content") {
		t.Error("result should not contain old skill content")
	}
	if !strings.Contains(result, "My File") {
		t.Error("result should contain My File")
	}
	if !strings.Contains(result, "More content") {
		t.Error("result should contain More content")
	}
}

func TestFetchSkills(t *testing.T) {
	if testing.Short() {
		t.Skip("skipping network test in short mode")
	}

	skills, err := FetchSkills()
	if err != nil {
		t.Fatalf("FetchSkills: %v", err)
	}

	if len(skills) == 0 {
		t.Error("expected at least one skill")
	}

	// Check that we got the leap-guide
	found := false
	for _, s := range skills {
		if strings.Contains(s.Name, "leap-guide") {
			found = true
			if !strings.Contains(s.Content, "LeAP") {
				t.Error("leap-guide content should contain 'LeAP'")
			}
		}
	}
	if !found {
		t.Error("expected to find leap-guide skill")
	}
}

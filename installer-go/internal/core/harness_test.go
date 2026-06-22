package core

import (
	"strings"
	"testing"
)

func TestAllHarnesses_MinimumCount(t *testing.T) {
	all := AllHarnesses()
	if len(all) < 25 {
		t.Errorf("expected at least 25 harnesses, got %d", len(all))
	}
}

func TestAllHarnesses_UniqueIDs(t *testing.T) {
	seen := make(map[string]bool)
	for _, h := range AllHarnesses() {
		if seen[h.ID] {
			t.Errorf("duplicate harness ID: %q", h.ID)
		}
		seen[h.ID] = true
	}
}

func TestFromID_KnownHarnesses(t *testing.T) {
	cases := []string{"claude", "claude-desktop", "cursor", "vscode", "windsurf", "gemini", "codex", "goose"}
	for _, id := range cases {
		h := FromID(id)
		if h == nil {
			t.Errorf("FromID(%q) returned nil", id)
			continue
		}
		if h.ID != id {
			t.Errorf("FromID(%q).ID = %q", id, h.ID)
		}
	}
}

func TestFromID_Unknown(t *testing.T) {
	if h := FromID("nonexistent-harness"); h != nil {
		t.Errorf("expected nil for unknown id, got %+v", h)
	}
}

func TestValidIDs_ContainsExpected(t *testing.T) {
	ids := ValidIDs()
	for _, expected := range []string{"claude", "cursor", "vscode", "gemini"} {
		if !strings.Contains(ids, expected) {
			t.Errorf("ValidIDs() missing %q", expected)
		}
	}
}

func TestHarnesses_MethodConsistency(t *testing.T) {
	for _, h := range AllHarnesses() {
		switch h.Method {
		case InstallMethodFileEdit:
			if h.Shape == nil {
				t.Errorf("harness %q: FileEdit method requires non-nil Shape", h.ID)
			}
			if h.DeepLink != nil {
				t.Errorf("harness %q: FileEdit method should have nil DeepLink", h.ID)
			}
			if h.ConfigPath == "" {
				t.Errorf("harness %q: FileEdit method requires non-empty ConfigPath", h.ID)
			}
		case InstallMethodDeepLink:
			if h.DeepLink == nil {
				t.Errorf("harness %q: DeepLink method requires non-nil DeepLink", h.ID)
			}
			if h.Shape != nil {
				t.Errorf("harness %q: DeepLink method should have nil Shape", h.ID)
			}
		case InstallMethodPrintInstructions:
			if h.Instructions == "" {
				t.Errorf("harness %q: PrintInstructions method requires non-empty Instructions", h.ID)
			}
			if h.Shape != nil {
				t.Errorf("harness %q: PrintInstructions method should have nil Shape", h.ID)
			}
		}
	}
}

func TestTypeField_Value(t *testing.T) {
	cases := []struct {
		tf   TypeField
		want string
	}{
		{TypeFieldNone, ""},
		{TypeFieldHttp, "http"},
		{TypeFieldStreamableHttp, "streamable-http"},
		{TypeFieldRemote, "remote"},
	}
	for _, c := range cases {
		if got := c.tf.Value(); got != c.want {
			t.Errorf("TypeField(%d).Value() = %q, want %q", c.tf, got, c.want)
		}
	}
}

func TestNewHarnesses_VSCodeFile(t *testing.T) {
	h := FromID("vscode-file")
	if h == nil {
		t.Fatal("vscode-file harness not found")
	}
	if h.Method != InstallMethodFileEdit {
		t.Errorf("vscode-file should use FileEdit, got %d", h.Method)
	}
	if h.Shape == nil || h.Shape.Kind != ConfigShapeVSCodeJson {
		t.Errorf("vscode-file should use ConfigShapeVSCodeJson")
	}
}

func TestNewHarnesses_WindsurfDeepLink(t *testing.T) {
	h := FromID("windsurf-dl")
	if h == nil {
		t.Fatal("windsurf-dl harness not found")
	}
	if h.Method != InstallMethodDeepLink {
		t.Errorf("windsurf-dl should use DeepLink, got %d", h.Method)
	}
	if h.DeepLink == nil || *h.DeepLink != DeepLinkWindsurf {
		t.Errorf("windsurf-dl should use DeepLinkWindsurf")
	}
}

func TestDetectInstalled_ReturnsSlice(t *testing.T) {
	// Just verify the function runs without panicking and returns a subset.
	installed := DetectInstalled()
	all := AllHarnesses()
	if len(installed) > len(all) {
		t.Errorf("DetectInstalled() returned %d, more than AllHarnesses() %d", len(installed), len(all))
	}
}

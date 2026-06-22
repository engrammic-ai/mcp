package platform

import (
	"os"
	"testing"
)

func TestIsDumb(t *testing.T) {
	// Save and restore env
	origTerm := os.Getenv("TERM")
	origNoColor := os.Getenv("NO_COLOR")
	defer func() {
		os.Setenv("TERM", origTerm)
		if origNoColor == "" {
			os.Unsetenv("NO_COLOR")
		} else {
			os.Setenv("NO_COLOR", origNoColor)
		}
	}()

	os.Setenv("TERM", "dumb")
	os.Unsetenv("NO_COLOR")
	if !IsDumb() {
		t.Error("expected IsDumb() true for TERM=dumb")
	}

	os.Setenv("TERM", "xterm-256color")
	os.Setenv("NO_COLOR", "1")
	if !IsDumb() {
		t.Error("expected IsDumb() true for NO_COLOR")
	}

	os.Setenv("TERM", "xterm-256color")
	os.Unsetenv("NO_COLOR")
	if IsDumb() {
		t.Error("expected IsDumb() false for normal terminal")
	}
}

func TestUseRichUI_NonTTY(t *testing.T) {
	// In test environment, stdout is typically not a TTY, so UseRichUI() should be false
	// unless both IsTTY() and !IsDumb() are true.
	origTerm := os.Getenv("TERM")
	origNoColor := os.Getenv("NO_COLOR")
	defer func() {
		os.Setenv("TERM", origTerm)
		if origNoColor == "" {
			os.Unsetenv("NO_COLOR")
		} else {
			os.Setenv("NO_COLOR", origNoColor)
		}
	}()

	// Force dumb terminal — UseRichUI must be false regardless of TTY state
	os.Setenv("TERM", "dumb")
	os.Unsetenv("NO_COLOR")
	if UseRichUI() {
		t.Error("expected UseRichUI() false when terminal is dumb")
	}
}

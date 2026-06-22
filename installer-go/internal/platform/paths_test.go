package platform

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestExpandPath_Tilde(t *testing.T) {
	home, err := os.UserHomeDir()
	if err != nil {
		t.Skip("cannot determine home dir")
	}

	got := ExpandPath("~/foo/bar")
	want := filepath.Join(home, "foo", "bar")
	if got != want {
		t.Errorf("ExpandPath(~/foo/bar) = %q, want %q", got, want)
	}
}

func TestExpandPath_NoTilde(t *testing.T) {
	input := "/absolute/path"
	if got := ExpandPath(input); got != input {
		t.Errorf("ExpandPath(%q) = %q, want %q", input, got, input)
	}

	input = "relative/path"
	if got := ExpandPath(input); got != input {
		t.Errorf("ExpandPath(%q) = %q, want %q", input, got, input)
	}
}

func TestUserConfigDir(t *testing.T) {
	dir := UserConfigDir()
	if !strings.HasSuffix(dir, ".engrammic") {
		t.Errorf("UserConfigDir() = %q, want path ending in .engrammic", dir)
	}
}

func TestEnsureConfigDir(t *testing.T) {
	if err := EnsureConfigDir(); err != nil {
		t.Fatalf("EnsureConfigDir() error: %v", err)
	}
	info, err := os.Stat(UserConfigDir())
	if err != nil {
		t.Fatalf("config dir does not exist after EnsureConfigDir(): %v", err)
	}
	if !info.IsDir() {
		t.Error("UserConfigDir() path is not a directory")
	}
}

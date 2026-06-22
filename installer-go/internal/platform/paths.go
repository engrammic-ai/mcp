package platform

import (
	"os"
	"path/filepath"
	"strings"
)

// ExpandPath expands a leading "~/" to the user's home directory.
// If the home directory cannot be determined, the path is returned unchanged.
func ExpandPath(p string) string {
	if strings.HasPrefix(p, "~/") {
		home, err := os.UserHomeDir()
		if err != nil {
			return p
		}
		return filepath.Join(home, p[2:])
	}
	return p
}

// UserConfigDir returns the path to the Engrammic user config directory (~/.engrammic).
// Falls back to ".engrammic" if the home directory cannot be determined.
func UserConfigDir() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return ".engrammic"
	}
	return filepath.Join(home, ".engrammic")
}

// EnsureConfigDir creates the user config directory if it does not exist.
func EnsureConfigDir() error {
	return os.MkdirAll(UserConfigDir(), 0755)
}

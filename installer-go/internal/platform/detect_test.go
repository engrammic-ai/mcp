package platform

import (
	"testing"

	"github.com/anthropics/engrammic/installer/internal/core"
)

func TestDetectEditors_ReturnType(t *testing.T) {
	result := DetectEditors()
	// Result may be empty in CI, but must not panic and must be a valid slice.
	for _, h := range result {
		if h.ID == "" {
			t.Error("DetectEditors() returned harness with empty ID")
		}
		if h.ConfigPath == "" {
			t.Error("DetectEditors() returned harness with empty ConfigPath")
		}
	}
}

func TestDetectByTier_All(t *testing.T) {
	all := DetectByTier(0)
	editors := DetectEditors()
	if len(all) != len(editors) {
		t.Errorf("DetectByTier(0) len=%d, DetectEditors() len=%d; should be equal", len(all), len(editors))
	}
}

func TestDetectByTier_FileEdit(t *testing.T) {
	result := DetectByTier(1)
	for _, h := range result {
		if h.Method != core.InstallMethodFileEdit {
			t.Errorf("DetectByTier(1) returned harness %q with method %v, want InstallMethodFileEdit", h.ID, h.Method)
		}
	}
}

func TestDetectByTier_DeepLink(t *testing.T) {
	result := DetectByTier(2)
	for _, h := range result {
		if h.Method != core.InstallMethodDeepLink {
			t.Errorf("DetectByTier(2) returned harness %q with method %v, want InstallMethodDeepLink", h.ID, h.Method)
		}
	}
}

func TestDetectByTier_Unknown(t *testing.T) {
	result := DetectByTier(99)
	if result != nil {
		t.Errorf("DetectByTier(99) = %v, want nil", result)
	}
}

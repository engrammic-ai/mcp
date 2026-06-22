package platform

import "github.com/anthropics/engrammic/installer/internal/core"

// DetectEditors returns harnesses whose config parent directory exists on disk.
// This wraps core.DetectInstalled and only returns user-level (absolute-path) harnesses.
func DetectEditors() []core.Harness {
	return core.DetectInstalled()
}

// DetectByTier returns detected harnesses filtered by install method tier.
//
// Tier mapping (core.Harness has no Tier field; method is the proxy):
//
//	0  — all detected harnesses (no filter)
//	1  — InstallMethodFileEdit  (direct config file edits)
//	2  — InstallMethodDeepLink  (URI-scheme installation)
//	3  — InstallMethodPrintInstructions (manual steps)
func DetectByTier(tier int) []core.Harness {
	detected := DetectEditors()
	if tier == 0 {
		return detected
	}
	var wantMethod core.InstallMethod
	switch tier {
	case 1:
		wantMethod = core.InstallMethodFileEdit
	case 2:
		wantMethod = core.InstallMethodDeepLink
	case 3:
		wantMethod = core.InstallMethodPrintInstructions
	default:
		return nil
	}
	var result []core.Harness
	for _, h := range detected {
		if h.Method == wantMethod {
			result = append(result, h)
		}
	}
	return result
}

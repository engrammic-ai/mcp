package core

import (
	"errors"
	"fmt"
	"net"
	"os/exec"
	"runtime"
	"strings"
)

const DefaultPort = 8000

var ErrNoAvailablePort = errors.New("no available port found")

func IsPortAvailable(port int) bool {
	ln, err := net.Listen("tcp", fmt.Sprintf(":%d", port))
	if err != nil {
		return false
	}
	ln.Close()
	return true
}

func FindAvailablePort(startPort int) (int, error) {
	const maxAttempts = 100
	for i := 0; i < maxAttempts; i++ {
		p := startPort + i
		if IsPortAvailable(p) {
			return p, nil
		}
	}
	return 0, ErrNoAvailablePort
}

// WhoIsUsingPort returns best-effort info about what process is using the port.
// Returns empty string if unable to determine.
func WhoIsUsingPort(port int) string {
	switch runtime.GOOS {
	case "linux":
		return whoIsUsingPortLinux(port)
	case "darwin":
		return whoIsUsingPortDarwin(port)
	case "windows":
		return whoIsUsingPortWindows(port)
	default:
		return ""
	}
}

func whoIsUsingPortLinux(port int) string {
	out, err := exec.Command("ss", "-tlnp", fmt.Sprintf("sport = :%d", port)).Output()
	if err == nil {
		lines := strings.Split(strings.TrimSpace(string(out)), "\n")
		// skip header line
		for _, line := range lines[1:] {
			if strings.Contains(line, fmt.Sprintf(":%d", port)) {
				return strings.TrimSpace(line)
			}
		}
	}
	// fallback: fuser
	out, err = exec.Command("fuser", fmt.Sprintf("%d/tcp", port)).Output()
	if err == nil {
		pid := strings.TrimSpace(string(out))
		if pid != "" {
			return fmt.Sprintf("pid %s", pid)
		}
	}
	return ""
}

func whoIsUsingPortDarwin(port int) string {
	out, err := exec.Command("lsof", "-i", fmt.Sprintf(":%d", port)).Output()
	if err != nil {
		return ""
	}
	lines := strings.Split(strings.TrimSpace(string(out)), "\n")
	for _, line := range lines[1:] {
		if strings.Contains(line, "LISTEN") || strings.Contains(line, fmt.Sprintf(":%d", port)) {
			return strings.TrimSpace(line)
		}
	}
	return ""
}

func whoIsUsingPortWindows(port int) string {
	out, err := exec.Command("netstat", "-ano").Output()
	if err != nil {
		return ""
	}
	portStr := fmt.Sprintf(":%d", port)
	for _, line := range strings.Split(string(out), "\n") {
		if strings.Contains(line, portStr) && strings.Contains(line, "LISTENING") {
			return strings.TrimSpace(line)
		}
	}
	return ""
}

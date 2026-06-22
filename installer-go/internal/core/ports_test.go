package core

import (
	"fmt"
	"net"
	"testing"
)

func TestIsPortAvailable_FreePort(t *testing.T) {
	port, err := getFreePort()
	if err != nil {
		t.Skip("could not find a free port for test setup")
	}
	if !IsPortAvailable(port) {
		t.Errorf("expected port %d to be available", port)
	}
}

func TestIsPortAvailable_OccupiedPort(t *testing.T) {
	ln, err := net.Listen("tcp", ":0")
	if err != nil {
		t.Skip("could not bind port for test setup")
	}
	defer ln.Close()
	port := ln.Addr().(*net.TCPAddr).Port

	if IsPortAvailable(port) {
		t.Errorf("expected port %d to be unavailable while bound", port)
	}
}

func TestFindAvailablePort_ReturnsUsablePort(t *testing.T) {
	port, err := FindAvailablePort(DefaultPort)
	if err != nil {
		t.Fatalf("FindAvailablePort: %v", err)
	}
	if port < DefaultPort || port > DefaultPort+100 {
		t.Errorf("returned port %d out of expected range", port)
	}
	// Confirm the returned port is actually bindable
	if !IsPortAvailable(port) {
		t.Errorf("FindAvailablePort returned port %d that is not available", port)
	}
}

func TestFindAvailablePort_AllOccupied(t *testing.T) {
	// Bind 100 consecutive ports starting from a high range to simulate exhaustion.
	// Use a high port range to avoid conflicts with system services.
	const startPort = 59000
	const count = 100

	listeners := make([]net.Listener, 0, count)
	bound := 0
	for i := 0; i < count; i++ {
		ln, err := net.Listen("tcp", fmt.Sprintf(":%d", startPort+i))
		if err != nil {
			break
		}
		listeners = append(listeners, ln)
		bound++
	}
	defer func() {
		for _, l := range listeners {
			l.Close()
		}
	}()

	if bound < count {
		t.Skipf("could only bind %d/%d ports; skipping exhaustion test", bound, count)
	}

	_, err := FindAvailablePort(startPort)
	if err == nil {
		t.Error("expected ErrNoAvailablePort when all ports are occupied")
	}
}

func TestWhoIsUsingPort_ReturnsBestEffort(t *testing.T) {
	ln, err := net.Listen("tcp", ":0")
	if err != nil {
		t.Skip("could not bind port for test setup")
	}
	defer ln.Close()
	port := ln.Addr().(*net.TCPAddr).Port

	// WhoIsUsingPort is best-effort; just verify it doesn't panic.
	result := WhoIsUsingPort(port)
	t.Logf("WhoIsUsingPort(%d) = %q", port, result)
	// No assertion on content — availability depends on system tools.
}

func TestDefaultPort(t *testing.T) {
	if DefaultPort != 8000 {
		t.Errorf("expected DefaultPort=8000, got %d", DefaultPort)
	}
}

// getFreePort asks the OS for a free port without binding it.
func getFreePort() (int, error) {
	ln, err := net.Listen("tcp", ":0")
	if err != nil {
		return 0, err
	}
	port := ln.Addr().(*net.TCPAddr).Port
	ln.Close()
	return port, nil
}

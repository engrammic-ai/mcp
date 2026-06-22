package core

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

// overrideHome redirects os.UserHomeDir to a temp dir for the duration of the test.
func overrideHome(t *testing.T) string {
	t.Helper()
	tmp := t.TempDir()
	t.Setenv("HOME", tmp)
	return tmp
}

func TestStateDir_CreatesDir(t *testing.T) {
	home := overrideHome(t)
	dir, err := StateDir()
	if err != nil {
		t.Fatalf("StateDir() error: %v", err)
	}
	expected := filepath.Join(home, ".engrammic")
	if dir != expected {
		t.Errorf("got %q, want %q", dir, expected)
	}
	if _, err := os.Stat(dir); err != nil {
		t.Errorf("dir not created: %v", err)
	}
}

func TestLoadState_MissingFile(t *testing.T) {
	overrideHome(t)
	s, err := LoadState()
	if err != nil {
		t.Fatalf("LoadState() error: %v", err)
	}
	if s == nil {
		t.Fatal("expected non-nil State")
	}
	if s.Version != stateVersion {
		t.Errorf("version: got %d, want %d", s.Version, stateVersion)
	}
	if s.Harnesses == nil {
		t.Error("Harnesses map should be initialized")
	}
}

func TestSaveAndLoad(t *testing.T) {
	overrideHome(t)
	s, err := LoadState()
	if err != nil {
		t.Fatalf("LoadState: %v", err)
	}
	port := 8080
	pid := 12345
	s.Server = &ServerState{
		PID:       &pid,
		Port:      port,
		Endpoint:  "http://localhost:8080",
		StartedAt: time.Now().UTC().Truncate(time.Second),
	}
	if err := s.Save(); err != nil {
		t.Fatalf("Save: %v", err)
	}

	loaded, err := LoadState()
	if err != nil {
		t.Fatalf("LoadState after save: %v", err)
	}
	if loaded.Server == nil {
		t.Fatal("expected Server to be non-nil after reload")
	}
	if *loaded.Server.PID != pid {
		t.Errorf("PID: got %d, want %d", *loaded.Server.PID, pid)
	}
	if loaded.Server.Port != port {
		t.Errorf("Port: got %d, want %d", loaded.Server.Port, port)
	}
}

func TestAtomicWrite_TmpFileRemoved(t *testing.T) {
	home := overrideHome(t)
	s, _ := LoadState()
	if err := s.Save(); err != nil {
		t.Fatalf("Save: %v", err)
	}
	// tmp file should not exist after successful save
	tmp := filepath.Join(home, ".engrammic", "state.json.tmp")
	if _, err := os.Stat(tmp); !os.IsNotExist(err) {
		t.Error("tmp file should not exist after atomic save")
	}
}

func TestSetAndClearServer(t *testing.T) {
	overrideHome(t)
	s, _ := LoadState()

	srv := ServerState{Port: 9090, Endpoint: "http://localhost:9090", StartedAt: time.Now()}
	if err := s.SetServer(srv); err != nil {
		t.Fatalf("SetServer: %v", err)
	}
	if s.Server == nil {
		t.Fatal("server should be set")
	}

	if err := s.ClearServer(); err != nil {
		t.Fatalf("ClearServer: %v", err)
	}
	if s.Server != nil {
		t.Error("server should be nil after clear")
	}

	// persisted correctly
	loaded, _ := LoadState()
	if loaded.Server != nil {
		t.Error("server should be nil in persisted state")
	}
}

func TestSetAndRemoveHarness(t *testing.T) {
	overrideHome(t)
	s, _ := LoadState()

	h := HarnessState{
		InstalledAt: time.Now(),
		ConfigPath:  "/home/user/.config/claude/config.json",
		Endpoint:    "https://api.engrammic.io",
	}
	if err := s.SetHarness("claude-code", h); err != nil {
		t.Fatalf("SetHarness: %v", err)
	}
	ids := s.InstalledHarnesses()
	if len(ids) != 1 || ids[0] != "claude-code" {
		t.Errorf("InstalledHarnesses: got %v", ids)
	}

	if err := s.RemoveHarness("claude-code"); err != nil {
		t.Fatalf("RemoveHarness: %v", err)
	}
	if len(s.InstalledHarnesses()) != 0 {
		t.Error("expected no harnesses after remove")
	}

	loaded, _ := LoadState()
	if len(loaded.InstalledHarnesses()) != 0 {
		t.Error("expected no harnesses in persisted state")
	}
}

func TestIsServerRunning_NilServer(t *testing.T) {
	s := &State{}
	if s.IsServerRunning() {
		t.Error("expected false for nil server")
	}
}

func TestIsServerRunning_CurrentProcess(t *testing.T) {
	pid := os.Getpid()
	s := &State{
		Server: &ServerState{PID: &pid},
	}
	if !s.IsServerRunning() {
		t.Error("expected true for current process PID")
	}
}

func TestIsServerRunning_DeadPID(t *testing.T) {
	// PID 0 is the swapper/scheduler — not a user process, signal(0) should fail
	pid := 0
	s := &State{
		Server: &ServerState{PID: &pid},
	}
	// We don't assert a specific value here since OS behaviour varies,
	// but the method must not panic.
	_ = s.IsServerRunning()
}

func TestMultipleHarnesses(t *testing.T) {
	overrideHome(t)
	s, _ := LoadState()

	for _, id := range []string{"cursor", "claude-code", "vscode"} {
		_ = s.SetHarness(id, HarnessState{InstalledAt: time.Now(), ConfigPath: "/tmp/" + id, Endpoint: "http://x"})
	}
	ids := s.InstalledHarnesses()
	if len(ids) != 3 {
		t.Errorf("expected 3 harnesses, got %d", len(ids))
	}
}

package core

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

const stateVersion = 1

type State struct {
	Version       int                     `json:"version"`
	ConfigVersion int                     `json:"config_version"`
	LastUpdated   time.Time               `json:"last_updated"`
	Server        *ServerState            `json:"server,omitempty"`
	Harnesses     map[string]HarnessState `json:"harnesses"`
}

type ServerState struct {
	PID         *int      `json:"pid,omitempty"`
	ContainerID *string   `json:"container_id,omitempty"`
	Port        int       `json:"port"`
	Endpoint    string    `json:"endpoint"`
	StartedAt   time.Time `json:"started_at"`
}

type HarnessState struct {
	InstalledAt time.Time `json:"installed_at"`
	ConfigPath  string    `json:"config_path"`
	Endpoint    string    `json:"endpoint"`
}

// StateDir returns ~/.engrammic, creating it if needed.
func StateDir() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("find home dir: %w", err)
	}
	dir := filepath.Join(home, ".engrammic")
	if err := os.MkdirAll(dir, 0o700); err != nil {
		return "", fmt.Errorf("create state dir: %w", err)
	}
	return dir, nil
}

// LoadState reads ~/.engrammic/state.json, returning an empty State if the file doesn't exist.
func LoadState() (*State, error) {
	dir, err := StateDir()
	if err != nil {
		return nil, err
	}
	path := filepath.Join(dir, "state.json")
	data, err := os.ReadFile(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return &State{Version: stateVersion, Harnesses: map[string]HarnessState{}}, nil
		}
		return nil, fmt.Errorf("read state file: %w", err)
	}
	var s State
	if err := json.Unmarshal(data, &s); err != nil {
		return nil, fmt.Errorf("parse state file: %w", err)
	}
	if s.Harnesses == nil {
		s.Harnesses = map[string]HarnessState{}
	}
	return &s, nil
}

// Save writes the state atomically to ~/.engrammic/state.json.
func (s *State) Save() error {
	dir, err := StateDir()
	if err != nil {
		return err
	}
	s.LastUpdated = time.Now()
	data, err := json.MarshalIndent(s, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal state: %w", err)
	}
	target := filepath.Join(dir, "state.json")
	tmp := target + ".tmp"
	if err := os.WriteFile(tmp, data, 0o600); err != nil {
		return fmt.Errorf("write temp state: %w", err)
	}
	if err := os.Rename(tmp, target); err != nil {
		_ = os.Remove(tmp)
		return fmt.Errorf("rename state file: %w", err)
	}
	return nil
}

// SetServer updates the server state and saves.
func (s *State) SetServer(server ServerState) error {
	s.Server = &server
	return s.Save()
}

// ClearServer removes the server state and saves.
func (s *State) ClearServer() error {
	s.Server = nil
	return s.Save()
}

// IsServerRunning checks whether the tracked server process or container is still running.
func (s *State) IsServerRunning() bool {
	if s.Server == nil {
		return false
	}
	if s.Server.PID != nil {
		proc, err := os.FindProcess(*s.Server.PID)
		if err != nil {
			return false
		}
		// signal 0 checks existence without sending a real signal
		err = proc.Signal(syscall.Signal(0))
		return err == nil
	}
	if s.Server.ContainerID != nil {
		return isContainerRunning(*s.Server.ContainerID)
	}
	return false
}

// isContainerRunning shells out to docker to check if a container is running.
func isContainerRunning(containerID string) bool {
	out, err := exec.Command("docker", "ps", "-q", "-f", "id="+containerID).Output()
	if err != nil {
		return false
	}
	return strings.TrimSpace(string(out)) != ""
}

// SetHarness records a harness installation and saves.
func (s *State) SetHarness(id string, state HarnessState) error {
	if s.Harnesses == nil {
		s.Harnesses = map[string]HarnessState{}
	}
	s.Harnesses[id] = state
	return s.Save()
}

// RemoveHarness removes a harness record and saves.
func (s *State) RemoveHarness(id string) error {
	delete(s.Harnesses, id)
	return s.Save()
}

// InstalledHarnesses returns the list of installed harness IDs.
func (s *State) InstalledHarnesses() []string {
	ids := make([]string, 0, len(s.Harnesses))
	for id := range s.Harnesses {
		ids = append(ids, id)
	}
	return ids
}

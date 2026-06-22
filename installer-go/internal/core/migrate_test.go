package core

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// --- BackupConfig / RestoreConfig ---

func TestBackupRestoreConfig(t *testing.T) {
	tmp := t.TempDir()
	original := filepath.Join(tmp, "config.json")
	content := []byte(`{"mcpServers":{}}`)
	if err := os.WriteFile(original, content, 0o600); err != nil {
		t.Fatal(err)
	}

	// Override home via env so backups go into tmp
	t.Setenv("HOME", tmp)

	backupPath, err := BackupConfig(original)
	if err != nil {
		t.Fatalf("BackupConfig: %v", err)
	}
	if _, err := os.Stat(backupPath); err != nil {
		t.Fatalf("backup file missing: %v", err)
	}

	// Corrupt original, then restore
	if err := os.WriteFile(original, []byte("corrupted"), 0o600); err != nil {
		t.Fatal(err)
	}
	if err := RestoreConfig(backupPath, original); err != nil {
		t.Fatalf("RestoreConfig: %v", err)
	}
	got, _ := os.ReadFile(original)
	if string(got) != string(content) {
		t.Errorf("restored content mismatch: got %q", got)
	}
}

func TestBackupPrunesOldBackups(t *testing.T) {
	tmp := t.TempDir()
	t.Setenv("HOME", tmp)

	src := filepath.Join(tmp, "cfg.json")
	if err := os.WriteFile(src, []byte(`{}`), 0o600); err != nil {
		t.Fatal(err)
	}

	for i := 0; i < maxBackupsPerFile+3; i++ {
		if _, err := BackupConfig(src); err != nil {
			t.Fatalf("iteration %d: %v", i, err)
		}
	}

	backupDir := filepath.Join(tmp, ".engrammic", "backups")
	entries, _ := os.ReadDir(backupDir)
	count := 0
	for _, e := range entries {
		if strings.HasSuffix(e.Name(), "_cfg.json") {
			count++
		}
	}
	if count > maxBackupsPerFile {
		t.Errorf("expected at most %d backups, got %d", maxBackupsPerFile, count)
	}
}

// --- JSON merge ---

func TestMergeJsonMap_EmptyFile(t *testing.T) {
	shape := *standardJSON
	out, err := MergeServerConfig(nil, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	var root map[string]any
	if err := json.Unmarshal(out, &root); err != nil {
		t.Fatalf("output not valid JSON: %v", err)
	}
	servers := root["mcpServers"].(map[string]any)
	entry := servers["engrammic"].(map[string]any)
	if entry["url"] != "http://localhost:4000/mcp" {
		t.Errorf("url mismatch: %v", entry["url"])
	}
	if entry["type"] != "http" {
		t.Errorf("type mismatch: %v", entry["type"])
	}
}

func TestMergeJsonMap_PreservesOtherEntries(t *testing.T) {
	existing := []byte(`{"mcpServers":{"other":{"url":"http://other/mcp"}}}`)
	shape := *standardJSON
	out, err := MergeServerConfig(existing, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	var root map[string]any
	if err := json.Unmarshal(out, &root); err != nil {
		t.Fatal(err)
	}
	servers := root["mcpServers"].(map[string]any)
	if _, ok := servers["other"]; !ok {
		t.Error("other entry was removed")
	}
	if _, ok := servers["engrammic"]; !ok {
		t.Error("engrammic entry missing")
	}
}

func TestMergeJsonMap_UpdatesExistingEntry(t *testing.T) {
	existing := []byte(`{"mcpServers":{"engrammic":{"url":"http://old/mcp","type":"http"}}}`)
	shape := *standardJSON
	out, err := MergeServerConfig(existing, shape, "engrammic", "http://new/mcp")
	if err != nil {
		t.Fatal(err)
	}
	var root map[string]any
	if err := json.Unmarshal(out, &root); err != nil {
		t.Fatal(err)
	}
	servers := root["mcpServers"].(map[string]any)
	entry := servers["engrammic"].(map[string]any)
	if entry["url"] != "http://new/mcp" {
		t.Errorf("url not updated: %v", entry["url"])
	}
}

func TestMergeJsonMap_DottedContainer(t *testing.T) {
	// ampJSON uses "amp.mcpServers"
	shape := *ampJSON
	out, err := MergeServerConfig(nil, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	var root map[string]any
	if err := json.Unmarshal(out, &root); err != nil {
		t.Fatal(err)
	}
	amp := root["amp"].(map[string]any)
	servers := amp["mcpServers"].(map[string]any)
	if _, ok := servers["engrammic"]; !ok {
		t.Error("engrammic entry missing under amp.mcpServers")
	}
}

// --- JSON remove ---

func TestRemoveJsonMap_RemovesEntry(t *testing.T) {
	existing := []byte(`{"mcpServers":{"engrammic":{"url":"http://localhost:4000/mcp"},"other":{"url":"http://other"}}}`)
	shape := *standardJSON
	out, err := RemoveServerConfig(existing, shape, "engrammic")
	if err != nil {
		t.Fatal(err)
	}
	var root map[string]any
	if err := json.Unmarshal(out, &root); err != nil {
		t.Fatal(err)
	}
	servers := root["mcpServers"].(map[string]any)
	if _, ok := servers["engrammic"]; ok {
		t.Error("engrammic entry should have been removed")
	}
	if _, ok := servers["other"]; !ok {
		t.Error("other entry should be preserved")
	}
}

func TestRemoveJsonMap_RemovesEmptyContainer(t *testing.T) {
	existing := []byte(`{"mcpServers":{"engrammic":{"url":"http://localhost:4000/mcp"}}}`)
	shape := *standardJSON
	out, err := RemoveServerConfig(existing, shape, "engrammic")
	if err != nil {
		t.Fatal(err)
	}
	var root map[string]any
	if err := json.Unmarshal(out, &root); err != nil {
		t.Fatal(err)
	}
	if _, ok := root["mcpServers"]; ok {
		t.Error("empty mcpServers container should have been removed")
	}
}

// --- TOML merge ---

func TestMergeCodexToml_Empty(t *testing.T) {
	shape := ConfigShape{Kind: ConfigShapeCodexToml}
	out, err := MergeServerConfig(nil, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(out), "engrammic") {
		t.Error("engrammic missing from TOML output")
	}
	if !strings.Contains(string(out), "http://localhost:4000/mcp") {
		t.Error("endpoint missing from TOML output")
	}
}

func TestMergeCodexToml_PreservesOtherSections(t *testing.T) {
	existing := []byte("[mcp_servers.other]\nurl = \"http://other/mcp\"\n")
	shape := ConfigShape{Kind: ConfigShapeCodexToml}
	out, err := MergeServerConfig(existing, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(out), "other") {
		t.Error("other section removed")
	}
	if !strings.Contains(string(out), "engrammic") {
		t.Error("engrammic missing")
	}
}

func TestRemoveCodexToml(t *testing.T) {
	existing := []byte("[mcp_servers.engrammic]\nurl = \"http://localhost:4000/mcp\"\n\n[mcp_servers.other]\nurl = \"http://other\"\n")
	shape := ConfigShape{Kind: ConfigShapeCodexToml}
	out, err := RemoveServerConfig(existing, shape, "engrammic")
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(out), "engrammic") {
		t.Error("engrammic should have been removed")
	}
	if !strings.Contains(string(out), "other") {
		t.Error("other section removed unexpectedly")
	}
}

// --- YAML merge (Goose) ---

func TestMergeGooseYaml_Empty(t *testing.T) {
	shape := ConfigShape{Kind: ConfigShapeGooseYaml}
	out, err := MergeServerConfig(nil, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(out), "engrammic") {
		t.Error("engrammic missing from YAML output")
	}
	if !strings.Contains(string(out), "http://localhost:4000/mcp") {
		t.Error("endpoint missing from YAML output")
	}
}

func TestMergeGooseYaml_PreservesOtherEntries(t *testing.T) {
	existing := []byte("extensions:\n  other:\n    endpoint: http://other\n    type: sse\n")
	shape := ConfigShape{Kind: ConfigShapeGooseYaml}
	out, err := MergeServerConfig(existing, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(out), "other") {
		t.Error("other entry removed")
	}
	if !strings.Contains(string(out), "engrammic") {
		t.Error("engrammic missing")
	}
}

func TestRemoveGooseYaml(t *testing.T) {
	existing := []byte("extensions:\n  engrammic:\n    endpoint: http://localhost:4000/mcp\n    type: sse\n  other:\n    endpoint: http://other\n    type: sse\n")
	shape := ConfigShape{Kind: ConfigShapeGooseYaml}
	out, err := RemoveServerConfig(existing, shape, "engrammic")
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(out), "engrammic") {
		t.Error("engrammic should have been removed")
	}
	if !strings.Contains(string(out), "other") {
		t.Error("other entry removed unexpectedly")
	}
}

// --- YAML merge (Hermes) ---

func TestMergeHermesYaml_Empty(t *testing.T) {
	shape := ConfigShape{Kind: ConfigShapeHermesYaml}
	out, err := MergeServerConfig(nil, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(out), "mcp_servers") {
		t.Error("mcp_servers missing")
	}
	if !strings.Contains(string(out), "engrammic") {
		t.Error("engrammic missing")
	}
}

func TestRemoveHermesYaml(t *testing.T) {
	existing := []byte("mcp_servers:\n  engrammic:\n    url: http://localhost:4000/mcp\n  other:\n    url: http://other\n")
	shape := ConfigShape{Kind: ConfigShapeHermesYaml}
	out, err := RemoveServerConfig(existing, shape, "engrammic")
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(out), "engrammic") {
		t.Error("engrammic should have been removed")
	}
	if !strings.Contains(string(out), "other") {
		t.Error("other entry removed unexpectedly")
	}
}

// --- Continue YAML ---

func TestMergeContinueYaml(t *testing.T) {
	shape := ConfigShape{Kind: ConfigShapeContinueYaml}
	out, err := MergeServerConfig(nil, shape, "engrammic", "http://localhost:4000/mcp")
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(out), "engrammic") {
		t.Error("name missing from Continue YAML")
	}
	if !strings.Contains(string(out), "http://localhost:4000/mcp") {
		t.Error("url missing from Continue YAML")
	}
}

func TestRemoveContinueYaml(t *testing.T) {
	shape := ConfigShape{Kind: ConfigShapeContinueYaml}
	out, err := RemoveServerConfig([]byte("name: engrammic\n"), shape, "engrammic")
	if err != nil {
		t.Fatal(err)
	}
	if len(out) != 0 {
		t.Errorf("expected empty bytes for Continue remove, got %q", out)
	}
}

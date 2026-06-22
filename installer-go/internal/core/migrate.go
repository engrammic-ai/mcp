package core

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"github.com/pelletier/go-toml/v2"
	"gopkg.in/yaml.v3"
)

const maxBackupsPerFile = 10

// BackupConfig copies the file at configPath to ~/.engrammic/backups/<timestamp>_<basename>.
// Returns the backup path.
func BackupConfig(configPath string) (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("find home dir: %w", err)
	}
	backupDir := filepath.Join(home, ".engrammic", "backups")
	if err := os.MkdirAll(backupDir, 0o700); err != nil {
		return "", fmt.Errorf("create backup dir: %w", err)
	}

	data, err := os.ReadFile(configPath)
	if err != nil {
		return "", fmt.Errorf("read config: %w", err)
	}

	ts := time.Now().Format("20060102T150405")
	base := filepath.Base(configPath)
	backupName := ts + "_" + base
	backupPath := filepath.Join(backupDir, backupName)

	if err := os.WriteFile(backupPath, data, 0o600); err != nil {
		return "", fmt.Errorf("write backup: %w", err)
	}

	pruneOldBackups(backupDir, base)

	return backupPath, nil
}

// pruneOldBackups removes oldest backups for a given base filename, keeping at most maxBackupsPerFile.
func pruneOldBackups(backupDir, base string) {
	entries, err := os.ReadDir(backupDir)
	if err != nil {
		return
	}

	var matching []string
	for _, e := range entries {
		if !e.IsDir() && strings.HasSuffix(e.Name(), "_"+base) {
			matching = append(matching, filepath.Join(backupDir, e.Name()))
		}
	}

	sort.Strings(matching) // timestamp prefix makes lexical sort chronological
	for len(matching) > maxBackupsPerFile {
		_ = os.Remove(matching[0])
		matching = matching[1:]
	}
}

// RestoreConfig copies the backup file back to originalPath, overwriting it.
func RestoreConfig(backupPath, originalPath string) error {
	src, err := os.Open(backupPath)
	if err != nil {
		return fmt.Errorf("open backup: %w", err)
	}
	defer src.Close()

	data, err := io.ReadAll(src)
	if err != nil {
		return fmt.Errorf("read backup: %w", err)
	}

	if err := os.MkdirAll(filepath.Dir(originalPath), 0o755); err != nil {
		return fmt.Errorf("create config dir: %w", err)
	}

	if err := os.WriteFile(originalPath, data, 0o600); err != nil {
		return fmt.Errorf("write config: %w", err)
	}

	return nil
}

// MergeServerConfig merges an Engrammic MCP server entry into existing config content.
// It adds or updates only the entry for serverName, preserving all other entries.
func MergeServerConfig(existing []byte, shape ConfigShape, serverName, endpoint string) ([]byte, error) {
	switch shape.Kind {
	case ConfigShapeJsonMap, ConfigShapeVSCodeJson, ConfigShapeOpenCodeJson:
		return mergeJsonMap(existing, shape, serverName, endpoint)
	case ConfigShapeCodexToml:
		return mergeCodexToml(existing, serverName, endpoint)
	case ConfigShapeGooseYaml:
		return mergeGooseYaml(existing, serverName, endpoint)
	case ConfigShapeHermesYaml:
		return mergeHermesYaml(existing, serverName, endpoint)
	case ConfigShapeContinueYaml:
		return mergeContinueYaml(existing, serverName, endpoint)
	default:
		return mergeJsonMap(existing, shape, serverName, endpoint)
	}
}

// RemoveServerConfig removes the Engrammic MCP server entry from config content.
// It preserves all other entries.
func RemoveServerConfig(existing []byte, shape ConfigShape, serverName string) ([]byte, error) {
	switch shape.Kind {
	case ConfigShapeJsonMap, ConfigShapeVSCodeJson, ConfigShapeOpenCodeJson:
		return removeJsonMap(existing, shape, serverName)
	case ConfigShapeCodexToml:
		return removeCodexToml(existing, serverName)
	case ConfigShapeGooseYaml:
		return removeGooseYaml(existing, serverName)
	case ConfigShapeHermesYaml:
		return removeHermesYaml(existing, serverName)
	case ConfigShapeContinueYaml:
		return removeContinueYaml(existing, serverName)
	default:
		return removeJsonMap(existing, shape, serverName)
	}
}

// buildServerEntry builds the JSON object for a server entry using the given shape.
func buildServerEntry(shape ConfigShape, endpoint string) map[string]any {
	entry := map[string]any{
		shape.UrlField: endpoint,
	}
	if shape.TypeField != TypeFieldNone {
		entry["type"] = shape.TypeField.Value()
	}
	return entry
}

// mergeJsonMap handles JSON configs with a container key like "mcpServers".
func mergeJsonMap(existing []byte, shape ConfigShape, serverName, endpoint string) ([]byte, error) {
	var root map[string]any
	if len(existing) == 0 {
		root = map[string]any{}
	} else {
		if err := json.Unmarshal(existing, &root); err != nil {
			return nil, fmt.Errorf("parse JSON config: %w", err)
		}
	}

	// Navigate/create container path (supports dotted keys like "amp.mcpServers")
	parts := strings.Split(shape.Container, ".")
	cur := root
	for i, part := range parts {
		if i == len(parts)-1 {
			container, _ := cur[part].(map[string]any)
			if container == nil {
				container = map[string]any{}
			}
			container[serverName] = buildServerEntry(shape, endpoint)
			cur[part] = container
		} else {
			next, _ := cur[part].(map[string]any)
			if next == nil {
				next = map[string]any{}
				cur[part] = next
			}
			cur = next
		}
	}

	out, err := json.MarshalIndent(root, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("marshal JSON config: %w", err)
	}
	return append(out, '\n'), nil
}

// removeJsonMap removes a server entry from a JSON map config.
func removeJsonMap(existing []byte, shape ConfigShape, serverName string) ([]byte, error) {
	if len(existing) == 0 {
		return existing, nil
	}

	var root map[string]any
	if err := json.Unmarshal(existing, &root); err != nil {
		return nil, fmt.Errorf("parse JSON config: %w", err)
	}

	parts := strings.Split(shape.Container, ".")
	cur := root
	for i, part := range parts {
		if i == len(parts)-1 {
			container, _ := cur[part].(map[string]any)
			if container != nil {
				delete(container, serverName)
				if len(container) == 0 {
					delete(cur, part)
				} else {
					cur[part] = container
				}
			}
		} else {
			next, _ := cur[part].(map[string]any)
			if next == nil {
				return existing, nil
			}
			cur = next
		}
	}

	if len(root) == 0 {
		return []byte("{}\n"), nil
	}

	out, err := json.MarshalIndent(root, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("marshal JSON config: %w", err)
	}
	return append(out, '\n'), nil
}

// codexTomlConfig represents the structure of a Codex TOML config.
type codexTomlConfig struct {
	McpServers map[string]codexMcpServer `toml:"mcp_servers,omitempty"`
	Extra      map[string]any            `toml:",inline"`
}

type codexMcpServer struct {
	Url string `toml:"url"`
}

// mergeCodexToml adds/updates [mcp_servers.<serverName>] in a TOML config.
func mergeCodexToml(existing []byte, serverName, endpoint string) ([]byte, error) {
	var root map[string]any
	if len(existing) == 0 {
		root = map[string]any{}
	} else {
		if err := toml.Unmarshal(existing, &root); err != nil {
			return nil, fmt.Errorf("parse TOML config: %w", err)
		}
	}

	mcpServers, _ := root["mcp_servers"].(map[string]any)
	if mcpServers == nil {
		mcpServers = map[string]any{}
	}
	mcpServers[serverName] = map[string]any{"url": endpoint}
	root["mcp_servers"] = mcpServers

	out, err := toml.Marshal(root)
	if err != nil {
		return nil, fmt.Errorf("marshal TOML config: %w", err)
	}
	return out, nil
}

// removeCodexToml removes [mcp_servers.<serverName>] from a TOML config.
func removeCodexToml(existing []byte, serverName string) ([]byte, error) {
	if len(existing) == 0 {
		return existing, nil
	}

	var root map[string]any
	if err := toml.Unmarshal(existing, &root); err != nil {
		return nil, fmt.Errorf("parse TOML config: %w", err)
	}

	mcpServers, _ := root["mcp_servers"].(map[string]any)
	if mcpServers != nil {
		delete(mcpServers, serverName)
		if len(mcpServers) == 0 {
			delete(root, "mcp_servers")
		} else {
			root["mcp_servers"] = mcpServers
		}
	}

	if len(root) == 0 {
		return []byte{}, nil
	}

	out, err := toml.Marshal(root)
	if err != nil {
		return nil, fmt.Errorf("marshal TOML config: %w", err)
	}
	return out, nil
}

// mergeGooseYaml adds/updates the MCP server entry in a Goose YAML config.
// Goose uses: extensions.<serverName>.endpoint / type = "sse"
func mergeGooseYaml(existing []byte, serverName, endpoint string) ([]byte, error) {
	var root yaml.Node
	if len(existing) == 0 {
		root = yaml.Node{Kind: yaml.DocumentNode, Content: []*yaml.Node{
			{Kind: yaml.MappingNode, Tag: "!!map"},
		}}
	} else {
		if err := yaml.Unmarshal(existing, &root); err != nil {
			return nil, fmt.Errorf("parse YAML config: %w", err)
		}
		if root.Kind == 0 {
			root = yaml.Node{Kind: yaml.DocumentNode, Content: []*yaml.Node{
				{Kind: yaml.MappingNode, Tag: "!!map"},
			}}
		}
	}

	mapping := root.Content[0]
	// Find or create "extensions" key
	extNode := yamlFindOrCreateMapping(mapping, "extensions")
	// Find or create serverName key under extensions
	serverNode := yamlFindOrCreateMapping(extNode, serverName)
	// Set endpoint and type
	yamlSetString(serverNode, "endpoint", endpoint)
	yamlSetString(serverNode, "type", "sse")

	out, err := yaml.Marshal(&root)
	if err != nil {
		return nil, fmt.Errorf("marshal YAML config: %w", err)
	}
	return out, nil
}

// removeGooseYaml removes an MCP server entry from a Goose YAML config.
func removeGooseYaml(existing []byte, serverName string) ([]byte, error) {
	return removeYamlEntry(existing, "extensions", serverName)
}

// mergeHermesYaml adds/updates the MCP server entry in a Hermes YAML config.
// Hermes uses: mcp_servers.<serverName>.url
func mergeHermesYaml(existing []byte, serverName, endpoint string) ([]byte, error) {
	var root yaml.Node
	if len(existing) == 0 {
		root = yaml.Node{Kind: yaml.DocumentNode, Content: []*yaml.Node{
			{Kind: yaml.MappingNode, Tag: "!!map"},
		}}
	} else {
		if err := yaml.Unmarshal(existing, &root); err != nil {
			return nil, fmt.Errorf("parse YAML config: %w", err)
		}
		if root.Kind == 0 {
			root = yaml.Node{Kind: yaml.DocumentNode, Content: []*yaml.Node{
				{Kind: yaml.MappingNode, Tag: "!!map"},
			}}
		}
	}

	mapping := root.Content[0]
	serversNode := yamlFindOrCreateMapping(mapping, "mcp_servers")
	serverNode := yamlFindOrCreateMapping(serversNode, serverName)
	yamlSetString(serverNode, "url", endpoint)

	out, err := yaml.Marshal(&root)
	if err != nil {
		return nil, fmt.Errorf("marshal YAML config: %w", err)
	}
	return out, nil
}

// removeHermesYaml removes an MCP server entry from a Hermes YAML config.
func removeHermesYaml(existing []byte, serverName string) ([]byte, error) {
	return removeYamlEntry(existing, "mcp_servers", serverName)
}

// mergeContinueYaml writes a standalone Continue.dev server YAML file.
// Continue uses a per-server file: .continue/mcpServers/<name>.yaml
func mergeContinueYaml(existing []byte, serverName, endpoint string) ([]byte, error) {
	config := map[string]any{
		"name": serverName,
		"transport": map[string]any{
			"type": "http",
			"url":  endpoint,
		},
	}
	out, err := yaml.Marshal(config)
	if err != nil {
		return nil, fmt.Errorf("marshal Continue YAML: %w", err)
	}
	return out, nil
}

// removeContinueYaml returns empty bytes (caller should delete the file).
func removeContinueYaml(_ []byte, _ string) ([]byte, error) {
	return []byte{}, nil
}

// removeYamlEntry removes serverName from a top-level container key in a YAML doc.
func removeYamlEntry(existing []byte, containerKey, serverName string) ([]byte, error) {
	if len(existing) == 0 {
		return existing, nil
	}

	var root yaml.Node
	if err := yaml.Unmarshal(existing, &root); err != nil {
		return nil, fmt.Errorf("parse YAML config: %w", err)
	}
	if root.Kind == 0 || len(root.Content) == 0 {
		return existing, nil
	}

	mapping := root.Content[0]
	for i := 0; i < len(mapping.Content)-1; i += 2 {
		if mapping.Content[i].Value == containerKey {
			container := mapping.Content[i+1]
			if container.Kind != yaml.MappingNode {
				break
			}
			for j := 0; j < len(container.Content)-1; j += 2 {
				if container.Content[j].Value == serverName {
					container.Content = append(container.Content[:j], container.Content[j+2:]...)
					break
				}
			}
			// Remove container key entirely if empty
			if len(container.Content) == 0 {
				mapping.Content = append(mapping.Content[:i], mapping.Content[i+2:]...)
			}
			break
		}
	}

	out, err := yaml.Marshal(&root)
	if err != nil {
		return nil, fmt.Errorf("marshal YAML config: %w", err)
	}
	return out, nil
}

// yamlFindOrCreateMapping finds a mapping child by key in a mapping node,
// creating it if it does not exist.
func yamlFindOrCreateMapping(mapping *yaml.Node, key string) *yaml.Node {
	for i := 0; i < len(mapping.Content)-1; i += 2 {
		if mapping.Content[i].Value == key {
			return mapping.Content[i+1]
		}
	}
	keyNode := &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: key}
	valNode := &yaml.Node{Kind: yaml.MappingNode, Tag: "!!map"}
	mapping.Content = append(mapping.Content, keyNode, valNode)
	return valNode
}

// yamlSetString sets a key to a string value in a mapping node, updating if present.
func yamlSetString(mapping *yaml.Node, key, value string) {
	for i := 0; i < len(mapping.Content)-1; i += 2 {
		if mapping.Content[i].Value == key {
			mapping.Content[i+1].Value = value
			return
		}
	}
	keyNode := &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: key}
	valNode := &yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: value}
	mapping.Content = append(mapping.Content, keyNode, valNode)
}

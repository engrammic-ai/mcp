// plain.go
package ui

import (
	"bufio"
	"fmt"
	"os"
	"strconv"
	"strings"
)

var reader = bufio.NewReader(os.Stdin)

func PlainSelect(prompt string, options []string, defaultIdx int) int {
	fmt.Println(prompt)
	for i, opt := range options {
		fmt.Printf("  %d. %s\n", i+1, opt)
	}
	fmt.Printf("Choice [%d]: ", defaultIdx+1)

	line, _ := reader.ReadString('\n')
	line = strings.TrimSpace(line)
	if line == "" {
		return defaultIdx
	}
	n, err := strconv.Atoi(line)
	if err != nil || n < 1 || n > len(options) {
		return defaultIdx
	}
	return n - 1
}

func PlainConfirm(prompt string, defaultYes bool) bool {
	hint := "[y/N]"
	if defaultYes {
		hint = "[Y/n]"
	}
	fmt.Printf("%s %s: ", prompt, hint)

	line, _ := reader.ReadString('\n')
	line = strings.TrimSpace(strings.ToLower(line))
	if line == "" {
		return defaultYes
	}
	return line == "y" || line == "yes"
}

func PlainInput(prompt string, defaultVal string) string {
	if defaultVal != "" {
		fmt.Printf("%s [%s]: ", prompt, defaultVal)
	} else {
		fmt.Printf("%s: ", prompt)
	}

	line, _ := reader.ReadString('\n')
	line = strings.TrimSpace(line)
	if line == "" {
		return defaultVal
	}
	return line
}

func PlainMultiSelect(prompt string, options []string, selected []bool) []bool {
	fmt.Println(prompt)
	for i, opt := range options {
		mark := "[ ]"
		if selected[i] {
			mark = "[x]"
		}
		fmt.Printf("  %d. %s %s\n", i+1, mark, opt)
	}
	fmt.Print("Toggle (1-N), done (d): ")

	for {
		line, _ := reader.ReadString('\n')
		line = strings.TrimSpace(strings.ToLower(line))
		if line == "d" || line == "" {
			return selected
		}
		n, err := strconv.Atoi(line)
		if err == nil && n >= 1 && n <= len(options) {
			selected[n-1] = !selected[n-1]
		}
		// Re-render
		fmt.Printf("\033[%dA", len(options)+2) // Move up
		for i, opt := range options {
			mark := "[ ]"
			if selected[i] {
				mark = "[x]"
			}
			fmt.Printf("  %d. %s %s\033[K\n", i+1, mark, opt)
		}
		fmt.Print("Toggle (1-N), done (d): ")
	}
}

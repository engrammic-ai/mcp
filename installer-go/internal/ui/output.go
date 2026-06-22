// output.go
package ui

import (
	"fmt"
	"os"
	"strings"

	"github.com/charmbracelet/lipgloss"
	"github.com/anthropics/engrammic/installer/internal/platform"
)

func printf(style lipgloss.Style, icon, format string, args ...any) {
	msg := fmt.Sprintf(format, args...)
	if platform.UseRichUI() {
		fmt.Println(style.Render(icon + " " + msg))
	} else {
		fmt.Printf("[%s] %s\n", icon, msg)
	}
}

func Success(format string, args ...any) {
	printf(SuccessStyle, "✓", format, args...)
}

func Error(format string, args ...any) {
	printf(ErrorStyle, "✗", format, args...)
}

func Warn(format string, args ...any) {
	printf(WarnStyle, "⚠", format, args...)
}

func Info(format string, args ...any) {
	printf(SubtleStyle, "•", format, args...)
}

func Fatal(format string, args ...any) {
	Error(format, args...)
	os.Exit(1)
}

func Title(text string) {
	if platform.UseRichUI() {
		fmt.Println(TitleStyle.Render(text))
	} else {
		fmt.Println(text)
		fmt.Println(strings.Repeat("-", len(text)))
	}
}

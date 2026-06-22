// theme.go
package ui

import "github.com/charmbracelet/lipgloss"

// Color palette
var (
	ColorPrimary  = lipgloss.Color("39")  // Blue
	ColorSuccess  = lipgloss.Color("42")  // Green
	ColorWarning  = lipgloss.Color("214") // Yellow/Orange
	ColorError    = lipgloss.Color("196") // Red
	ColorSubtle   = lipgloss.Color("241") // Gray
)

// Convenience aliases matching the brief's interface
var (
	Primary = ColorPrimary
	// Note: Success, Warning, Error, Subtle cannot be exported as var names
	// in this package because they clash with the function names Success(),
	// Error(), Warn(). Use the Color* variants instead.
)

var (
	TitleStyle = lipgloss.NewStyle().
			Bold(true).
			Foreground(ColorPrimary)

	SuccessStyle = lipgloss.NewStyle().
			Foreground(ColorSuccess)

	ErrorStyle = lipgloss.NewStyle().
			Foreground(ColorError)

	WarnStyle = lipgloss.NewStyle().
			Foreground(ColorWarning)

	SubtleStyle = lipgloss.NewStyle().
			Foreground(ColorSubtle)

	BoxStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorPrimary).
			Padding(1, 2)
)

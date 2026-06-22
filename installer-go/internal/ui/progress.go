// progress.go
package ui

import (
	"fmt"
	"strings"
	"time"

	"github.com/anthropics/engrammic/installer/internal/platform"
)

type ItemStatus int

const (
	StatusPending ItemStatus = iota
	StatusRunning
	StatusDone
	StatusFailed
	StatusSkipped
)

type ProgressItem struct {
	Name   string
	Status ItemStatus
	Detail string
}

type ProgressList struct {
	Items []ProgressItem
	frame int
}

func NewProgressList(items []string) *ProgressList {
	pl := &ProgressList{}
	for _, name := range items {
		pl.Items = append(pl.Items, ProgressItem{Name: name, Status: StatusPending})
	}
	return pl
}

func (p *ProgressList) SetStatus(name string, status ItemStatus, detail string) {
	for i := range p.Items {
		if p.Items[i].Name == name {
			p.Items[i].Status = status
			p.Items[i].Detail = detail
			return
		}
	}
}

func (p *ProgressList) Tick() {
	p.frame++
}

var spinnerFrames = []string{"⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"}

func (p *ProgressList) Render() string {
	var lines []string
	for _, item := range p.Items {
		var icon, style string
		switch item.Status {
		case StatusPending:
			icon = "○"
			style = "subtle"
		case StatusRunning:
			icon = spinnerFrames[p.frame%len(spinnerFrames)]
			style = "running"
		case StatusDone:
			icon = "✓"
			style = "success"
		case StatusFailed:
			icon = "✗"
			style = "error"
		case StatusSkipped:
			icon = "○"
			style = "subtle"
		}

		line := fmt.Sprintf("  %s %-20s", icon, item.Name)
		if item.Detail != "" {
			line += "  " + item.Detail
		}

		if platform.UseRichUI() {
			switch style {
			case "success":
				line = SuccessStyle.Render(line)
			case "error":
				line = ErrorStyle.Render(line)
			case "subtle":
				line = SubtleStyle.Render(line)
			}
		}
		lines = append(lines, line)
	}
	return strings.Join(lines, "\n")
}

func (p *ProgressList) StartTicker(render func()) func() {
	ticker := time.NewTicker(80 * time.Millisecond)
	done := make(chan struct{})
	go func() {
		for {
			select {
			case <-ticker.C:
				p.Tick()
				render()
			case <-done:
				ticker.Stop()
				return
			}
		}
	}()
	return func() { close(done) }
}

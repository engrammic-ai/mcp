// banner.go
package ui

import (
	"fmt"

	"github.com/anthropics/engrammic/installer/internal/platform"
)

const banner = `
 ╔═╗┌┐┌┌─┐┬─┐┌─┐┌┬┐┌┬┐┬┌─┐
 ║╣ ││││ ┬├┬┘├─┤││││││││
 ╚═╝┘└┘└─┘┴└─┴ ┴┴ ┴┴ ┴┴└─┘
`

func PrintBanner() {
	if platform.UseRichUI() {
		fmt.Println(TitleStyle.Render(banner))
	} else {
		fmt.Println("Engrammic Installer")
	}
}

.PHONY: build build-all test

build:
	go build -o ./engrammic ./cmd/engrammic

test:
	go test ./...

build-all:
	GOOS=linux GOARCH=amd64 go build -o ./engrammic-linux-amd64 ./cmd/engrammic
	GOOS=linux GOARCH=arm64 go build -o ./engrammic-linux-arm64 ./cmd/engrammic
	GOOS=darwin GOARCH=amd64 go build -o ./engrammic-darwin-amd64 ./cmd/engrammic
	GOOS=darwin GOARCH=arm64 go build -o ./engrammic-darwin-arm64 ./cmd/engrammic
	GOOS=windows GOARCH=amd64 go build -o ./engrammic-windows-amd64.exe ./cmd/engrammic

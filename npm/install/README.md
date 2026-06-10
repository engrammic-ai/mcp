# @engrammic/install

Thin npm shim over the [Engrammic](https://docs.engrammic.ai) Rust installer CLI.
Running `npx @engrammic/install` downloads and executes the correct prebuilt binary
for your platform — no compilation required.

## Usage

```sh
npx @engrammic/install
```

Or install globally:

```sh
npm install -g @engrammic/install
engrammic-install
```

### curl alternative

```sh
curl -fsSL https://get.engrammic.ai/install.sh | sh
```

## Supported platforms

| Platform        | Architecture |
|-----------------|--------------|
| Linux           | x64          |
| Linux           | arm64        |
| macOS           | x64          |
| macOS           | arm64 (Apple Silicon) |
| Windows         | x64          |

## How it works

On install, npm selects only the matching `@engrammic/install-{platform}` optional
dependency for your machine. The launcher (`index.js`) resolves that package's
directory, finds the prebuilt binary inside it, and spawns it with your arguments.

## Documentation

https://docs.engrammic.ai

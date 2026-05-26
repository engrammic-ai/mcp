# Getting Started

## 1. Authenticate

```bash
engrammic-mcp login
```

Opens browser for OAuth. Tokens stored at `~/.engrammic/credentials.json`.

**CI/headless environments:** Set `ENGRAMMIC_API_KEY` instead:
```bash
export ENGRAMMIC_API_KEY=your-key
```

## 2. First Session

Load the onboarding skill:
```
patterns(action: 'get', name: 'onboarding')
```

Or as slash command: `/engrammic-onboarding`

## 3. Try Memory

```
remember(content: "User prefers dark mode")
recall(query: "user preferences")
```

See [Skills](https://github.com/engrammic-ai/skills) for workflow guidance.

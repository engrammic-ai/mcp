# EAG Concepts

EAG (Epistemic Augmented Generation) is a memory model for AI agents. It organizes context into four layers:

## The Four Layers

| Layer | What it holds | Lifetime |
|-------|---------------|----------|
| **Memory** | Observations, events, things you noticed | Decays over time |
| **Knowledge** | Facts backed by evidence | Persists until contradicted |
| **Wisdom** | Beliefs synthesized from facts | Revises on new evidence |
| **Intelligence** | Reasoning chains | Session-scoped |

## When to Use Each Layer

**Memory** - "I noticed X" or "The user said Y"
- Ephemeral observations
- No evidence required
- Decays naturally

**Knowledge** - "X is true because [evidence]"
- Claims you can back up
- Requires evidence URI
- Persists until contradicted

**Wisdom** - "Based on [facts], I believe [conclusion]"
- Synthesized understanding
- Emerges from multiple facts
- Revises when evidence changes

**Intelligence** - "Let me reason through this"
- Working memory for current task
- Disappears after session

## Quick Heuristics

- **Memory:** Would I tell a colleague about this tomorrow? If no, don't store.
- **Knowledge:** Do I have evidence? If no, use Memory.
- **Wisdom:** Can I fill in "Based on [facts], I believe [conclusion]"? If no, it's a hunch - use Memory.

## Tools Mapping

| Tool | Layer | Purpose |
|------|-------|---------|
| `remember` | Memory | Store observations |
| `learn` | Knowledge | Store claims with evidence |
| `believe` | Wisdom | Declare commitments |
| `recall` | All | Search and retrieve |
| `link` | All | Create relationships |
| `trace` | All | Query provenance |
| `reason` | Intelligence | Record reasoning steps |
| `hypothesize` | Intelligence | Form tentative beliefs (session-scoped) |
| `commit` | Wisdom | Promote hypotheses to commitments |

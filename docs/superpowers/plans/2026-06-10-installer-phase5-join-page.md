# Installer Phase 5: Join-Page Alignment

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align `web/join` with the decisions shipped in the installer overhaul (Phases 1–4): add a headless/CI toggle to the curl hero, remove the dead `get.engrammic.ai/skills` step from the config panel and replace it with accurate copy, verify per-editor restart hints are consistent, apply a plain-language copy pass across all three paths (one-command / pick-your-editor / self-host), and update the two in-repo docs (`JOIN-UX-SPEC.md` archived; `PLAN-onboarding-improvements.md` supersession-annotated).

**Architecture:** No structural component changes. All targeted edits are within existing files. A new test file `src/components/curl-box.test.ts` exercises the canonical curl constants. The vitest environment is `node`; tests are pure unit tests against exported constants (no DOM/testing-library needed, consistent with the existing test suite).

**Tech Stack:** Next.js 16 + React 19 + TypeScript 5, Tailwind CSS v4, Vitest 3. Commands run from `web/join/`.

**Spec:** `mcp-client/docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md` § "Join-page alignment" + Decisions ("get.engrammic.ai/skills is cut", "curl-primary stands", "`-y` toggle", "JOIN-UX-SPEC.md archived").

**Git repo:** `/home/novusedge/Projects/delta-prime/web` is its own git repository (`.git` exists at that level). All commits in this plan target that repo. The `mcp-client` repo is a separate checkout; the spec lives there for reference only — do not commit to it in this plan.

**PRE-FLIGHT — canonical curl URL audit:**

| Location | Current command | Matches Phase 2 canonical? |
|---|---|---|
| `src/components/curl-box.tsx` line 7 | `curl -fsSL https://get.engrammic.ai/install.sh \| bash` | **YES** — domain and path match Phase 2 (`get.engrammic.ai/install.sh \| sh`). Note: `bash` vs `sh` — spec says `sh`; flagged as a **mismatch to fix** in Task 1. |
| `src/components/config-panel.tsx` line 101 | `curl -fsSL get.engrammic.ai/skills \| bash` | **REMOVED** in Task 2 (the endpoint is cut). |
| Headless variant (does not exist yet) | — | Task 1 adds `curl -fsSL https://get.engrammic.ai/install.sh \| sh -s -- -y` |

The `bash` → `sh` discrepancy in the hero is a real gap. Phase 2's install.sh uses a POSIX shebang and the spec examples always write `sh`. **Fix in Task 1 while adding the toggle.**

**Sequencing note:** Tasks 1 and 2 are independent of each other (different files) and may be run in parallel. Task 3 is a read-only audit of `harnesses.ts` — no changes expected. Tasks 4, 5, and 6 are independent of each other and of Tasks 1–3. Task 7 (tests) depends on Task 1's exported constants. Task 8 (verification + commit) is last.

---

### Component map (pre-flight audit)

| File | Role |
|---|---|
| `src/components/curl-box.tsx` | **Hero curl box** — macOS/Linux tab shows the main installer one-liner; Windows tab shows PowerShell. No headless/CI tab yet. Exports `CurlBox`. Defines `UNIX` and `WIN` constants at module scope (not exported — Task 1 exports them so tests can import). |
| `src/components/config-panel.tsx` | **Slide-out panel** rendered when a harness tile is selected (or when a tile is expanded). Contains Step 1 (MCP connect), Step 2 (dead skills curl — **must be replaced**), Step 3 (restart hint). |
| `src/components/install-hero.tsx` | **Page hero** — wraps `CurlBox`, the harness picker grid, the slide-out panel (via `AnimatePresence`), and the self-host callout block. Copy lives inline here for the hero heading, subheading, divider label, self-host callout headline, and self-host callout body. |
| `src/components/harness-tile.tsx` | Alternative harness rendering used in the catalog page; expands inline rather than in a slide-out panel. Delegates to `ConfigPanel`. No restart-hint copy of its own. |
| `src/components/copy-button.tsx` | Generic copy-to-clipboard button; no copy to change. |
| `src/components/harness-icon.tsx` | SVG icon lookup; no copy to change. |
| `src/lib/harnesses.ts` | Single source of truth for all harness data including `restartHint` per harness, `ENDPOINT` constant, deep-link builders. The file that must stay in sync with the installer CLI. |
| `src/lib/harnesses.test.ts` | Existing tests for harness data shape, deep link encoding, popular tiles. |
| `src/lib/catalog.ts` / `catalog.test.ts` | Harness filtering logic; not touched by this phase. |
| `src/lib/analytics.ts` / `analytics.test.ts` | Event tracking (no-ops in server context); not touched. |
| `src/app/page.tsx` | Root page — renders `InstallHero`; not touched. |
| `src/app/catalog/page.tsx` | Catalog page — renders harness tiles including `HarnessTile`; not touched. |
| `JOIN-UX-SPEC.md` | Older design doc (archived by this phase). |
| `PLAN-onboarding-improvements.md` | Earlier improvement plan; supersession-annotated by this phase. |

---

### Task 1: curl-box — fix `bash` → `sh`, export constants, add headless/CI toggle

**File:** `src/components/curl-box.tsx`

**Current state (full file, 43 lines):**
- `UNIX = 'curl -fsSL https://get.engrammic.ai/install.sh | bash'` — `bash` should be `sh`.
- `WIN = 'irm https://get.engrammic.ai/install.ps1 | iex'`
- Two tabs: "macOS / Linux" and "Windows"; one `cmd` displayed per tab; one `CopyButton`.
- No headless variant.

**After this task:**
- `bash` → `sh` in `UNIX`.
- Both `UNIX` and `WIN` exported (needed by Task 7 tests).
- New exported constant `UNIX_HEADLESS = 'curl -fsSL https://get.engrammic.ai/install.sh | sh -s -- -y'`.
- Under the main command block, a small toggle link: clicking it reveals/hides a second code row showing `UNIX_HEADLESS` with the label "Non-interactive — auto-configures detected editors (for CI / headless installs)".
- The toggle only shows when the `unix` tab is active (PowerShell has no `-y` equivalent in Phase 2).
- Analytics event: `curl_copy` with `os: 'unix-headless'` on copy.

- [ ] **Step 1: Read the current file** (already done above — proceed to edits).

- [ ] **Step 2: Replace `src/components/curl-box.tsx` in full:**

```tsx
'use client';

import { useState } from 'react';
import { CopyButton } from './copy-button';
import { track } from '@/lib/analytics';

export const UNIX = 'curl -fsSL https://get.engrammic.ai/install.sh | sh';
export const WIN = 'irm https://get.engrammic.ai/install.ps1 | iex';
export const UNIX_HEADLESS = 'curl -fsSL https://get.engrammic.ai/install.sh | sh -s -- -y';

export function CurlBox() {
  const [os, setOs] = useState<'unix' | 'windows'>('unix');
  const [showHeadless, setShowHeadless] = useState(false);
  const cmd = os === 'unix' ? UNIX : WIN;

  return (
    <div className="rounded-xl border border-border bg-card shadow-sm overflow-hidden">
      <div className="flex border-b border-border">
        {(['unix', 'windows'] as const).map((o) => (
          <button
            key={o}
            type="button"
            onClick={() => { setOs(o); setShowHeadless(false); }}
            className={`flex-1 px-4 py-2.5 text-sm font-medium transition-colors ${
              os === o
                ? 'bg-accent/10 text-accent border-b-2 border-accent -mb-px'
                : 'text-muted-foreground hover:text-foreground hover:bg-muted/50'
            }`}
          >
            {o === 'unix' ? 'macOS / Linux' : 'Windows'}
          </button>
        ))}
      </div>
      <div className="p-4">
        <div className="flex items-center gap-3 rounded-lg bg-foreground/5 border border-border p-3">
          <code className="flex-1 overflow-x-auto font-mono text-sm">{cmd}</code>
          <CopyButton text={cmd} onCopied={() => track({ name: 'curl_copy', os })} />
        </div>
        <p className="mt-3 text-xs text-muted-foreground">
          Detects installed tools and configures memory + skills for all of them.
        </p>

        {os === 'unix' && (
          <div className="mt-3">
            <button
              type="button"
              onClick={() => setShowHeadless((v) => !v)}
              className="text-xs text-muted-foreground hover:text-foreground underline underline-offset-2 transition-colors"
            >
              {showHeadless ? 'Hide' : 'Need a non-interactive version?'}
            </button>
            {showHeadless && (
              <div className="mt-2 space-y-1.5">
                <div className="flex items-center gap-3 rounded-lg bg-foreground/5 border border-border p-3">
                  <code className="flex-1 overflow-x-auto font-mono text-xs">{UNIX_HEADLESS}</code>
                  <CopyButton
                    text={UNIX_HEADLESS}
                    onCopied={() => track({ name: 'curl_copy', os: 'unix-headless' })}
                  />
                </div>
                <p className="text-xs text-muted-foreground leading-relaxed">
                  Non-interactive: auto-configures detected editors without prompts.
                  Use this for CI pipelines or scripted installs (<code className="font-mono text-foreground bg-muted px-1 py-0.5 rounded">-y</code> flag).
                </p>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Verify `analytics.ts` accepts `os: 'unix-headless'`**

  Open `src/lib/analytics.ts` and check the `CurlCopyEvent` type. The existing type likely has `os: 'unix' | 'windows'`. Extend it to `os: 'unix' | 'unix-headless' | 'windows'` so TypeScript does not error.

  If `analytics.ts` uses a union type for the `curl_copy` event's `os` field, the edit is:
  ```ts
  // Before:
  os: 'unix' | 'windows'
  // After:
  os: 'unix' | 'unix-headless' | 'windows'
  ```
  If the event type is `Record<string, unknown>` or already permissive, no change needed — confirm by running `pnpm build`.

- [ ] **Step 4: Build check**

  ```bash
  cd /home/novusedge/Projects/delta-prime/web/join && pnpm build 2>&1 | tail -10
  ```
  Expected: no TypeScript errors on `curl-box.tsx` or `analytics.ts`. Fix any type errors before proceeding.

---

### Task 2: config-panel — remove dead skills curl; replace Step 2 copy

**File:** `src/components/config-panel.tsx`

**Current Step 2 block (lines 87–109):**
```tsx
{/* STEP 2: Install Skills (Highly Recommended) */}
<div className="space-y-3">
  <div className="flex items-center gap-2">
    <div className="flex size-6 items-center justify-center rounded-full bg-accent/25 text-accent text-xs font-bold">
      2
    </div>
    <h3 className="font-semibold text-sm text-foreground">Step 2: Install Skills (Recommended)</h3>
  </div>
  <div className="pl-8 space-y-3">
    <p className="text-xs text-muted-foreground leading-relaxed">
      Skills add advanced agent workflows like <code ...>/engrammic-debug</code>, <code ...>/engrammic-checkpoint</code>, etc.
    </p>
    <div className="flex items-center gap-2 rounded-lg border border-border bg-background p-2.5">
      <code className="flex-1 overflow-x-auto font-mono text-[11px] text-foreground">
        curl -fsSL get.engrammic.ai/skills | bash
      </code>
      <CopyButton text="curl -fsSL get.engrammic.ai/skills | bash" onCopied={...} />
    </div>
  </div>
</div>
```

The `get.engrammic.ai/skills` endpoint is cut (spec Decision). Remove the dead curl entirely and replace Step 2 with copy that correctly explains where skills come from.

**New Step 2 copy:** "The installer already handles skills — if you ran the curl command, you are done. If you already have the CLI and just want to (re-)install skills, run `engrammic skills` in your terminal."

This removes the import of `Terminal` (check whether it is still used — if only used in the dead block, remove the import too). The `CopyButton` import and the `mcpDone` state remain (used in Step 1).

- [ ] **Step 1: Replace the Step 2 block in `src/components/config-panel.tsx`.**

  Find the block between the first `<div className="border-t border-border/60 my-4" />` and the second one (i.e., the entire Step 2 div, lines 87–109 approximately). Replace it with:

  ```tsx
  {/* STEP 2: Skills — handled by the installer */}
  <div className="space-y-3">
    <div className="flex items-center gap-2">
      <div className="flex size-6 items-center justify-center rounded-full bg-accent/25 text-accent text-xs font-bold">
        2
      </div>
      <h3 className="font-semibold text-sm text-foreground">Step 2: Skills</h3>
    </div>
    <div className="pl-8 space-y-2">
      <p className="text-xs text-muted-foreground leading-relaxed">
        Skills (advanced workflows like{' '}
        <code className="font-mono text-foreground bg-muted px-1 py-0.5 rounded">/engrammic-debug</code>
        {', '}
        <code className="font-mono text-foreground bg-muted px-1 py-0.5 rounded">/engrammic-checkpoint</code>
        , etc.) are installed automatically by the main installer. If you ran the curl command above, you are already done.
      </p>
      <p className="text-xs text-muted-foreground leading-relaxed">
        Already have the CLI and want to add or refresh skills?
      </p>
      <div className="flex items-center gap-2 rounded-lg border border-border bg-background p-2.5">
        <code className="flex-1 font-mono text-[11px] text-foreground">engrammic skills</code>
        <CopyButton
          text="engrammic skills"
          onCopied={() => track({ name: 'copy_cli', harness: harness.id })}
        />
      </div>
    </div>
  </div>
  ```

- [ ] **Step 2: Clean up now-unused imports.**

  Check the `Terminal` import at line 6: `import { Terminal, Zap, Check } from 'lucide-react';`
  `Terminal` is not used anywhere else in `config-panel.tsx`. Remove it:
  ```tsx
  // Before:
  import { Terminal, Zap, Check } from 'lucide-react';
  // After:
  import { Zap, Check } from 'lucide-react';
  ```
  (`Zap` is used in the deep-link button; `Check` is used in the step-1 number badge.)

- [ ] **Step 3: Build check**

  ```bash
  cd /home/novusedge/Projects/delta-prime/web/join && pnpm build 2>&1 | tail -10
  ```
  Expected: no errors. Confirm no remaining reference to `get.engrammic.ai/skills` anywhere in `src/`:
  ```bash
  grep -r "get.engrammic.ai/skills" /home/novusedge/Projects/delta-prime/web/join/src/
  ```
  Expected: no output.

---

### Task 3: Per-editor restart hints — audit and fix inconsistencies

**File:** `src/lib/harnesses.ts` (read-only audit + targeted fixes if gaps found)

The restart hints were pre-audited during this plan's research. The full 22-harness list with hints follows. No hint is blank.

**Audit findings — inconsistencies to fix:**

1. **`claude` (Claude Code):** `'Start a new session'` — correct (Claude Code is a CLI; there is no window reload).
2. **`claude-desktop` (Claude Desktop):** `'Quit and reopen the app'` — correct.
3. **`cursor`:** `'Cmd/Ctrl+Shift+P → "Reload Window"'` — correct; consistent with vscode.
4. **`windsurf`:** `'Cmd/Ctrl+Shift+P → "Reload Window"'` — correct.
5. **`antigravity`:** `'Cmd/Ctrl+Shift+P → "Reload Window"'` — correct.
6. **`gemini`:** `'Start a new session'` — correct (CLI).
7. **`pi`:** `'Start a new session'` — correct.
8. **`copilot`:** `'Start a new session'` — correct.
9. **`codex`:** `'Start a new session'` — correct.
10. **`vscode`:** `'Cmd/Ctrl+Shift+P → "Reload Window"'` — correct.
11. **`goose`:** `'Start a new session'` — correct.
12. **`amp`:** `'Start a new session'` — correct.
13. **`opencode`:** `'Start a new session'` — correct.
14. **`amazonq`:** `'Start a new session'` — correct.
15. **`zed`:** `'Quit and reopen the app'` — correct.
16. **`kiro`:** `'Start a new session'` — correct.
17. **`junie`:** `'Start a new session'` — correct.
18. **`jetbrains`:** `'Restart the IDE'` — correct.
19. **`cline`:** `'Cmd/Ctrl+Shift+P → "Reload Window"'` — correct (VS Code extension).
20. **`roo`:** `'Cmd/Ctrl+Shift+P → "Reload Window"'` — correct (VS Code extension).
21. **`continue`:** `'Cmd/Ctrl+Shift+P → "Reload Window"'` — correct (VS Code extension).
22. **`trae`:** `'Restart the app'` — acceptable; no inconsistency with `jetbrains`.

**Verdict:** All restart hints are present and internally consistent. Three groups are used:
- CLI tools: `'Start a new session'`
- VS Code–based (including Cursor, Windsurf, Antigravity, Cline, Roo, Continue): `'Cmd/Ctrl+Shift+P → "Reload Window"'`
- Desktop apps (Claude Desktop, Zed, Trae, JetBrains): `'Quit and reopen the app'` / `'Restart the app'` / `'Restart the IDE'`

**No edits needed to `harnesses.ts` for restart hints.** If the implementer finds a hint missing on a future harness addition, follow the three-group pattern above.

---

### Task 4: Copy pass — install-hero.tsx

**File:** `src/components/install-hero.tsx`

Apply plain-language copy changes to the three paths visible in this component.

#### 4.1 One-command path (Section 1)

| Location | Current string | New string | Rationale |
|---|---|---|---|
| `<h2>` line 37 | `"One terminal command, configure everything"` | `"One command sets everything up"` | Shorter; "terminal command" is jargon to non-technical users — they know they paste in a terminal; no need to say so. |
| `<p>` subheading line 38–40 | `"Auto-detects your tools, registers the MCP server, and installs advanced workflows/skills."` | `"Finds your editors automatically, connects Engrammic's memory server, and installs ready-made workflows (skills) — no manual steps."` | Plain language; "MCP server" parenthesized as "memory server"; "skills" explained inline; "auto-detects" → "finds ... automatically". |

- [ ] **Step 1: Edit the `<h2>` text at line 37:**

  Old:
  ```tsx
  <h2 className="text-3xl font-bold tracking-tight">One terminal command, configure everything</h2>
  ```
  New:
  ```tsx
  <h2 className="text-3xl font-bold tracking-tight">One command sets everything up</h2>
  ```

- [ ] **Step 2: Edit the `<p>` subheading at lines 38–40:**

  Old:
  ```tsx
  <p className="mt-3 text-base text-muted-foreground max-w-xl mx-auto leading-relaxed">
    Auto-detects your tools, registers the MCP server, and installs advanced workflows/skills.
  </p>
  ```
  New:
  ```tsx
  <p className="mt-3 text-base text-muted-foreground max-w-xl mx-auto leading-relaxed">
    Finds your editors automatically, connects Engrammic&apos;s memory server, and installs
    ready-made workflows (skills) — no manual steps.
  </p>
  ```

#### 4.2 Pick-your-editor path (Section 2)

| Location | Current string | New string | Rationale |
|---|---|---|---|
| `<h3>` line 58 | `"Select your editor"` | `"Or set it up for a specific editor"` | Surfaces the "or" relationship to the one-command path; less terse. |
| `<p>` line 59–61 | `"Connect the MCP server manually or with 1-click"` | `"Connect Engrammic manually or in one click — then restart your editor to finish."` | "MCP server" → "Engrammic"; adds the restart reminder that users miss; "1-click" normalized to "one click". |

- [ ] **Step 3: Edit the `<h3>` at line 58:**

  Old:
  ```tsx
  <h3 className="text-xl font-bold">Select your editor</h3>
  ```
  New:
  ```tsx
  <h3 className="text-xl font-bold">Or set it up for a specific editor</h3>
  ```

- [ ] **Step 4: Edit the `<p>` at lines 59–61:**

  Old:
  ```tsx
  <p className="text-sm text-muted-foreground mt-1">
    Connect the MCP server manually or with 1-click
  </p>
  ```
  New:
  ```tsx
  <p className="text-sm text-muted-foreground mt-1">
    Connect Engrammic manually or in one click — then restart your editor to finish.
  </p>
  ```

#### 4.3 Self-host callout (Section 3)

| Location | Current string | New string | Rationale |
|---|---|---|---|
| Callout headline | `"Need to self-host for compliance?"` | `"Need to run it on your own machine or server?"` | "self-host" and "compliance" are jargon; the plain question covers the actual reasons (privacy, org policy, offline). |
| Callout body | `"See our Docker Compose setup to run your own local Engrammic memory server."` | `"Run Engrammic entirely on your own hardware using Docker — no data leaves your machine. License required."` | Explains the benefit (data stays local); names Docker explicitly; adds the license requirement so users aren't surprised. |
| Link text | `"Docker Compose Setup"` | `"Self-hosting guide →"` | Less technical; matches the docs link destination. |

- [ ] **Step 5: Edit the self-host callout block (lines 172–189):**

  Old:
  ```tsx
  <p className="font-bold text-sm text-foreground">Need to self-host for compliance?</p>
  <p className="text-xs text-muted-foreground mt-0.5">
    See our Docker Compose setup to run your own local Engrammic memory server.
  </p>
  ```
  New:
  ```tsx
  <p className="font-bold text-sm text-foreground">Need to run it on your own machine or server?</p>
  <p className="text-xs text-muted-foreground mt-0.5">
    Run Engrammic entirely on your own hardware using Docker — no data leaves your machine.
    License required.
  </p>
  ```

  Old link text:
  ```tsx
  Docker Compose Setup
  ```
  New link text:
  ```tsx
  Self-hosting guide
  ```

- [ ] **Step 6: Build check**

  ```bash
  cd /home/novusedge/Projects/delta-prime/web/join && pnpm build 2>&1 | tail -10
  ```

---

### Task 5: Doc housekeeping — archive JOIN-UX-SPEC.md

**File:** `JOIN-UX-SPEC.md` (confirmed absent from the directory — the file does not exist at `/home/novusedge/Projects/delta-prime/web/join/JOIN-UX-SPEC.md`).

**Note from audit:** `ls` of `web/join/` shows `PLAN-onboarding-improvements.md` but **no** `JOIN-UX-SPEC.md`. The spec references it being archived; it was either never committed to this repo or was already deleted. Either way, no file exists to add a banner to.

**Action:**

- [ ] **Step 1: Verify absence**

  ```bash
  ls /home/novusedge/Projects/delta-prime/web/join/JOIN-UX-SPEC.md 2>&1
  ```
  If the file is absent (expected: `No such file or directory`), this task is complete — note in the commit message that JOIN-UX-SPEC.md was not present in the working tree.

  If (unexpectedly) the file exists, add the following banner as the first two lines:

  ```markdown
  > **ARCHIVED — 2026-06-10.** This document is superseded by the installer overhaul spec:
  > `mcp-client/docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md` (§ "Join-page alignment"). Do not edit this file; edit the spec instead.
  ```

---

### Task 6: Doc housekeeping — update PLAN-onboarding-improvements.md

**File:** `PLAN-onboarding-improvements.md`

Add a banner at the top and annotate the Implementation Order section to distinguish items superseded by the installer overhaul from items that remain valid and undone.

- [ ] **Step 1: Add a banner as the first three lines of the file, before the existing `# Onboarding Flow Improvements` heading:**

  ```markdown
  > **STATUS — 2026-06-10:** This plan is partially superseded by the installer overhaul spec
  > (`mcp-client/docs/superpowers/specs/2026-06-10-installer-onboarding-overhaul-design.md`).
  > Items marked **[SUPERSEDED]** below are covered by the overhaul; items marked **[VALID]** are still pending.
  ```

- [ ] **Step 2: Annotate the Implementation Order section (§ "Implementation Order", lines 146–165).**

  The items and their status:

  **Phase 1: Quick wins**
  - `[ ] Add restart reminder to CLI output` → **[SUPERSEDED]** — covered by installer overhaul Phase 1b (print_restart_reminder at end of execute flow).
  - `[ ] Add Claude Desktop to CLI` → **[SUPERSEDED]** — `claude-desktop` harness is already shipped in `harnesses.ts` and `installer-cli`; visible in the join page. Done in earlier work.
  - `[ ] Add "Step 2: Skills" + restart instructions to join.engrammic.ai` → **[SUPERSEDED]** — Phase 5 (this plan) replaces the dead skills curl and keeps the restart hints; the original "Step 2" curl is cut because `get.engrammic.ai/skills` is cut.

  **Phase 2: Skills-only installer**
  - `[ ] Add engrammic skills subcommand` → **[VALID]** — the `engrammic skills` CLI subcommand is still needed (referenced in the new config-panel copy added in Task 2). Not yet shipped per installer-cli audit.
  - `[ ] Create install-skills.sh script` → **[SUPERSEDED]** — `get.engrammic.ai/skills` endpoint is cut by spec Decision; the separate script is not needed. Skills are handled by the main installer.
  - `[ ] Deploy to get.engrammic.ai/skills` → **[SUPERSEDED]** — endpoint cut; do not implement.

  **Phase 3: CLI UX overhaul**
  - `[ ] Flip default to non-interactive` → **[SUPERSEDED]** — the overhaul spec explicitly rejects this: "interactive by default, -y for auto" is kept; flags pre-answer the interview rather than flipping the default. See spec Decisions and Architecture §1.
  - `[ ] Clean up output formatting` → **[SUPERSEDED]** — covered by installer Phase 1b (plan-summary + per-step results table).
  - `[ ] Update -y → -i flag semantics` → **[SUPERSEDED]** — explicitly rejected by spec; `-y` stays as the non-interactive flag.

  **Phase 4: join.engrammic.ai redesign**
  - `[ ] Reorder hero (curl first)` → **[SUPERSEDED]** — curl is already the hero (implementation matched PLAN over JOIN-UX-SPEC). No reorder needed.
  - `[ ] Add post-install flow component` → **[SUPERSEDED]** — covered by Phase 5 config-panel update (Skills step updated; restart hint already in Step 3).
  - `[ ] Per-harness restart instructions` → **[SUPERSEDED]** — all 22 harnesses have `restartHint` in `harnesses.ts`; ConfigPanel Step 3 renders them. Done.

  **Still-valid items summary:**
  Only `engrammic skills` subcommand (Phase 2, item 4) remains a genuine open item — it is needed because the new config-panel copy (Task 2) references `engrammic skills` as the re-install command for users who already have the CLI.

  The edit to `PLAN-onboarding-improvements.md`: prepend the banner (Step 1) and inline the annotations next to each checkbox item in the Implementation Order section.

  The annotated Implementation Order section should read:

  ```markdown
  ## Implementation Order

  ### Phase 1: Quick wins (low effort, high impact)
  1. [x] Add restart reminder to CLI output — **[SUPERSEDED]** installer overhaul Phase 1b
  2. [x] Add Claude Desktop to CLI — **[SUPERSEDED]** already shipped
  3. [x] Add "Step 2: Skills" + restart instructions to join.engrammic.ai — **[SUPERSEDED]** Phase 5 (skills curl cut; hints kept)

  ### Phase 2: Skills-only installer
  4. [ ] Add `engrammic skills` subcommand — **[VALID]** still open; referenced by new join-page copy
  5. ~~[ ] Create `install-skills.sh` script~~ — **[SUPERSEDED]** `get.engrammic.ai/skills` endpoint cut
  6. ~~[ ] Deploy to `get.engrammic.ai/skills`~~ — **[SUPERSEDED]** endpoint cut; do not implement

  ### Phase 3: CLI UX overhaul
  7. ~~[ ] Flip default to non-interactive~~ — **[SUPERSEDED]** rejected by overhaul spec; `-y` stays as non-interactive flag
  8. [x] Clean up output formatting — **[SUPERSEDED]** installer Phase 1b plan-summary output
  9. ~~[ ] Update `-y` → `-i` flag semantics~~ — **[SUPERSEDED]** explicitly rejected by overhaul spec

  ### Phase 4: join.engrammic.ai redesign
  10. [x] Reorder hero (curl first) — **[SUPERSEDED]** already implemented before this plan
  11. [x] Add post-install flow component — **[SUPERSEDED]** Phase 5 config-panel update
  12. [x] Per-harness restart instructions — **[SUPERSEDED]** all 22 harnesses have restartHint; ConfigPanel Step 3 renders them
  ```

- [ ] **Step 3: Make the edits to `PLAN-onboarding-improvements.md`.**

  Use the Edit tool twice: once to prepend the banner block, once to replace the Implementation Order section with the annotated version above.

---

### Task 7: Tests — add curl-box constants test

**File:** `src/components/curl-box.test.ts` (new file)

The existing test suite (`src/**/*.test.ts`) covers `analytics`, `catalog`, and `harnesses` — all pure logic files. Vitest runs in `node` environment; no DOM. The `CurlBox` React component itself requires a DOM to render, so component render tests are out of scope for the current `vitest.config.ts` (environment: `node`, no `@testing-library`). **The correct approach for this project** is to export the constants from `curl-box.tsx` (done in Task 1) and test those directly — same pattern used in `harnesses.test.ts` which tests exported constants rather than rendered output.

The plan specifically requires:
1. A test asserting the rendered hero contains the canonical curl command — satisfied here by asserting the exported `UNIX` constant equals the canonical URL (the constant IS what renders; if it changes, the test fails).
2. A test asserting the headless variant toggle exists — satisfied by asserting `UNIX_HEADLESS` is exported and contains `-s -- -y`.

- [ ] **Step 1: Create `src/components/curl-box.test.ts`:**

  ```ts
  import { describe, it, expect } from 'vitest';
  import { UNIX, WIN, UNIX_HEADLESS } from './curl-box';

  describe('curl-box constants', () => {
    it('UNIX hero command uses the canonical install.sh URL with sh (not bash)', () => {
      expect(UNIX).toBe('curl -fsSL https://get.engrammic.ai/install.sh | sh');
    });

    it('UNIX hero command domain matches the Phase 2 canonical host', () => {
      expect(UNIX).toContain('get.engrammic.ai/install.sh');
    });

    it('UNIX_HEADLESS is the -y variant of the UNIX command', () => {
      // Must contain the same URL as the hero command
      expect(UNIX_HEADLESS).toContain('get.engrammic.ai/install.sh');
      // Must pass args through sh -s --
      expect(UNIX_HEADLESS).toContain('sh -s -- -y');
    });

    it('UNIX_HEADLESS is distinct from UNIX (headless toggle adds the -y flag)', () => {
      expect(UNIX_HEADLESS).not.toBe(UNIX);
    });

    it('WIN command is the PowerShell one-liner', () => {
      expect(WIN).toBe('irm https://get.engrammic.ai/install.ps1 | iex');
    });
  });
  ```

- [ ] **Step 2: Run tests**

  ```bash
  cd /home/novusedge/Projects/delta-prime/web/join && pnpm test 2>&1
  ```
  Expected: existing 9 tests pass; new 5 tests pass. Total: 14 tests, 0 failures.

  If the vitest import of a `.tsx` file fails (`.tsx` files are not in the `include` glob `src/**/*.test.ts`), update `vitest.config.ts` to also include `.test.tsx` files:
  ```ts
  // vitest.config.ts — only needed if tsx imports fail
  include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
  ```
  In this case the test file itself is `.ts` (importing from `.tsx`), which should work without config changes — vitest can import `.tsx` as a dependency of a `.ts` test file when using the default Vite transform. If not, add a `resolve.extensions` or `plugins: [react()]` entry per the vitest React docs. Preferred: keep the test file as `.ts`; add `@vitejs/plugin-react` only if the import fails.

---

### Task 8: Verification and commits

All commands run from `/home/novusedge/Projects/delta-prime/web/join`.

- [ ] **Step 1: Final test run**

  ```bash
  pnpm test 2>&1
  ```
  Expected: all tests pass (including the 5 new curl-box tests).

- [ ] **Step 2: Final build**

  ```bash
  pnpm build 2>&1 | tail -15
  ```
  Expected: `✓ Compiled successfully` (or Next.js static export success). No TypeScript errors. No ESLint errors that block build.

- [ ] **Step 3: Grep for dead endpoint (belt-and-suspenders)**

  ```bash
  grep -r "get.engrammic.ai/skills" /home/novusedge/Projects/delta-prime/web/join/src/
  ```
  Expected: no output.

  ```bash
  grep -r "| bash" /home/novusedge/Projects/delta-prime/web/join/src/
  ```
  Expected: no output (all `| bash` usages replaced with `| sh`).

- [ ] **Step 4: Commit — component changes (Tasks 1 + 2 + 4)**

  Run from `/home/novusedge/Projects/delta-prime/web` (repo root):

  ```bash
  git add join/src/components/curl-box.tsx join/src/components/config-panel.tsx join/src/components/install-hero.tsx
  git commit -m "$(cat <<'EOF'
  feat(join): add headless curl toggle, remove dead skills curl, plain-language copy

  - curl-box: fix bash → sh in hero command; export UNIX/WIN/UNIX_HEADLESS constants;
    add collapsible headless/CI toggle showing | sh -s -- -y with one-line explanation
  - config-panel: remove dead get.engrammic.ai/skills curl (endpoint cut); replace Step 2
    with copy pointing at the main installer and engrammic skills for existing CLI users
  - install-hero: plain-language copy pass across one-command, pick-your-editor, and
    self-host sections; jargon (MCP server, self-host, compliance) parenthesized or replaced

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
  EOF
  )"
  ```

- [ ] **Step 5: Commit — tests (Task 7)**

  ```bash
  git add join/src/components/curl-box.test.ts
  git commit -m "$(cat <<'EOF'
  test(join): add curl-box constant tests for canonical URL and headless toggle

  Asserts UNIX hero command matches get.engrammic.ai/install.sh with sh (not bash),
  UNIX_HEADLESS carries -s -- -y, and WIN is the PowerShell one-liner.

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
  EOF
  )"
  ```

- [ ] **Step 6: Commit — doc housekeeping (Tasks 5 + 6)**

  ```bash
  git add join/PLAN-onboarding-improvements.md
  # Only include JOIN-UX-SPEC.md if it was created/modified in Task 5
  git commit -m "$(cat <<'EOF'
  docs(join): annotate PLAN-onboarding-improvements with overhaul supersessions

  Mark items superseded by the installer overhaul spec (get.engrammic.ai/skills cut,
  -y/-i semantics, auto-default flip all rejected/covered). One still-valid item
  remains: engrammic skills subcommand (referenced by new config-panel copy).
  JOIN-UX-SPEC.md was absent from the working tree; noted in commit message.

  Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>
  EOF
  )"
  ```

---

### Checklist summary

| Task | File(s) | Status |
|---|---|---|
| 1 | `src/components/curl-box.tsx` | `[ ]` |
| 1 (type fix) | `src/lib/analytics.ts` | `[ ]` |
| 2 | `src/components/config-panel.tsx` | `[ ]` |
| 3 | `src/lib/harnesses.ts` | no changes needed |
| 4 | `src/components/install-hero.tsx` | `[ ]` |
| 5 | `JOIN-UX-SPEC.md` | absent from tree; verify only |
| 6 | `PLAN-onboarding-improvements.md` | `[ ]` |
| 7 | `src/components/curl-box.test.ts` | `[ ]` (new file) |
| 8 | verification + 3 commits in `web` repo | `[ ]` |

# Coming Soon

## Session UX Rework
- Goal: make sessions effortless and discoverable.
- Highlights:
  - Auto‑resume last session with clear indicator and quick switcher.
  - `:session list` and `:session rename <new>`; human‑readable names by default.
  - Auto‑save on every turn; optional autosnapshot before risky tool calls.
  - Visual session breadcrumb and quick actions (new, switch, rename, export).
  - Import/export sessions (JSON/Markdown) and merge histories.

## Bottom Bar (Status + Shortcuts)
- Always‑visible footer showing live state and controls:
  - Elapsed time while thinking (e.g., 19s).
  - Tokens: input, output, total; provider‑reported when available, fallback to estimates.
  - Context remaining: approximate tokens left before trim.
  - Mode: stream on/off, tools on/off, provider:model.
- Keyboard shortcuts (tentative):
  - Ctrl+C: cancel generation
  - Ctrl+J: insert newline in prompt
  - Ctrl+T: open transcript/export menu
  - Ctrl+S: session switcher
  - Ctrl+M: model picker
  - Ctrl+R: toggle stream
  - Ctrl+K: open key manager

## Notes
- Token usage will be sourced from provider responses when available; otherwise we’ll estimate using the active tokenizer.
- Context remaining is an estimate and will reflect any configured reserve buffer.
- We’ll keep a plain REPL fallback for minimal environments; the bottom bar will be opt‑in/off by flag or config.

If you have specific shortcuts or layout preferences, add them to this file and we’ll incorporate them into the first pass.

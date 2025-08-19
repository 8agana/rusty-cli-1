# Repository Guidelines

## Project Structure & Modules
- `src/main.rs`: CLI entry using `clap`; wires commands to handlers.
- `src/api.rs`: DeepSeek HTTP client (streaming and non-streaming).
- `src/chat.rs`: Interactive chat loop with history and system prompt.
- `src/chat_with_tools.rs`: Chat with tool-calling orchestration.
- `src/tools.rs`: Built-in tools (`shell`, `calculator`, `read_file`, `write_file`) and registry.
- `src/config.rs`: TOML config I/O at `~/.config/deepseek-cli/config.toml`.
- `src/mcp.rs`: Optional MCP client/tool wrapping.
- No test directory yet; prefer `tests/` for integration tests.

## Build, Test, and Development
- Build: `cargo build` (debug) or `cargo build --release` (optimized).
- Run: `cargo run -- chat "Hello"` or install with `cargo install --path .` then `deepseek-cli ...`.
- Format: `cargo fmt --all` (use before PRs).
- Lint: `cargo clippy --all-targets -- -D warnings`.
- Test: `cargo test` (add tests as described below).

## Coding Style & Naming
- Rust 2021; 4-space indent; keep functions small and async-aware.
- Names: modules/files `snake_case`; types/traits `PascalCase`; fns/vars `snake_case`; consts `SCREAMING_SNAKE_CASE`.
- Errors: return `anyhow::Result<T>`; use `?` and informative messages for API failures.
- Streaming: avoid blocking I/O; flush stdout when streaming chunks.

## Testing Guidelines
- Unit tests: add `#[cfg(test)] mod tests { ... }` near logic.
- Integration tests: place files under `tests/` (e.g., `tests/chat_flow.rs`).
- Names: end with `_test.rs` or `_tests.rs` for modules; use realistic prompts and mock boundaries where possible.
- Run all: `cargo test`; ensure tools with side effects are guarded or mocked.

## Commit & Pull Requests
- Commits: use Conventional Commits (e.g., `feat(cli): add --tools flag`, `fix(api): handle SSE errors`).
- PRs must include: clear description, rationale, examples (commands/output), and mention of docs/README updates.
- Pre-merge checklist: `cargo fmt`, `cargo clippy -D warnings`, `cargo build`, and `cargo test` pass.

## Security & Configuration
- Never commit secrets. Use `DEEPSEEK_API_KEY` or `deepseek-cli config set api-key "..."`.
- Tool safety: `shell` executes arbitrary commands—avoid destructive examples and validate inputs for new tools.
- Network: base URL defaults to `https://api.deepseek.com`; make it configurable only via config/flags.

## Architecture Overview
- Flow: `main` → parse CLI → create `DeepSeekClient` → route to `chat` or `chat_with_tools`.
- Extending tools: implement `ToolExecutor`, register in `ToolRegistry`, and expose JSON schema in tool definition.


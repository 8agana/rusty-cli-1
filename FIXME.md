# FIXME / Improvements

- Tools API correctness: `ToolExecutor` defines `async fn execute`, but several implementations are sync and `ToolRegistry::execute` isn’t `async`. Unify by making all tools async (use `tokio::process::Command` and `tokio::fs`) and change call sites to `await`.
  - In `tools.rs`: make `ToolRegistry::execute` `async` and implement `as_any` for every tool or remove downcasting needs.
  - In `chat_with_tools.rs`: `let result = registry.execute(name, args).await?;`.

- Avoid blocking I/O in async paths: `chat.rs` and `chat_with_tools.rs` use `std::io` for input/flush. Switch to `tokio::io` for reads and buffered writes to prevent blocking the runtime.

- Safer API key printing: `config get ApiKey` slices can panic on short keys. Guard length:
  - Example: `let shown = if k.len() > 10 { format!("{}...{}", &k[..6], &k[k.len()-4..]) } else { "***".into() };`

- HTTP client hardening: build `reqwest::Client` with timeouts, UA, and retries/backoff.
  - Example: `Client::builder().timeout(Duration::from_secs(60)).user_agent("deepseek-cli/0.1.0").build()?`.

- Streaming robustness: handle SSE keepalives and partial JSON frames; surface stream errors clearly and optionally fall back to non-streaming with a flag.

- CLI polish:
  - Add `-v/--verbose` (map to `tracing` levels) and `--base-url` override.
  - Global `--temperature` and read defaults from config when omitted.
  - `models` should query the API or cache a fetched list, not hard-code.

- Logging: `tracing` is a dependency but isn’t initialized. Add `tracing_subscriber` setup (env-driven level) and instrument key paths (`api`, streaming, tool exec).

- Install script mismatch: `install.sh` installs binary as `deepseek` but README refers to `deepseek-cli`. Align names or add a symlink/README note.

- Tool safety: `shell` executes arbitrary commands. Provide a "safe mode" (denylist/allowlist), confirmation prompts for destructive ops, and an option to disable shell by default.

- Calculator dependency: uses external `bc` (may be absent on Windows). Replace with a Rust expression evaluator crate or document requirement and add runtime check.

- MCP integration:
  - Wire `mcp_config.toml` into a CLI subcommand to spawn servers and register tools dynamically.
  - Ensure JSON-RPC framing is resilient (multi-line payloads) and add graceful shutdown.

- Testing & CI:
  - Add unit tests for `api` non-streaming, and integration tests for basic chat/tool flows. Use `wiremock`/`httptest` to mock DeepSeek endpoints.
  - Set up GitHub Actions: `cargo fmt --check`, `clippy -D warnings`, `build`, `test`.

- Config enhancements: allow persisting `base_url` and default `temperature`. Validate config file permissions and create with 0600 on Unix.

- Metadata & docs: add `LICENSE`, contribution guidelines, shell completions (`clap_complete`), and tone down informal phrasing in README.

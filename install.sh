#!/usr/bin/env bash
set -euo pipefail

echo "Building rusty-cli (release)..."
cargo build --release

BIN_SRC="target/release/rusty-cli"
BIN_DST="/usr/local/bin/rusty-cli"
ALIAS_DST="/usr/local/bin/rustycli"

if [[ ! -f "$BIN_SRC" ]]; then
  echo "Error: binary not found at $BIN_SRC" >&2
  exit 1
fi

echo "Installing to /usr/local/bin (sudo required)..."
sudo install -m 0755 "$BIN_SRC" "$BIN_DST"
sudo ln -sf "$BIN_DST" "$ALIAS_DST"

# Provider shims for convenience (no flags to remember)
create_shim() {
  local name=$1
  local provider=$2
  local path="/usr/local/bin/$name"
  sudo bash -c "cat > '$path'" <<SHIM
#!/usr/bin/env bash
exec "$BIN_DST" --provider $provider "$@"
SHIM
  sudo chmod 0755 "$path"
  echo "  shim -> $path (provider=$provider)"
}

echo "Creating provider shims..."
create_shim codex openai
create_shim claude openai   # switch to anthropic later if desired
create_shim grok grok
create_shim groq groq

echo "Installed:"
echo "  $BIN_DST"
echo "  alias -> $ALIAS_DST"
echo "  shims -> /usr/local/bin/{codex,claude,grok,groq}"
echo
echo "Try: rustycli --help"

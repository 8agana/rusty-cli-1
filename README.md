# DeepSeek CLI - Rust Edition

A fast, reliable CLI wrapper for the DeepSeek API written in Rust.

## Features

- ✅ Streaming responses for real-time output
- ✅ Interactive chat mode with history
- ✅ Configuration file support
- ✅ Multiple models (chat, coder, reasoner)
- ✅ System prompts
- ✅ Temperature control

## Installation

Quick install to use `rustycli` from anywhere:

```bash
./install.sh
```

This builds a release binary and installs it to `/usr/local/bin/rusty-cli` with a convenience alias `/usr/local/bin/rustycli`.

Manual install:

```bash
cargo build --release
sudo install -m 0755 target/release/rusty-cli /usr/local/bin/rusty-cli
sudo ln -sf /usr/local/bin/rusty-cli /usr/local/bin/rustycli
```

## Development

Run formatting, lints, and checks before committing:

```
# Format
cargo fmt

# Lint
cargo clippy --all-targets --all-features -D warnings

# Fast type-check
cargo check

# Build release
cargo build --release
```

## Configuration

Set your API key in one of three ways:

1. Environment variable:
```bash
export DEEPSEEK_API_KEY="your-api-key"
```

2. Command line flag:
```bash
rustycli --api-key "your-api-key" chat "Hello"
```

3. Config file:
```bash
rustycli config set api-key "your-api-key"
```

## Usage

### Quick chat
```bash
rustycli chat "What is Rust?"
```

### Interactive mode
```bash
rustycli chat --interactive
# or just
rustycli
```

### With system prompt
```bash
rustycli chat -s "You are a helpful coding assistant" "Write a Python hello world"
```

### Different models
```bash
rustycli -m deepseek-coder chat "Explain this code: fn main() {}"
rustycli -m deepseek-reasoner chat "Solve: 2x + 5 = 15"
```

### No streaming (wait for complete response)
```bash
rustycli --no-stream chat "Tell me a joke"
```

## Commands

- `chat [message]` - Send a message or start interactive mode
- `config set <key> <value>` - Set configuration values
- `config get [key]` - Get configuration values
- `models` - List available models

## Interactive Mode Commands

- `exit` or `quit` - End the session
- `clear` - Clear chat history
- `system <prompt>` - Set a new system prompt

## Why Rust?

Because the other DeepSeek CLIs are "bullshit" and we can do better. This one is:
- Fast (compiled, not interpreted)
- Memory efficient
- Properly handles streaming
- Has actual error messages
- Won't randomly break

Built by CC for Sam - because testing what's out there matters.

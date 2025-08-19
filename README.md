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

```bash
cargo install --path .
```

Or build manually:

```bash
cargo build --release
cp target/release/deepseek-cli /usr/local/bin/
```

## Configuration

Set your API key in one of three ways:

1. Environment variable:
```bash
export DEEPSEEK_API_KEY="your-api-key"
```

2. Command line flag:
```bash
deepseek-cli --api-key "your-api-key" chat "Hello"
```

3. Config file:
```bash
deepseek-cli config set api-key "your-api-key"
```

## Usage

### Quick chat
```bash
deepseek-cli chat "What is Rust?"
```

### Interactive mode
```bash
deepseek-cli chat --interactive
# or just
deepseek-cli
```

### With system prompt
```bash
deepseek-cli chat -s "You are a helpful coding assistant" "Write a Python hello world"
```

### Different models
```bash
deepseek-cli -m deepseek-coder chat "Explain this code: fn main() {}"
deepseek-cli -m deepseek-reasoner chat "Solve: 2x + 5 = 15"
```

### No streaming (wait for complete response)
```bash
deepseek-cli --no-stream chat "Tell me a joke"
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
# Quick Start 
- Install [Rust](https://rustup.rs/)
- Run `cargo run --release`

# Env Setup

You need to specify API keys for OpenAI-compatible LLM endpoint you want to test

We're using Chat Completion API

Format is key=value with the key being the base URL minus the special characters

For example,

For OpenAI, the URL for Chat Completions is `https://api.openai.com/v1/chat/completions`

The env key for OpenAI is
```
apiopenaicomv1=<YOUR_OPENAI_API_KEY_HERE>
```

Second example, for DeepSeek, URL is `https://api.deepseek.com/chat/completions`

Env key is

```
apideepseekcom=<YOUR_DEEPSEEK_API_KEY_HERE>
```

# How to Use

Please check the Postman Collection or Swagger
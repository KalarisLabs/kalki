# Compatibility

**Applies to:** v0.0.1 · **Updated:** 2026-11-11

What Kalki runs on, and which models it can talk to. Three words are used
precisely on this page:

- **Tested**: exercised by automated tests on that platform.
- **Built**: a release binary is produced for it.
- **Expected**: should work because it speaks a wire Kalki implements, and
  nobody has confirmed it. Tell us when you do.

## Operating systems

| Platform | Release binary | CI tests | Notes |
|---|---|---|---|
| Linux x86_64 | `x86_64-unknown-linux-musl`, static | Yes | Any distribution. No glibc requirement. |
| Linux aarch64 | `aarch64-unknown-linux-musl`, static | Yes | Includes Raspberry Pi OS 64-bit and ARM servers. |
| macOS Apple Silicon | `aarch64-apple-darwin` | Yes | macOS 15 runner. |
| macOS Intel | `x86_64-apple-darwin` | Yes | macOS 15 runner. |
| Windows x86_64 | `x86_64-pc-windows-msvc` | Yes | Windows 10 and 11. The development machine. |
| Windows on ARM | No | No | Runs the x86_64 binary under emulation. Expected. |
| WSL2 | Use the Linux binary | Covered by Linux | Recommended on Windows once the sandbox lands. |
| FreeBSD, others | No | No | Build from source. Unsupported. |

The CI matrix is defined in `.github/workflows/ci.yml`. Before the first push
to GitHub, only the Windows x86_64 row had been run, on a developer machine.

Building from source needs Rust 1.88 or newer and a C compiler (the TLS
library compiles C code). `git` must be on `PATH` at run time.

## Sandbox by platform

v0.0.1 has no kernel-enforced sandbox. The `bash` tool is limited by the
deny-first policy and a workspace path check. That stops mistakes. It does not
stop a hostile command. Run Kalki in a container or a virtual machine if the
workspace or the model is untrusted.

Planned confinement, v0.0.3:

| Platform | Mechanism | Strength |
|---|---|---|
| Linux | Landlock for files, seccomp for system calls, an empty network namespace, resource limits | Strong |
| macOS | Seatbelt profile | Medium. The API is deprecated by Apple and still used by other harnesses. |
| Windows | Job Object and AppContainer | Weaker. No per-path write rules as fine as Landlock. |
| Any | Run the executor in a container or WSL2 | Strong, at the cost of setup |

## Terminals

The headless CLI works in any terminal and in pipes. It respects `NO_COLOR`.

The terminal UI is planned for v0.0.3. Its targets are Windows Terminal,
iTerm2, Terminal.app, GNOME Terminal, Konsole, Alacritty, kitty, WezTerm,
Ghostty, the VS Code terminal, and `tmux` and SSH sessions inside any of
them. Details are in [TUI.md](TUI.md).

## Models and providers

Kalki speaks three wire protocols. Any server that speaks one of them works
without a Kalki release. Select the wire with `KALKI_PROVIDER` and the model
with `--model` or `KALKI_MODEL`.

| `KALKI_PROVIDER` | Wire | Configure with |
|---|---|---|
| `openai_compat` | OpenAI-compatible Chat Completions, `POST {base}/chat/completions`, streamed | `OPENAI_COMPAT_BASE_URL`, `OPENAI_COMPAT_API_KEY` |
| `openai_responses` (default) | OpenAI Responses | `OPENAI_API_KEY`, optional `OPENAI_BASE_URL` |
| `anthropic_messages` | Anthropic Messages | `ANTHROPIC_API_KEY`, optional `ANTHROPIC_BASE_URL` |

### Open-weight models

Open-weight models are reached through the `openai_compat` wire, which every
common server and host implements.

**Run it yourself**

| Server | Base URL to set | Status |
|---|---|---|
| Ollama | `http://localhost:11434/v1` | Expected |
| llama.cpp `llama-server` | `http://localhost:8080/v1` | Expected |
| LM Studio | `http://localhost:1234/v1` (the default) | Expected |
| vLLM | `http://localhost:8000/v1` | Expected |
| SGLang, TGI, LocalAI, Jan, KoboldCpp | their `/v1` endpoint | Expected |

Local servers usually need no key. Kalki still requires
`OPENAI_COMPAT_API_KEY` to be non-empty, so set it to any value.

**Hosted inference**

OpenRouter, Together AI, Fireworks AI, Groq, Cerebras, DeepInfra, Hugging Face
Inference Providers, NVIDIA NIM, SambaNova, Nebius, Novita, Hyperbolic,
Baseten, and the first-party APIs of DeepSeek, Moonshot (Kimi), Zhipu (GLM),
Alibaba (Qwen), Mistral, and xAI all expose an OpenAI-compatible endpoint.
Status for all of them: **Expected**. Set the base URL and key from the
provider's documentation.

**Model families**

Any model the server offers can be named: Qwen, GLM, Kimi, DeepSeek, Llama,
Mistral and Devstral, Gemma, Phi, gpt-oss, and others. Two limits apply today:

1. **Tool calling.** v0.0.1 uses the server's native function-calling field.
   A model or server without reliable native tool calls will not complete
   tool-using tasks. A text tool-call mode that needs no server support is
   planned for v0.0.2.
2. **Context budget.** v0.0.1 sends full history. Small-context models will
   overflow on long sessions. Per-model budgets arrive with the context
   compiler in v0.0.4.

### Proprietary models

| Provider | Wire | Status |
|---|---|---|
| OpenAI | `openai_responses` | Tested against recorded streams |
| Anthropic | `anthropic_messages` | Tested against recorded streams |
| Google Gemini | `openai_compat` through Google's compatibility endpoint | Expected. A native wire is not planned before v0.1. |

"Tested against recorded streams" means the decoder is verified against stream
fixtures and a loopback mock server. Nobody has yet run the full test matrix
against the live APIs. A conformance run against live endpoints is a v0.0.2
task.

## Agent conventions

| Convention | Support |
|---|---|
| `AGENTS.md` | This repository uses it. Reading a workspace `AGENTS.md` into context is planned for v0.0.2. |
| Agent Skills (`SKILL.md`) | This repository ships skills in `.agents/skills/`. Loading skills at run time is planned with the genome, v0.0.5. |
| Pi as an executor | Planned, v0.0.3. See [ADR-0004](adr/0004-executors-and-pi.md). |
| Model Context Protocol | Not planned before v0.1. Tools are `bash` and `apply_patch`. |

## Reporting a result

If you ran Kalki against a server or model listed as Expected, open an issue
with the `kalki doctor` output, the base URL, the model name, and whether a
tool-using task completed. That moves a row from Expected to Tested.

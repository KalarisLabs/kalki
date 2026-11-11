# Terminal UI specification

**Status:** design. Nothing on this page is built. Target release: v0.0.3.

The terminal is Kalki's main surface. This page says how it should behave so
that the implementation can be checked against it.

## Principles

1. **The terminal stays a terminal.** By default Kalki draws inline and leaves
   output in the scrollback. Selecting, copying, and searching use the
   terminal's own features. A full-screen mode is opt-in.
2. **The UI holds no logic.** The `kalki-tui` crate depends on
   `kalki-protocol` only. It renders events and sends commands. If the UI
   crashes, the session continues and can be re-attached.
3. **Nothing is hidden.** The status line always shows the model, the tokens
   in context, the session cost, and whether the last action was allowed,
   asked, or denied.
4. **It is fast or it is wrong.** Budgets below are release criteria.
5. **It works without colour.** State is never shown by colour alone.

## Layout

```
 ⏵ fix the flaky retry test                                   ← your message
 ● Reading crates/net/src/retry.rs
 ● bash  cargo test -p net retry        allowed by rule test-commands
   └ 14 passed, 1 failed  (2.1 s)  output: blob 3f9a… 18 KiB   ← handle, not a dump
 ● apply_patch  crates/net/src/retry.rs  +6 −2
 ✓ 15 passed
 The retry used a fixed delay. It now backs off with jitter.

 ┌──────────────────────────────────────────────────────────────────────────┐
 │ >                                                                        │  ← input
 └──────────────────────────────────────────────────────────────────────────┘
  qwen3-coder · ctx 11.2k/32k · $0.04 · turn 7 · checkpoint 7 · policy: default
```

- **Transcript.** One line per action. Tool output is collapsed to a result
  line and a blob handle. Expand it with a key.
- **Input.** Multi-line editor. `Enter` sends. `Shift+Enter` adds a line.
- **Status line.** One row. Described above.

## Commands

| Command | Does |
|---|---|
| `/help` | Lists commands and keys. |
| `/model` | Shows or switches the model. |
| `/rewind` | Lists checkpoints and restores the workspace and context to one. |
| `/tree` | Shows the subtask tree. |
| `/policy` | Shows the active rules and the last decisions. |
| `/cost` | Shows tokens and cost by turn. |
| `/genome` | Shows the active genome and its history. Read-only. |
| `/gate` | Shows pending and past admission decisions with their evidence. |
| `/audit` | Verifies the log chain for this session. |
| `/quit` | Detaches. The session keeps its state. |

`@` opens a file picker. `!` runs a shell command through the policy engine.

## Approval prompts

A rule with action `ask` pauses the session and shows:

```
 bash wants to run:  cargo publish --dry-run
 rule: ask-cargo-publish      working directory: ./crates/kalki
 [y] allow once   [a] allow for this session   [n] deny   [e] edit command
```

The prompt shows the exact argv, the rule that triggered it, and the working
directory. The default key is deny. A grant is written to the log with its
scope. Kalki never batches several commands into one approval.

## Keys

| Key | Action |
|---|---|
| `Enter` / `Shift+Enter` | Send / new line |
| `Esc` | Interrupt the model. Twice: clear the input. |
| `Ctrl+C` | Interrupt. Twice within a second: quit. |
| `Ctrl+R` | Search input history |
| `Ctrl+O` | Expand or collapse the tool output under the cursor |
| `Ctrl+L` | Redraw |
| `Tab` | Complete a path or command |

Keys are remappable in the config file. Every action is also a command, so the
UI is usable with no modifier keys.

## Performance budgets

| Metric | Budget | How it is measured |
|---|---|---|
| First paint after launch | 50 ms | Timestamp in a pseudo-terminal test |
| Key press to echo | 16 ms | Same |
| Frame time while streaming | 16 ms at the 95th percentile | Replay of a recorded 200 tokens/s stream |
| Resident memory, idle session | 60 MB | Process RSS after a 1,000-turn replay |

These numbers are targets set before any measurement. They will be revised
once measured, and the measurement method will be published with them.

## Accessibility and environments

- `NO_COLOR` and `--plain` remove colour and box drawing. Plain mode prints
  one line per event with a text prefix, which suits screen readers and logs.
- Symbols have ASCII fallbacks, chosen when the locale is not UTF-8.
- The UI adapts to widths from 60 columns. Below that it switches to plain mode.
- Works inside `tmux`, `screen`, and SSH. Does not require mouse support,
  true colour, or a particular font.
- On Windows it uses the modern console API through `crossterm`. The legacy
  console host is not supported.

## Testing

- **Frame snapshots.** A recorded event stream is rendered into a virtual
  terminal and the frame is compared with a stored snapshot.
- **Protocol replay.** The UI is driven by log replay with no model and no
  network, so every UI test is deterministic.
- **Resize and narrow-width cases** are part of the snapshot set.

## Libraries

`ratatui` with the `crossterm` backend, as decided in
[DEPENDENCIES.md](../DEPENDENCIES.md). They are added to the workspace when
the first UI code lands.

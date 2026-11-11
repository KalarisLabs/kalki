# CLAUDE.md

The rules for this repository are in AGENTS.md. They are imported here so that
there is one source.

@AGENTS.md

## Claude Code notes

- Skills are in `.agents/skills/`. Read a skill's `SKILL.md` when its row in
  AGENTS.md section 10 matches the task.
- Before pushing, run `gh api user --jq .login`. It must print
  `saynchowdhury` (AGENTS.md rule 44).
- For design-document review, use the `egm` reviewer agents when the plugin is
  installed.

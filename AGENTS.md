<!-- loom:v2 -->
## LOOM

This project uses LOOM as Agent-only continuity infrastructure. Run `loom context` when entering the
project, after a context reset, before substantial work, and when the human gives feedback or a direction
change. When a Task is active, the command restores its execution protocol, exact state, and declared
context; rerun it before editing after any interruption. Keep project truth, design documents, professional
capability dossiers, and Tasks current through the CLI, and never ask the human to operate LOOM. Do not
rerun LOOM before every tool call: persist and restore at meaningful work boundaries. The installed
`loom` command is the stable project-local invocation. If it is not on PATH in a source checkout, use
the absolute path to that checkout's `cli/bin/loom.js`; do not guess a relative path from this project.

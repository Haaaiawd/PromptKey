# Project Whole and Document Map

> This is the concise entry point, not the container for every design decision. Describe the whole and
> link the documents that make it buildable. Add or remove documents according to project complexity.
> The Agent uses `loom context` to compile this with the active Task and referenced files.

## Intended result

What should exist or become possible when this project succeeds?

## People and operating reality

Who or what experiences the result, and in what real situation?

## Whole experience or behavior

Describe the coherent end-to-end result rather than a feature inventory.

## Boundaries and consequential assumptions

What must not be changed, lost, invented, or expanded without authority?

## Design document map

Link every product, experience, system, contract, verification, or operations document and state the
decision surface it owns. Complex subsystems should have their own files under `.loom/design/`.

## Professional capability map

Link each separate field dossier under `.loom/capabilities/<field>/capability.md` and state which
design decisions it changes. Do not merge distinct fields into one dossier. Capabilities are shaped in
four steps: `loom capability research` → `synthesize` → `confirm --source human` (user-confirmed)
or `confirm --source agent` (explicitly provisional when the human is unavailable).

## Project structure

Point to `.loom/STRUCTURE.md` — where source code, tests, docs, assets, and configuration files live.
The Agent reads this before creating or moving files.

## Work map

Point to `.loom/tasks.json`; do not duplicate volatile Task state here. Each Task uses
`acceptance[]` with `criterion`, `verify_by`, and `evidence` fields. Completion requires one
`acceptance_results` entry per criterion with concrete evidence. Use `done_when[]` only for legacy
Tasks.

## Decision history

Consequential changes to existing decisions go in `.loom/DECISIONS.md`. Use `loom decision --json-file`
to record what changed, why, and which tasks were affected. `loom check` warns when a done Task is
marked affected by a later decision.

## Completion and failure

What observable evidence means the project worked? What could look complete while actually failing?

## Staged visibility and review

The human funds this project with attention and patience. Long stretches without visible progress
erode that patience, even when the work is sound. Design the Work Map so the human sees the project
growing, not just LOOM state changing.

- **Human-visible acceptance**: when designing Tasks, prefer acceptance criteria whose evidence is
  something the human can see or feel — a command running, a page rendering, a file with real content,
  a test passing in front of them. Machine-only verification is valid but should not be the only thing
  the human sees for long stretches.
- **Staged showcase**: every few Tasks, or at each natural project milestone, show the human something
  real that now works. Run the CLI, open the page, display the data, walk through the flow. A working
  thing creates momentum; a status update does not.
- **Staged review**: at material checkpoints, review what was built — run tests, inspect code quality,
  check against design intent. Catch drift early while it is cheap to fix. Tell the human what passed
  and what surprised you.
- **Verification gate**: after a batch of Tasks, run `loom check` and the project's own tests together.
  Both should pass before telling the human the batch is done. If tests fail or coverage drops, fix
  before moving on — do not let partial work accumulate behind a green-looking summary.
- **Excitement is a feature**: if the project has a surface the human will enjoy seeing — a UI, a CLI
  with clean output, a visualization, a working demo — prioritize reaching that surface early. The
  human's "I want to see more of this" feeling is real project fuel. Do not save the satisfying part
  for last if an early slice can deliver it.

## Keeper handoff

Before material execution, run `loom project ready` to freeze a digest, then ask a fresh Agent to
run `loom keeper prompt` and `loom keeper record`. Repair findings and prepare again;
a fresh Keeper must verify closure, including minor gaps. See `loom review --help`.

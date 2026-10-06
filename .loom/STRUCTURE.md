# Project structure

> Where things live in this project. The Agent reads this before creating or moving files.
> Update this when the structure changes. Delete sections that do not apply. Add sections
> that do. This is a map, not a prescription — each project declares its own conventions.

## Source code

Where implementation files go. Example: `src/` for application logic, `src/core/` for
domain logic, `src/cli/` for command-line interface.

## Tests

Where test files go and how they mirror source structure. Example: `tests/` mirroring
`src/` layout, or `__tests__/` co-located with source.

## Documents

Where project documentation goes (excluding `.loom/` which is LOOM state). Example:
`docs/` for user-facing docs, `README.md` at root for entry.

## Configuration and build

Where build configs, CI definitions, and dependency manifests go. Example:
`package.json`, `.github/workflows/`, `tsconfig.json`.

## Assets and fixtures

Where static assets, test fixtures, and data files go. Example: `assets/`, `fixtures/`,
`data/`.

## Conventions

Any naming or placement conventions the Agent should follow. Example: "one module per
file", "test files end with `.test.`", "config files are JSON not YAML".

# AGENTS.md

Repository instructions for coding agents working in `ClawPlane`.

## Core Principles

- DRY: Do not duplicate logic, constants, or workflows. Reuse existing modules, functions, and config.
- KISS: Prefer the simplest solution that satisfies current requirements.

## Implementation Rules

- Make focused, minimal changes; avoid broad refactors unless required.
- Extend existing code paths before introducing new abstractions.
- Keep files and functions small and readable.
- Avoid premature optimization and speculative features.
- Keep dependencies minimal; prefer standard library when practical.
- Update documentation when behavior or commands change.

## Validation

- Run the smallest relevant checks after changes (build/test/lint as applicable).
- If checks are skipped, state that clearly in your final note.

<!-- Point GitHub Copilot at the same rules every other agent follows. -->
# Copilot instructions

Read [AGENTS.md](../AGENTS.md) first, then the two files it lists under
`.agents/instructions/`. They are the contract. This file only points at it
and repeats what Copilot needs on every task.

- Setup and checks: `just setup` once, then `just check` before every push.
  `just test` runs the language tests.
- Commits: Conventional Commits, exactly one accepted type per commit.
- Write the failing test first. Never weaken a gate to make it pass.
- No path that names a machine. English only.
- State assumptions before implementing. Touch only what the task requires.

Nothing here grants tools, permissions or authority.

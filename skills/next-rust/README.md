# The Next Rust skill

A portable skill that teaches an AI coding agent how to build applications with
[Next Rust](https://github.com/iplustsolution/next-rust) and how to work on the framework itself.

It is plain Markdown with YAML frontmatter, so any tool or model can use it — nothing here is specific to one
assistant.

```
skills/next-rust/
├── SKILL.md                            the skill: mental model, rules, recipes, verification
└── references/
    ├── app-api.md                      exact signatures for pages, loaders, routes, actions, metadata, tests
    ├── cli-and-config.md               CLI flags, every next-rust.toml key, environment variables
    ├── framework-internals.md          crate map, dependency rules, CI, release automation, traps
    └── diagnostics.md                  every NR0xxx build error and its fix
```

`SKILL.md` is the entry point and stays small enough to keep in context; it points to the reference files for
details, so an agent only loads what the task needs.

## Install it

### Claude Code, Claude Desktop, claude.ai

1. Get the files:

   ```sh
   git clone https://github.com/iplustsolution/next-rust
   ```

2. Copy the skill where Claude looks for skills — `~/.claude/skills/` for every project on the machine, or
   `.claude/skills/` inside one project:

   ```sh
   mkdir -p ~/.claude/skills
   cp -r next-rust/skills/next-rust ~/.claude/skills/
   ```

3. Start a new session. Nothing else to configure: the skill is offered automatically when a task involves
   Next Rust. To confirm it is there, ask "which skills do you have?" or run `/skills` in Claude Code.

Already working inside this repository? It is found automatically — no copying needed.

### Cursor, Windsurf, GitHub Copilot, Zed, Cline and similar

1. Copy the folder into your project:

   ```sh
   cp -r next-rust/skills/next-rust my-app/skills/next-rust
   ```

2. Add one line to the instructions file your tool reads — `AGENTS.md`, `.cursor/rules/next-rust.mdc` or
   `.github/copilot-instructions.md`:

   ```md
   For any work in this repository, read skills/next-rust/SKILL.md first and follow it.
   ```

This repository ships an `AGENTS.md` that already does it.

### Any other agent, or an API integration

Paste the contents of `SKILL.md` into the system prompt, or load it as a retrievable document alongside the
`references/` files. The frontmatter `description` is written to be used as the retrieval trigger.

### A plain chat, with no tooling

Attach `SKILL.md` to the conversation and ask your question. Attach the relevant reference file too when you
need exact signatures.

## What it covers

- The mental model: Next.js App Router semantics, compile-time routing, one static binary.
- The compile-time contracts an agent cannot guess: filenames, required export names, which exports must be
  synchronous, where `load` must live.
- Recipes for the common tasks: pages, layouts, dynamic routes, data loading, 404s and redirects, API routes,
  server actions with form validation, client islands, styling, static/dynamic/ISR.
- How to verify work (`cargo build`, `next-rust routes --layouts`, `next-rust build`, `TestClient` tests).
- How to debug a failed build from its `NR0xxx` code.
- How to work on the framework: crate boundaries, the duplicated browser runtime, documentation written in
  Rust, the checks CI runs, and the release automation that owns the version number.

## Keeping it accurate

The skill states a framework version at the top of each reference file. When the framework changes in a way
that affects it — a new special file, a new config key, a changed signature, a new diagnostic — update the
skill in the same change, the way the documentation site and the README are updated. `evals/evals.json` holds
the task prompts used to check that the skill actually helps; rerun them after substantial edits.

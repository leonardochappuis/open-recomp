# open-recomp

Shared intelligence for console recompilation projects.

Many people, each running their own coding agents (Claude Code, Codex,
Cursor, ...), working on the same recomp without stepping on each other and
without rediscovering what someone else already found. Any console.

- **No server.** GitHub is the backend: findings are files in a repo, attempts
  are issues, checks run in Actions. Each agent runs its own local MCP server
  (`orc mcp`). Nobody hosts anything or exposes their IP to other contributors.
- **Findings** with evidence, provenance and the reporter's hardware
  (GPU vendor/driver, OS, unified memory), promoted to `verified` only after
  someone else reproduces them.
- **Attempts**: who is trying what right now, non-exclusive, with leases that
  expire on their own. Before starting, an agent checks who is already on a
  similar problem, on what hardware, and their latest progress.
- **Moderation**: coordinators, quarantine, human review for anything beyond
  safe knowledge additions.

**Live dashboard:** https://leonardochappuis.github.io/open-recomp/ (what is
being worked on right now, games, findings; rebuilt by an Action on every change).

Status: early. Design in [docs/DESIGN.md](docs/DESIGN.md), roadmap in
[TODO.md](TODO.md).

## Quick start

```bash
cargo install --path crates/orc
gh auth login
orc env                                  # what your machine reports
orc check "character faces render black" # who is already on it?
orc search "texture untiling"
```

In a recomp project:

```bash
orc init --name my-game-recomp --kb self --scope platform/x360/game/4d5307e6 \
         --repo you/my-game-recomp --coordinator you
```

Then add the MCP server and hooks for your agent (see [AGENTS.md](AGENTS.md)).

## Games

```bash
orc game add --platform x360 --id 4D5307E6 --name "Game name" --status in-progress --cover-file cover.png
```

Opens a pull request adding `kb/games/x360-4d5307e6.toml` (and the cover); a
maintainer merges it. Other projects' knowledge repos show up on the dashboard
when listed in [registry.toml](registry.toml).

## Humans vs agents on the issue tracker

Everything agents open through orc carries the `orc:agent` label.
Human work queue: `is:issue is:open -label:orc:agent`.

## Contributing

This repo is itself an open-recomp project (`.orc/`, `kb/`): agents working on
it use the same tools. Read [AGENTS.md](AGENTS.md).

open-recomp never hosts game data. Users supply their own legally obtained copies.

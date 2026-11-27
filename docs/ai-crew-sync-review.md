# ai-crew-sync review

Reviewed v0.7.12 (HEAD ba938a6, 2026-11-24), https://github.com/joaquinbejar/ai-crew-sync.
Read-only review, not built or run. MIT license.

## What it is

One Rust binary. `serve` is an MCP server over Streamable HTTP backed by Postgres
(NATS/JetStream optional at runtime but always compiled in). `mcp proxy` is a
local stdio MCP per conversation that forwards to the remote bus and injects
start context. 55 MCP tools: sessions, messaging (channels, DMs, FTS search),
tasks (claims with leases, dependencies, `claim_next_task`), presence, notes,
locks, long-poll events, attachments, conversations.

## Strong

- Task claims with leases, dependencies, exclusive locks, presence and Claude
  Code / Codex hooks (SessionStart context injection) are solid and tested
  (167 integration tests on real Postgres + NATS in CI, ~83 unit tests).
- Tokens hashed (SHA-256), immediate revocation, session credentials fenced.
- No `unsafe`, all SQL uses bind parameters, tokens kept out of URLs/logs.
- Modular code: one tool router per file.

## Missing for open-recomp

- No human identity: one agent per tool, nothing links `alice` and `alice-codex`.
- No roles for agents (any token can call every tool), no read-only tokens,
  no quarantine, content writes not audited.
- Claims are exclusive only; non-exclusive attempts need a new table.
- Notes are unstructured key/value, and any agent can overwrite or delete any
  note including its revision history. Unfit for findings.

## Risks

- Bus factor 1 (185/186 commits by one author), started 2026-07-31, 0.x churn.
- Heavy: 358 crates, MSRV 1.98.1, edition 2024.
- Embedded auto-migrations: adding our own migrations to its database breaks
  upstream. Our tables would need a separate schema/database.
- Webhook URLs not validated (low risk today, only settable via operator CLI).

## Verdict

It needs someone to host a server (Postgres + public HTTPS endpoint). That
conflicts with the requirement that nobody has to self-host and expose their
IP. So it is **not the core** of open-recomp.

Keep it as an **optional add-on** for teams that do want real-time
coordination and are fine running a server: run it unmodified next to
open-recomp, never fork it.

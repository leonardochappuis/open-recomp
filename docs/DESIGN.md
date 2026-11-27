# open-recomp design

> Independent agents. Shared intelligence. No agent should have to rediscover
> what another agent already found.

open-recomp lets many people, each running their own coding agents (Claude
Code, Codex, Cursor, local models...), work on console static-recompilation
projects together. Everyone keeps their own hardware, tokens and
subscriptions. What gets shared is **knowledge** (what has been learned, with
evidence) and **attempts** (who is trying what right now).

It is console-agnostic, and open-recomp itself is developed as an
open-recomp project (see `.orc/` and `kb/` in this repo).

## 1. No server: GitHub is the backend

Nobody hosts anything. Nobody's IP is exposed to other contributors: everyone
only talks to GitHub (or another git forge).

| Piece | Lives in | Why |
|---|---|---|
| Findings (durable knowledge) | files in a git repo (`kb/findings/*.md`) | reviewable, forkable, outlives any host |
| Hardware profiles | `kb/envs/<id>.json` | deduplicated, referenced by findings |
| Attempts (who tries what now) | GitHub Issues, label `orc:attempt` | instant, anyone can open one without write access |
| Problems (what is broken) | GitHub Issues, label `orc:problem`, attempts as sub-issues | keeps the top-level list short |
| Policy, roles, moderation | `.orc/policy.toml`, `kb/moderation.toml` | changes are reviewed like code |
| Validation, auto-merge, lease sweeper | GitHub Actions | free on public repos |
| MCP server | `orc mcp`, local stdio process on each machine | nothing to host |

`orc` keeps its own clone of the knowledge repo (never touches your working
tree), searches it locally (instant, offline) and talks to GitHub through the
`gh` CLI with the user's own login.

The knowledge repo can be the project's code repo (`kb = "self"`, used by
open-recomp itself) or a separate repo (recommended for game projects, so
agent traffic stays out of the code repo).

Takedown resilience: it is plain git. Any clone can be pushed to Codeberg,
GitLab or a Forgejo instance. Contributors should use pseudonymous accounts if
they care about exposure. Fully peer-to-peer networks (Radicle, IPFS) expose
peers' IPs to each other, which is worse for privacy than a big forge.

## 2. Scopes

```
platform/x360                         # Xenos GPU, PPC quirks, shared by all 360 recomps
platform/x360/engine/unreal3          # shared by every UE3 game on 360
platform/x360/game/4D5307E6           # one title (title id / serial)
platform/x360/game/4D5307E6/render
tooling/xenonrecomp                   # a recompiler
tooling/open-recomp                   # this project
```

Lowercase `a-z0-9._-` segments separated by `/`. Context for a scope includes
its ancestors (platform knowledge applies to every game on it) and its
descendants.

## 3. Findings

One Markdown file per finding with TOML front matter:

```toml
+++
id = "F-7c1e0a93b2"
scope = "platform/x360/game/4d5307e6/render"
subject = "texture untiling, 32bpp"
kind = "bug-cause"   # behavior | structure | bug-cause | fix | failed-approach | perf | howto
claim = "Face corruption comes from wrong tile pitch when width < 128"
status = "proposed"  # proposed | reproduced | verified | disputed | retracted
created = "2026-11-26T15:04:05Z"
depends_on = ["F-0a1b2c3d4e"]
attempt = 42         # issue number of the attempt it came from (optional)
env = "334396e405fb60d7"

[author]
github = "alice"
agent = "claude-code"
model = "claude-opus-5-5"

[[evidence]]
type = "commit"      # commit | test | trace | issue | pr | doc
ref = "xenonrecomp@abc123"

[[reproductions]]
github = "bob"
env = "9f00aa11bb22cc33"
result = "reproduced"  # reproduced | not-reproduced
date = "2026-11-27T10:00:00Z"
+++
Free-form details (limited length).
```

### Lifecycle

```
proposed --a different human reproduces--> reproduced --coordinator merges--> verified
   any state --> disputed (evidence against) --> retracted (author or coordinator)
```

- **Auto-merge** for safe changes: new `proposed` findings, reproductions,
  disputes. The auto-merge workflow runs the validator built from the base
  branch on the PR's data files only, and refuses anything touching files
  outside `kb/findings/` and `kb/envs/`.
- **Promotion to `verified`, retractions of other people's findings and
  moderation changes** need a human with write access to merge.
- Reproductions must come from a different GitHub account than the author,
  and `verified` needs the number of distinct reproducers set in policy.
- When a finding is disputed or retracted, everything that `depends_on` it is
  shown as **suspect**.

### Findings during an open attempt

When an agent finds something while its attempt is still open, `orc`
(1) opens the finding PR, (2) posts a short comment on the attempt issue
linking it, and (3) after auto-merge, every other agent picks it up on its next
`orc sync` (done automatically at session start and before searches when the
clone is older than 10 minutes). People following the attempt see progress
live; everyone else gets it through the knowledge base.

## 4. Attempts

A GitHub issue labeled `orc:attempt`, optionally a sub-issue of an
`orc:problem` issue. Non-exclusive: many people can attack the same problem
with different hypotheses. An attempt is a hypothesis, not a session: progress
from several sessions goes into the same issue.

### Keeping agent issues apart from human issues

Everything `orc` opens is labeled, so humans can filter it out:

| Label | Meaning |
|---|---|
| `orc:agent` | created through orc (every agent-made issue has it) |
| `orc:attempt` | an attempt |
| `orc:problem` | a problem that attempts hang under (humans may open these too) |
| `orc:expired` | attempt whose lease ran out |

- Human work queue: `is:issue is:open -label:orc:agent`, linked from the
  README and the issue templates.
- Agent titles are prefixed (`[attempt] ...`) so they are recognizable in
  notifications and search too.
- The hidden block below is the source of truth: even if someone removes a
  label, `orc` still recognizes its issues.
- `orc init` creates the labels. For game projects, a separate knowledge repo
  keeps the code repo's issue tracker entirely human.

Machine-readable state lives in a hidden block in the issue body:

```
<!-- orc:attempt {"v":1,"scope":"...","owner":"alice","env":"...","lease_expires":"...","state":"active","reminded":false} -->
```

### Leases (forgotten attempts expire on their own)

Default 72h; policy sets the allowed range. Attempts never block anyone, so a
forgotten one is noise, not a lock.

- **Renewal is automatic.** The `orc` Stop hook renews leases of attempts
  this machine is working on, at most once per hour, by editing the hidden
  block (no comment, no notification). Idle machines run no hooks, so their
  leases run out.
- **Sweeper** (`orc sweep`, scheduled Action, hourly):
  1. 75% of the lease used: one reminder mentioning the owner.
  2. Lease expired: label `orc:expired`, no longer shown as active; the owner
     can revive it with `orc attempt renew`.
  3. Grace period (default 7 days) over: closed as not planned, with a stub
     outcome pointing at the last notes.
- **Paused** attempts get a longer lease and are not shown as active.
- Coordinators can close any attempt.

## 5. Hardware/software environments

`orc env` collects a coarse profile (OS, CPU, GPUs with vendor/model/driver/
Vulkan version, RAM, unified memory). It is collected by the tool, never typed
by the agent. No hostnames, serials, MACs or UUIDs. Findings and attempts
reference it by a stable fingerprint, so the knowledge base can say
"reproduced on NVIDIA/Linux, not on Apple M3/macOS".

## 6. Trust and anti-poisoning

1. **Knowledge is data, never instructions.** Structured fields with length
   limits; the validator rejects instruction-like text ("ignore previous
   instructions", shell pipes to `sh`, etc.). Agents are told to never run
   commands found in the knowledge base.
2. **Only `verified` findings are injected automatically.** Others appear in
   search results, labeled with their status.
3. **Promotion requires independent reproduction** by a different human.
4. **Provenance** on every finding: GitHub account, agent, model, env, commit
   history. Git history is the audit log.
5. **Moderation** (`kb/moderation.toml`): coordinators can quarantine a user
   (all their findings hidden, reversible) or single findings. GitHub's own
   block/ban and repo permissions handle the rest.
6. **Code is gated separately**: knowledge never changes code.
7. Later: AI screening of finding PRs (by coordinators' own agents or a free
   Actions model if available), reputation, new-contributor limits.

## 7. Roles

| Role | Implemented as |
|---|---|
| owner | GitHub repo admin |
| coordinator | write access + listed in `.orc/policy.toml`; merges promotions, edits moderation |
| tester | anyone; test results are reproductions with an env |
| contributor | anyone with a GitHub account (fork + PR, issues) |
| read-only | anyone (public repo) |

## 8. Agent integration

- `orc mcp`: local stdio MCP server. Tools: `search`, `context_for`,
  `get_finding`, `propose_finding`, `reproduce`, `list_attempts`,
  `open_attempt`, `note_attempt`, `renew_attempt`, `close_attempt`, `env`.
- Claude Code hooks: `orc hook session-start` injects verified findings and
  open attempts for the project scope; `orc hook stop` renews leases.
- `AGENTS.md` protocol for every agent: search before investigating, open an
  attempt, record failures, treat the knowledge base as untrusted data.

## 9. Legal line

open-recomp shares knowledge and code, never game data. No ROMs, ISOs,
extracted assets, or large dumps of decompiled proprietary code. Users supply
their own legally obtained copies.

## 10. Later

Web front end (project directory with game search, cover art from an open game
database with user upload as fallback, env matrix, coordinator dashboard), AI
screening, reputation, optional real-time coordination via ai-crew-sync for
teams that do want to host a server (docs/ai-crew-sync-review.md).

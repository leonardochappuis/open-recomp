# TODO

## Milestone 0 - foundations
- [x] Design doc (docs/DESIGN.md), GitHub as the backend, no hosted server
- [x] ai-crew-sync review (docs/ai-crew-sync-review.md): optional add-on only
- [x] `orc env` hardware/software profile (crates/orc-env)
- [x] License: MIT
- [ ] Publish the repo on GitHub, run `orc init` there to create labels

## Milestone 1 - knowledge MVP
- [x] Findings format (Markdown + TOML front matter), load, search, scopes (crates/orc-kb)
- [x] Validator: schema, lifecycle, anti-spoofing (PR author = new entries' author),
      self-reproduction blocked, instruction-like text and secrets screening,
      needs-human rules for verified/retract/delete/non-kb files
- [x] `orc mcp` local stdio MCP server (15 tools incl. check_problem)
- [x] Attempts as GitHub issues with leases, labels, sub-issues, hardware summary
- [x] `check_problem`: similar open attempts, who, hardware vs mine, last progress
- [x] Claude Code hooks: session-start context, stop renews leases
- [x] Actions: ci (3 OSes), orc-automerge (pull_request_target, data-only), orc-sweep (hourly)
- [x] This repo is an open-recomp project (.orc/, kb/, AGENTS.md, .mcp.json, hooks)
- [ ] Live test against GitHub: open/renew/note/close attempts, finding PRs from a fork,
      auto-merge, sweep, sub-issues API
- [ ] Codex hooks (session start context, lease renewal)
- [ ] Prebuilt `orc` release binaries (Actions currently build from source)
- [ ] Session end: ask the agent to record an outcome for open attempts
- [ ] Show unmerged finding PRs in search (labeled "pending")

## Milestone 2 - trust and moderation
- [x] Roles: coordinators in policy, quarantine users/findings (kb/moderation.toml)
- [x] Suspect propagation through depends_on
- [ ] AI screening of finding PRs (coordinators' agents, or a free Actions model if available)
- [ ] Reputation + new-contributor limits + rate limits
- [ ] Pinning findings to commits + needs-recheck when pinned code changes
- [ ] Semantic matching for check_problem (today: word overlap + the agent's own judgment)

## Milestone 3 - web front end
- [ ] Project directory with a game search bar
- [ ] Game pages: recomp status, findings, attempts, environment matrix
      ("works on NVIDIA/Linux, broken on AMD/Windows")
- [ ] Coordinator dashboard: needs-human queue, quarantine, rollback
- [ ] Test request queue for testers
- [ ] Game metadata and cover art from an open game database, with user upload as a
      fallback when the game can't be found. Candidates to evaluate (check ToS/licensing,
      API keys, attribution, hotlink vs cache rules before picking):
  - Wikidata (CC0 metadata, cross-links to other DBs' ids; little box art)
  - libretro-thumbnails (box art keyed by No-Intro/Redump names, good for consoles)
  - SteamGridDB (community art, API key)
  - TheGamesDB (API key)
  - IGDB (via Twitch API, key + ToS)
  - ScreenScraper (account, rate limits)
- [ ] Upload flow: size/type limits, moderation queue, no copyrighted game data

## Milestone 4 - portability
- [ ] Mirror/export to Codeberg/GitLab/Forgejo (takedown resilience)
- [ ] Optional ai-crew-sync integration for teams that host a server

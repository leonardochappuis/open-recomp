//! `orc mcp`: a local MCP server over stdio (newline-delimited JSON-RPC).
//! Nothing is hosted: each agent starts its own copy, which reads the local
//! clone of the knowledge base and talks to GitHub as the user.

use crate::attempts::{self, OpenArgs, Outcome};
use crate::context;
use crate::project::Ctx;
use crate::publish::{self, NewFinding};
use anyhow::{anyhow, bail, Context, Result};
use orc_kb::search::{search, Query};
use orc_kb::{Evidence, Kind, ReproResult, Status};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

pub fn serve() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let ctx = Ctx::discover(&cwd);
    let mut client_name: Option<String> = None;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                reply(&mut stdout, json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":e.to_string()}}))?;
                continue;
            }
        };
        let Some(id) = msg.get("id").cloned() else { continue }; // notification
        let method = msg["method"].as_str().unwrap_or_default();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let result = match method {
            "initialize" => {
                client_name = params["clientInfo"]["name"].as_str().map(String::from);
                let asked = params["protocolVersion"].as_str().unwrap_or_default();
                let version = if PROTOCOL_VERSIONS.contains(&asked) { asked } else { PROTOCOL_VERSIONS[0] };
                Ok(json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "open-recomp", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": format!("{}\n{}", context::PREAMBLE, context::PROTOCOL),
                }))
            }
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => {
                let name = params["name"].as_str().unwrap_or_default();
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let out = match &ctx {
                    Ok(ctx) => call(ctx, name, &args, client_name.as_deref()),
                    Err(e) => Err(anyhow!("{e}")),
                };
                Ok(match out {
                    Ok(text) => json!({ "content": [{ "type": "text", "text": text }] }),
                    Err(e) => {
                        json!({ "content": [{ "type": "text", "text": format!("error: {e:#}") }], "isError": true })
                    }
                })
            }
            _ => Err(json!({ "code": -32601, "message": format!("method not found: {method}") })),
        };
        let resp = match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r }),
            Err(e) => json!({ "jsonrpc": "2.0", "id": id, "error": e }),
        };
        reply(&mut stdout, resp)?;
    }
    Ok(())
}

fn reply(out: &mut impl Write, v: Value) -> Result<()> {
    writeln!(out, "{v}")?;
    out.flush()?;
    Ok(())
}

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": props, "required": required, "additionalProperties": false },
    })
}

fn evidence_schema() -> Value {
    json!({
        "type": "array",
        "description": "Checkable evidence: a commit (repo@sha), test name, trace link, issue/PR, doc.",
        "items": {
            "type": "object",
            "properties": {
                "type": { "type": "string", "enum": ["commit", "test", "trace", "issue", "pr", "doc", "other"] },
                "ref": { "type": "string" }
            },
            "required": ["type", "ref"]
        }
    })
}

fn tools() -> Vec<Value> {
    let scope = json!({ "type": "string", "description": "Scope path, e.g. platform/x360/game/4d5307e6/render. Defaults to the project's scope." });
    let number = json!({ "type": "integer", "description": "Attempt issue number" });
    vec![
        tool("search", "Search the shared knowledge base. Do this before investigating anything: someone may already know.",
            json!({
                "query": { "type": "string" },
                "scope": scope,
                "status": { "type": "array", "items": { "type": "string", "enum": ["proposed","reproduced","verified","disputed","retracted"] } },
                "limit": { "type": "integer" }
            }), &["query"]),
        tool("check_problem", "Call this FIRST whenever the user reports a problem or asks you to look into something. Finds similar open attempts (who is on it, on what hardware compared to this machine, latest progress) and related findings, and tells you what to ask the user before starting.",
            json!({ "description": { "type": "string", "description": "The problem in the user's words plus anything you already know" }, "scope": scope }), &["description"]),
        tool("context_for", "Verified findings, recent unverified ones and open attempts for a scope.",
            json!({ "scope": scope }), &[]),
        tool("get_finding", "Full text of one finding, with evidence, reproductions and disputes.",
            json!({ "id": { "type": "string" } }), &["id"]),
        tool("propose_finding", "Share something you learned, with evidence. Also record failed approaches (kind failed-approach). Opens a pull request; if attempt is given, the attempt issue gets a comment.",
            json!({
                "scope": scope,
                "subject": { "type": "string", "description": "What it is about: address, symbol, file or topic (3-120 chars)" },
                "kind": { "type": "string", "enum": ["behavior","structure","bug-cause","fix","failed-approach","perf","howto"] },
                "claim": { "type": "string", "description": "One or two sentences that can be checked (10-500 chars)" },
                "details": { "type": "string", "description": "Markdown details" },
                "evidence": evidence_schema(),
                "depends_on": { "type": "array", "items": { "type": "string" } },
                "attempt": number,
                "model": { "type": "string", "description": "Model id you are running as" }
            }), &["subject", "kind", "claim"]),
        tool("reproduce", "Record that you independently checked someone else's finding (on your hardware).",
            json!({
                "id": { "type": "string" },
                "result": { "type": "string", "enum": ["reproduced", "not-reproduced"] },
                "notes": { "type": "string" },
                "evidence": evidence_schema()
            }), &["id", "result"]),
        tool("dispute", "Mark a finding as disputed, with the reason and evidence.",
            json!({ "id": { "type": "string" }, "reason": { "type": "string" }, "evidence": evidence_schema() }), &["id", "reason"]),
        tool("retract", "Retract your own finding.",
            json!({ "id": { "type": "string" }, "reason": { "type": "string" } }), &["id", "reason"]),
        tool("list_attempts", "Open attempts (who is trying what right now).", json!({ "scope": scope }), &[]),
        tool("open_attempt", "Announce that you are trying a hypothesis. Non-exclusive: others may try other approaches on the same problem. The lease renews automatically while you work.",
            json!({
                "scope": scope,
                "problem": { "type": "string" },
                "hypothesis": { "type": "string" },
                "approach": { "type": "string" },
                "problem_issue": { "type": "integer", "description": "Issue number of the problem this belongs to" },
                "lease_hours": { "type": "integer" }
            }), &["problem", "hypothesis", "approach"]),
        tool("note_attempt", "Post progress on your attempt.", json!({ "number": number, "text": { "type": "string" } }), &["number", "text"]),
        tool("renew_attempt", "Extend your attempt's lease.", json!({ "number": number, "hours": { "type": "integer" } }), &["number"]),
        tool("pause_attempt", "Pause your attempt (longer lease, not shown as active).", json!({ "number": number }), &["number"]),
        tool("close_attempt", "Close your attempt with an outcome. Record a finding first, even when it failed.",
            json!({
                "number": number,
                "outcome": { "type": "string", "enum": ["succeeded", "failed", "abandoned"] },
                "summary": { "type": "string" },
                "findings": { "type": "array", "items": { "type": "string" } }
            }), &["number", "outcome", "summary"]),
        tool("env", "This machine's hardware/software profile (attached automatically to findings).", json!({}), &[]),
    ]
}

fn s<'a>(a: &'a Value, k: &str) -> Option<&'a str> {
    a.get(k).and_then(Value::as_str)
}

fn req<'a>(a: &'a Value, k: &str) -> Result<&'a str> {
    s(a, k).with_context(|| format!("missing {k}"))
}

fn num(a: &Value, k: &str) -> Result<u64> {
    a.get(k).and_then(Value::as_u64).with_context(|| format!("missing {k}"))
}

fn strings(a: &Value, k: &str) -> Vec<String> {
    a.get(k).and_then(Value::as_array).into_iter().flatten().filter_map(|v| v.as_str().map(String::from)).collect()
}

fn evidence(a: &Value) -> Result<Vec<Evidence>> {
    match a.get("evidence") {
        None | Some(Value::Null) => Ok(vec![]),
        Some(v) => Ok(serde_json::from_value(v.clone()).context("evidence must be [{type, ref}]")?),
    }
}

fn scope_of(ctx: &Ctx, a: &Value) -> String {
    s(a, "scope").map(String::from).unwrap_or_else(|| ctx.project.default_scope.clone())
}

fn pretty(v: impl serde::Serialize) -> String {
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

pub fn call(ctx: &Ctx, name: &str, a: &Value, client: Option<&str>) -> Result<String> {
    let policy = || ctx.kb().map(|kb| kb.policy);
    Ok(match name {
        "search" => {
            let kb = ctx.kb()?;
            let statuses: Vec<Status> = strings(a, "status")
                .iter()
                .map(|x| serde_json::from_value(json!(x)).map_err(|_| anyhow!("unknown status {x}")))
                .collect::<Result<_>>()?;
            let q = Query {
                text: req(a, "query")?.to_string(),
                scope: s(a, "scope").map(String::from),
                statuses,
                limit: a.get("limit").and_then(Value::as_u64).unwrap_or(20) as usize,
            };
            format!("{}\n\n{}", context::PREAMBLE, context::hits_text(&search(&kb, &q)))
        }
        "check_problem" => crate::check::check(ctx, req(a, "description")?, s(a, "scope"))?,
        "context_for" => {
            let kb = ctx.kb()?;
            let attempts = ctx.repo().ok().and_then(|r| attempts::list(r).ok());
            let me = publish::login(ctx).ok();
            context::for_scope(&kb, &scope_of(ctx, a), attempts.as_deref(), me.as_deref())
        }
        "get_finding" => {
            let kb = ctx.kb()?;
            let id = req(a, "id")?;
            let f = kb.get(id).with_context(|| format!("no finding {id}"))?;
            let suspect = kb.suspects().contains(id);
            format!(
                "{}\n{}\n\n{}",
                context::PREAMBLE,
                if suspect { "SUSPECT: depends on a disputed or retracted finding." } else { "" },
                f.render()
            )
        }
        "propose_finding" => {
            let kind: Kind = req(a, "kind")?.parse().map_err(|e: String| anyhow!(e))?;
            let out = publish::propose(
                ctx,
                NewFinding {
                    scope: scope_of(ctx, a),
                    subject: req(a, "subject")?.to_string(),
                    kind,
                    claim: req(a, "claim")?.to_string(),
                    body: s(a, "details").unwrap_or_default().to_string(),
                    evidence: evidence(a)?,
                    depends_on: strings(a, "depends_on"),
                    attempt: a.get("attempt").and_then(Value::as_u64),
                    agent: client.map(String::from),
                    model: s(a, "model").map(String::from),
                },
            )?;
            pretty(out)
        }
        "reproduce" => {
            let result: ReproResult = serde_json::from_value(json!(req(a, "result")?)).context("bad result")?;
            pretty(publish::reproduce(ctx, req(a, "id")?, result, s(a, "notes").map(String::from), evidence(a)?)?)
        }
        "dispute" => pretty(publish::dispute(ctx, req(a, "id")?, req(a, "reason")?.to_string(), evidence(a)?)?),
        "retract" => pretty(publish::retract(ctx, req(a, "id")?, req(a, "reason")?.to_string())?),
        "list_attempts" => {
            let all = attempts::list(ctx.repo()?)?;
            let sc = s(a, "scope");
            let shown: Vec<&attempts::Attempt> =
                all.iter().filter(|x| sc.is_none_or(|sc| orc_kb::scope::related(&x.meta.scope, sc))).collect();
            if shown.is_empty() {
                "No open attempts.".into()
            } else {
                context::attempts_text(&shown)
            }
        }
        "open_attempt" => {
            let env = orc_env::Environment::collect();
            let scope = scope_of(ctx, a);
            let at = attempts::open(
                ctx.repo()?,
                &policy()?,
                OpenArgs {
                    scope: &scope,
                    problem: req(a, "problem")?,
                    hypothesis: req(a, "hypothesis")?,
                    approach: req(a, "approach")?,
                    problem_issue: a.get("problem_issue").and_then(Value::as_u64),
                    lease_hours: a.get("lease_hours").and_then(Value::as_u64).map(|h| h.min(u32::MAX as u64) as u32),
                    env: Some(&env),
                },
            )?;
            format!("Opened attempt #{} ({}), lease until {}.", at.number, at.url, at.meta.lease_expires)
        }
        "note_attempt" => attempts::note(ctx.repo()?, num(a, "number")?, req(a, "text")?)?,
        "renew_attempt" => {
            let hours = a.get("hours").and_then(Value::as_u64).map(|h| h.min(u32::MAX as u64) as u32);
            let m = attempts::renew(ctx.repo()?, num(a, "number")?, &policy()?, hours)?;
            format!("Lease extended until {}.", m.lease_expires)
        }
        "pause_attempt" => {
            let m = attempts::pause(ctx.repo()?, num(a, "number")?, &policy()?)?;
            format!("Paused until {}.", m.lease_expires)
        }
        "close_attempt" => {
            let outcome: Outcome = serde_json::from_value(json!(req(a, "outcome")?)).context("bad outcome")?;
            attempts::close(
                ctx.repo()?,
                num(a, "number")?,
                &policy()?,
                outcome,
                req(a, "summary")?,
                &strings(a, "findings"),
            )?;
            "Closed.".into()
        }
        "env" => {
            let e = orc_env::Environment::collect();
            pretty(json!({ "id": e.fingerprint(), "summary": e.summary(), "env": e }))
        }
        other => bail!("unknown tool {other}"),
    })
}

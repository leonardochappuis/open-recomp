//! Claude Code hooks. They never fail the session: errors go to stderr and
//! the hook exits 0.

use crate::attempts;
use crate::context;
use crate::project::Ctx;
use crate::publish;
use crate::state::LocalState;
use chrono::Utc;
use serde_json::{json, Value};
use std::io::Read;
use std::path::PathBuf;

fn hook_ctx() -> Option<Ctx> {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let cwd = serde_json::from_str::<Value>(&input)
        .ok()
        .and_then(|v| v["cwd"].as_str().map(PathBuf::from))
        .or_else(|| std::env::current_dir().ok())?;
    Ctx::discover(&cwd).ok()
}

/// SessionStart: inject verified knowledge and open attempts.
pub fn session_start() {
    let Some(ctx) = hook_ctx() else { return };
    if let Err(e) = ctx.sync(false) {
        eprintln!("orc: sync failed, using cached knowledge: {e}");
    }
    let kb = match ctx.kb() {
        Ok(kb) => kb,
        Err(e) => return eprintln!("orc: {e}"),
    };
    let attempts = ctx.repo().ok().and_then(|r| attempts::list(r).ok());
    let me = publish::login(&ctx).ok();
    let text = context::for_scope(&kb, &ctx.project.default_scope, attempts.as_deref(), me.as_deref());
    let out = json!({
        "hookSpecificOutput": { "hookEventName": "SessionStart", "additionalContext": text }
    });
    println!("{out}");
}

/// Stop: renew leases of attempts this machine is working on (hourly).
pub fn stop() {
    let Some(ctx) = hook_ctx() else { return };
    let Ok(repo) = ctx.repo() else { return };
    let due = LocalState::load().due(repo, Utc::now());
    if due.is_empty() {
        return;
    }
    let policy = ctx.kb().map(|kb| kb.policy).unwrap_or_default();
    for n in due {
        if let Err(e) = attempts::renew(repo, n, &policy, None) {
            eprintln!("orc: not renewing #{n}: {e}");
            let _ = LocalState::update(|s| s.untrack(repo, n));
        }
    }
}

//! "Is anyone already on this?" Before an agent starts on a problem, it calls
//! `check_problem` with the user's description. orc finds similar open
//! attempts (cheap word matching to cast the net; the agent itself judges
//! which candidates really are the same problem), says who is on them, on
//! what hardware compared to this machine, and what their last progress was.

use crate::attempts::{self, Attempt, Hw, HwMatch, State};
use crate::context::{hits_text, PREAMBLE};
use crate::gh;
use crate::project::Ctx;
use anyhow::Result;
use orc_kb::scope;
use orc_kb::search::{search, tokens, Query, STOPWORDS};
use std::fmt::Write;

/// Content words, lowercased, with a light plural/gerund stem.
fn words(s: &str) -> Vec<String> {
    let mut v: Vec<String> =
        tokens(s).into_iter().filter(|w| !STOPWORDS.contains(&w.as_str())).map(|w| stem(&w)).collect();
    v.sort();
    v.dedup();
    v
}

fn stem(w: &str) -> String {
    for suf in ["ing", "ies", "es", "s", "ed"] {
        if w.len() > suf.len() + 3 {
            if let Some(base) = w.strip_suffix(suf) {
                return if suf == "ies" { format!("{base}y") } else { base.to_string() };
            }
        }
    }
    w.to_string()
}

/// Share of the query's content words that appear in the document.
pub fn similarity(query: &str, doc: &str) -> f64 {
    let q = words(query);
    if q.is_empty() {
        return 0.0;
    }
    let d = words(doc);
    let hit = q.iter().filter(|w| d.contains(w)).count();
    hit as f64 / q.len() as f64
}

/// The human-written part of an attempt body (no hidden block, no footer).
fn problem_text(body: &str) -> String {
    body.lines().skip(1).take_while(|l| l.trim() != "---").collect::<Vec<_>>().join("\n")
}

fn field(body: &str, name: &str) -> Option<String> {
    let prefix = format!("**{name}:** ");
    body.lines().find_map(|l| l.strip_prefix(&prefix)).map(|s| s.trim().to_string())
}

fn short(s: &str, max: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
        s
    } else {
        format!("{}...", s.chars().take(max).collect::<String>())
    }
}

pub struct Candidate<'a> {
    pub attempt: &'a Attempt,
    pub score: f64,
}

/// Rank open attempts by similarity to a problem description.
pub fn rank<'a>(attempts: &'a [Attempt], text: &str, scope_path: Option<&str>) -> Vec<Candidate<'a>> {
    let mut c: Vec<Candidate> = attempts
        .iter()
        .filter(|a| a.meta.state != State::Expired)
        .map(|a| {
            let mut score = similarity(text, &format!("{}\n{}", a.title, problem_text(&a.body)));
            if scope_path.is_some_and(|s| scope::related(&a.meta.scope, s)) {
                score += 0.15;
            }
            Candidate { attempt: a, score }
        })
        .filter(|c| c.score >= 0.3)
        .collect();
    c.sort_by(|a, b| b.score.total_cmp(&a.score));
    c.truncate(6);
    c
}

fn last_progress(repo: &str, a: &Attempt) -> Option<String> {
    if a.comments == 0 {
        return None;
    }
    let v = gh::api("GET", &format!("repos/{repo}/issues/{}/comments?per_page=1&page={}", a.number, a.comments), None)
        .ok()?;
    let c = v.as_array()?.first()?;
    let when = c["created_at"].as_str().unwrap_or_default().get(..10).unwrap_or_default().to_string();
    Some(format!("{when}: {}", short(c["body"].as_str()?, 280)))
}

fn hw_line(theirs: Option<&Hw>, mine: &Hw) -> (HwMatch, String) {
    let Some(t) = theirs else {
        return (HwMatch::Same, "hardware unknown".into());
    };
    let m = t.compare(mine);
    let vendors = if t.gpu_vendors.is_empty() { "unknown GPU".into() } else { t.gpu_vendors.join("+") };
    let note = match m {
        HwMatch::Same => format!("same GPU vendor and OS as you ({vendors}, {})", t.os),
        HwMatch::DifferentOs => format!("same GPU vendor ({vendors}) but on {} (you: {})", t.os, mine.os),
        HwMatch::DifferentGpu => format!(
            "DIFFERENT GPU vendor: they are on {vendors}, you are on {}; the cause and fix may differ on your hardware",
            if mine.gpu_vendors.is_empty() { "unknown".into() } else { mine.gpu_vendors.join("+") }
        ),
    };
    (m, format!("{note} [{}]", t.summary))
}

pub fn check(ctx: &Ctx, text: &str, scope_path: Option<&str>) -> Result<String> {
    let me_env = orc_env::Environment::collect();
    let mine = Hw::from_env(&me_env);
    let mut out = format!("{PREAMBLE}\n\nYour machine: {}\n\n", mine.summary);

    let attempts = match ctx.repo() {
        Ok(repo) => attempts::list(repo).map(|a| (repo.to_string(), a)).ok(),
        Err(_) => None,
    };
    let mut same = Vec::new();
    let mut different = Vec::new();
    if let Some((repo, all)) = &attempts {
        let ranked = rank(all, text, scope_path);
        if ranked.is_empty() {
            out.push_str("Similar open attempts: none found.\n\n");
        } else {
            out.push_str("Similar open attempts (judge yourself whether they are really the same problem):\n");
            for c in &ranked {
                let a = c.attempt;
                let (m, hw) = hw_line(a.meta.hw.as_ref(), &mine);
                let who = format!("@{} in #{}", a.author, a.number);
                if m == HwMatch::DifferentGpu {
                    different.push(who)
                } else {
                    same.push(who)
                }
                let _ = writeln!(out, "- #{} by @{} ({}) {}", a.number, a.author, a.url, a.title);
                let _ = writeln!(out, "  hardware: {hw}");
                if let Some(h) = field(&a.body, "Hypothesis") {
                    let _ = writeln!(out, "  hypothesis: {}", short(&h, 200));
                }
                if let Some(ap) = field(&a.body, "Approach") {
                    let _ = writeln!(out, "  approach: {}", short(&ap, 200));
                }
                let state = match a.meta.state {
                    State::Active => {
                        format!("active, lease until {}", a.meta.lease_expires.format("%Y-%m-%d %H:%M UTC"))
                    }
                    State::Paused => "paused".into(),
                    State::Expired => "expired".into(),
                };
                let _ = writeln!(out, "  state: {state}; last activity {}", a.updated_at.get(..10).unwrap_or("?"));
                match last_progress(repo, a) {
                    Some(p) => {
                        let _ = writeln!(out, "  last progress {p}");
                    }
                    None => out.push_str("  last progress: none posted yet\n"),
                }
            }
            out.push('\n');
        }
    } else {
        out.push_str("Open attempts: unavailable (no GitHub repo configured or offline).\n\n");
    }

    let kb = ctx.kb()?;
    let hits =
        search(&kb, &Query { text: text.into(), scope: scope_path.map(String::from), limit: 8, ..Default::default() });
    let _ = write!(out, "Related findings:\n{}\n", hits_text(&hits));

    out.push_str("What to do now:\n");
    if same.is_empty() && different.is_empty() {
        out.push_str(
            "- Nobody seems to be on this. If the findings above don't already answer it, open an attempt \
             (open_attempt) before investigating, and record findings as you go.\n",
        );
    } else {
        if !same.is_empty() {
            let _ = writeln!(out, "- Already being worked on with comparable hardware by {}.", same.join(", "));
        }
        if !different.is_empty() {
            let _ = writeln!(
                out,
                "- Also being worked on by {}, but on a different GPU vendor than yours, so their approach may not apply to you.",
                different.join(", ")
            );
        }
        out.push_str(
            "- Before investigating, tell the user who is on it, on what hardware, and the latest progress, then ask \
             whether to (a) follow or help an existing attempt (note_attempt / reproduce their findings on this \
             machine), (b) open a new attempt with a different angle or for this hardware, or (c) proceed anyway.\n",
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn similar_wording_matches() {
        let a = "Character faces render black on AMD";
        assert!(similarity("oh, the faces of the characters are black", a) >= 0.99);
        assert!(similarity("black faces on characters in cutscenes", a) >= 0.6);
        assert!(similarity("audio crackles in menus", a) < 0.1);
    }

    #[test]
    fn hw_comparison() {
        let amd = Hw { os: "linux".into(), gpu_vendors: vec!["amd".into()], summary: "a".into() };
        let amd_win = Hw { os: "windows".into(), gpu_vendors: vec!["amd".into()], summary: "b".into() };
        let nv = Hw { os: "linux".into(), gpu_vendors: vec!["nvidia".into()], summary: "c".into() };
        assert_eq!(amd.compare(&amd), HwMatch::Same);
        assert_eq!(amd.compare(&amd_win), HwMatch::DifferentOs);
        assert_eq!(amd.compare(&nv), HwMatch::DifferentGpu);
        let (_, line) = hw_line(Some(&amd), &nv);
        assert!(line.contains("they are on amd, you are on nvidia"), "{line}");
    }

    #[test]
    fn body_fields() {
        let body = "<!-- orc:attempt {} -->\n**Scope:** `x`\n**Problem:** faces black\n\n**Hypothesis:** shader precision\n\n---\nfooter words";
        assert_eq!(field(body, "Hypothesis").as_deref(), Some("shader precision"));
        assert!(!problem_text(body).contains("footer"));
    }
}

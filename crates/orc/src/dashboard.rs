//! `orc dashboard`: builds a static dashboard (GitHub Pages) from the
//! knowledge base and GitHub issues. An Action reruns it on every change, so
//! the page is near-live without any server; the page polls `data.json`.
//!
//! The hub (this repo) also lists other projects in `registry.toml`:
//!
//! ```toml
//! [[projects]]
//! repo = "someone/some-game-recomp-kb"
//! ```

use crate::attempts::{self, Hw};
use crate::check::field;
use crate::gh;
use crate::project::{data_dir, git};
use anyhow::{Context, Result};
use chrono::{SecondsFormat, Utc};
use orc_kb::{FsSource, Kb, Project, PROJECT_FILE};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const REGISTRY_FILE: &str = "registry.toml";
const SITE_FILES: &[&str] = &["index.html", "app.js", "style.css", "favicon.svg"];
const MAX_ACTIVITY: usize = 400;
const MAX_BODY: usize = 4000;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    #[serde(default)]
    projects: Vec<RegistryEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryEntry {
    /// GitHub owner/name of the project's knowledge repo.
    repo: String,
}

struct Source {
    repo: Option<String>,
    root: PathBuf,
}

pub fn build(root: &Path, hub_repo: Option<&str>, site: &Path, out: &Path, offline: bool) -> Result<()> {
    std::fs::create_dir_all(out)?;
    for f in SITE_FILES {
        let from = site.join(f);
        if from.exists() {
            std::fs::copy(&from, out.join(f)).with_context(|| format!("copying {}", from.display()))?;
        }
    }

    let mut sources = vec![Source { repo: hub_repo.map(String::from), root: root.to_path_buf() }];
    let mut warnings: Vec<String> = Vec::new();
    if let Ok(text) = std::fs::read_to_string(root.join(REGISTRY_FILE)) {
        let reg: Registry = toml::from_str(&text).with_context(|| format!("invalid {REGISTRY_FILE}"))?;
        for e in reg.projects {
            if gh::check_repo(&e.repo).is_err() || Some(e.repo.as_str()) == hub_repo {
                continue;
            }
            if offline {
                continue;
            }
            match fetch(&e.repo) {
                Ok(dir) => sources.push(Source { repo: Some(e.repo), root: dir }),
                Err(err) => warnings.push(format!("{}: {err:#}", e.repo)),
            }
        }
    }

    let mut d = Data::default();
    for s in &sources {
        if let Err(e) = d.add_project(s, out, offline) {
            warnings.push(format!("{}: {e:#}", s.repo.as_deref().unwrap_or("local")));
        }
    }
    d.activity.sort_by(|a, b| b["time"].as_str().cmp(&a["time"].as_str()));
    d.activity.truncate(MAX_ACTIVITY);
    for w in &warnings {
        eprintln!("orc dashboard: {w}");
    }

    let data = json!({
        "generated_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "hub": hub_repo,
        "projects": d.projects,
        "games": d.games,
        "findings": d.findings,
        "attempts": d.attempts,
        "closed_attempts": d.closed,
        "problems": d.problems,
        "envs": d.envs,
        "activity": d.activity,
        "warnings": warnings,
    });
    std::fs::write(out.join("data.json"), serde_json::to_vec(&data)?)?;
    Ok(())
}

/// Shallow clone (or refresh) a registered project's knowledge repo.
fn fetch(repo: &str) -> Result<PathBuf> {
    let dir = data_dir().join("registry").join(repo);
    if dir.join(".git").exists() {
        git(Some(&dir), &["fetch", "--quiet", "--depth", "1", "origin"])?;
        git(Some(&dir), &["reset", "--quiet", "--hard", "FETCH_HEAD"])?;
    } else {
        std::fs::create_dir_all(dir.parent().expect("has parent"))?;
        let url = format!("https://github.com/{repo}.git");
        git(None, &["clone", "--quiet", "--depth", "1", &url, &dir.to_string_lossy()])?;
    }
    Ok(dir)
}

#[derive(Default)]
struct Data {
    projects: Vec<Value>,
    games: Vec<Value>,
    findings: Vec<Value>,
    attempts: Vec<Value>,
    closed: Vec<Value>,
    problems: Vec<Value>,
    envs: Map<String, Value>,
    activity: Vec<Value>,
}

fn s(v: &Value, k: &str) -> String {
    v[k].as_str().unwrap_or_default().to_string()
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_string()
    } else {
        format!("{}...", text.chars().take(max).collect::<String>())
    }
}

impl Data {
    fn event(&mut self, time: &str, kind: &str, repo: &str, who: &str, text: &str, target: Value) {
        self.activity
            .push(json!({ "time": time, "type": kind, "repo": repo, "who": who, "text": text, "target": target }));
    }

    fn add_project(&mut self, src: &Source, out: &Path, offline: bool) -> Result<()> {
        let kb = Kb::load(&FsSource::new(&src.root));
        let project: Option<Project> =
            std::fs::read_to_string(src.root.join(PROJECT_FILE)).ok().and_then(|t| toml::from_str(&t).ok());
        let repo = src.repo.clone().unwrap_or_else(|| "local".into());
        let blob = |path: &str| src.repo.as_ref().map(|r| format!("https://github.com/{r}/blob/HEAD/{path}"));

        self.projects.push(json!({
            "repo": repo,
            "name": project.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| repo.clone()),
            "default_scope": project.as_ref().map(|p| p.default_scope.clone()),
            "url": src.repo.as_ref().map(|r| format!("https://github.com/{r}")),
            "findings": kb.findings.len(),
            "games": kb.games.len(),
        }));

        for (id, json) in &kb.envs {
            if let Ok(env) = serde_json::from_str::<orc_env::Environment>(json) {
                self.envs.insert(id.clone(), serde_json::to_value(Hw::from_env(&env))?);
            }
        }

        let suspects = kb.suspects();
        for f in kb.visible() {
            let path = format!("{}/{}.md", orc_kb::FINDINGS_DIR, f.id);
            self.findings.push(json!({
                "id": f.id, "repo": repo, "scope": f.scope, "subject": f.subject, "kind": f.kind,
                "claim": f.claim, "status": f.status, "created": f.created,
                "author": f.author.github, "agent": f.author.agent, "model": f.author.model,
                "env": f.env, "attempt": f.attempt, "depends_on": f.depends_on,
                "evidence": f.evidence, "suspect": suspects.contains(&f.id),
                "reproductions": f.reproductions, "disputes": f.disputes,
                "retracted_reason": f.retracted_reason,
                "body": truncate(&f.body, MAX_BODY), "url": blob(&path),
            }));
            self.event(&f.created, "finding", &repo, &f.author.github, &f.subject, json!({ "finding": f.id }));
            for r in &f.reproductions {
                let kind = if r.result == orc_kb::ReproResult::Reproduced { "reproduced" } else { "not-reproduced" };
                self.event(&r.date, kind, &repo, &r.github, &f.subject, json!({ "finding": f.id }));
            }
            for x in &f.disputes {
                self.event(&x.date, "disputed", &repo, &x.github, &f.subject, json!({ "finding": f.id }));
            }
        }

        for (_, g) in kb.games.values() {
            let cover = match &g.cover {
                Some(c) if c.starts_with("https://") => Some(c.clone()),
                Some(c) => copy_cover(&src.root, c, &repo, out),
                None => None,
            };
            let mut v = serde_json::to_value(g)?;
            v["scope"] = json!(g.scope());
            v["repo"] = json!(repo);
            v["cover"] = json!(cover);
            self.games.push(v);
        }

        let Some(gh_repo) = src.repo.as_deref().filter(|_| !offline) else { return Ok(()) };
        self.add_github(gh_repo)
    }

    fn add_github(&mut self, repo: &str) -> Result<()> {
        let mut attempt_numbers = BTreeSet::new();
        for a in attempts::list(repo)? {
            attempt_numbers.insert(a.number);
            self.event(&a.created_at, "attempt-opened", repo, &a.author, &a.title, json!({ "attempt": a.number }));
            self.attempts.push(json!({
                "repo": repo, "number": a.number, "url": a.url, "title": a.title, "author": a.author,
                "scope": a.meta.scope, "state": a.meta.state, "lease_expires": a.meta.lease_expires,
                "lease_hours": a.meta.lease_hours,
                "hw": a.meta.hw, "env": a.meta.env, "problem": a.meta.problem, "comments": a.comments,
                "created_at": a.created_at, "updated_at": a.updated_at,
                "hypothesis": field(&a.body, "Hypothesis"), "approach": field(&a.body, "Approach"),
                "problem_text": field(&a.body, "Problem"),
            }));
        }

        let closed = gh::api(
            "GET",
            &format!(
                "repos/{repo}/issues?labels={}&state=closed&sort=updated&direction=desc&per_page=50",
                attempts::LABEL_ATTEMPT
            ),
            None,
        )?;
        for v in closed.as_array().into_iter().flatten() {
            let Some(a) = attempts::from_issue(v) else { continue };
            attempt_numbers.insert(a.number);
            let closed_at = s(v, "closed_at");
            let reason = s(v, "state_reason");
            self.event(&a.created_at, "attempt-opened", repo, &a.author, &a.title, json!({ "attempt": a.number }));
            self.event(
                &closed_at,
                "attempt-closed",
                repo,
                &a.author,
                &a.title,
                json!({ "attempt": a.number, "reason": reason }),
            );
            self.closed.push(json!({
                "repo": repo, "number": a.number, "url": a.url, "title": a.title, "author": a.author,
                "scope": a.meta.scope, "hw": a.meta.hw, "closed_at": closed_at, "reason": reason,
                "created_at": a.created_at,
            }));
        }

        let problems = gh::api(
            "GET",
            &format!(
                "repos/{repo}/issues?labels={}&state=all&sort=updated&direction=desc&per_page=50",
                attempts::LABEL_PROBLEM
            ),
            None,
        )?;
        for v in problems.as_array().into_iter().flatten() {
            self.problems.push(json!({
                "repo": repo, "number": v["number"], "url": v["html_url"], "title": v["title"], "state": v["state"],
                "author": v["user"]["login"], "created_at": v["created_at"],
                "sub_issues": v["sub_issues_summary"],
            }));
        }

        // Progress notes and other comments on attempts.
        let comments =
            gh::api("GET", &format!("repos/{repo}/issues/comments?sort=created&direction=desc&per_page=100"), None)?;
        for c in comments.as_array().into_iter().flatten() {
            let n = s(c, "issue_url").rsplit('/').next().and_then(|x| x.parse::<u64>().ok());
            let Some(n) = n.filter(|n| attempt_numbers.contains(n)) else { continue };
            let body = s(c, "body");
            let (kind, text) = if let Some(t) = body.strip_prefix("**Progress:** ") {
                ("progress", t.to_string())
            } else if body.starts_with("**New finding**") {
                continue; // already shown as a finding
            } else if ["**Succeeded:** ", "**Failed:** ", "**Abandoned:** "].iter().any(|p| body.starts_with(p)) {
                ("outcome", body.replacen("**", "", 2))
            } else {
                ("comment", body.replace("**", ""))
            };
            let who = c["user"]["login"].as_str().unwrap_or_default();
            let who = if who == "github-actions[bot]" { "orc sweep" } else { who };
            self.event(
                &s(c, "created_at"),
                kind,
                repo,
                who,
                &truncate(&text, 300),
                json!({ "attempt": n, "url": c["html_url"] }),
            );
        }
        Ok(())
    }
}

/// Copy a cover from the knowledge repo into the site, return its site path.
fn copy_cover(root: &Path, cover: &str, repo: &str, out: &Path) -> Option<String> {
    if !orc_kb::game::is_cover(cover) {
        return None;
    }
    let name = Path::new(cover).file_name()?.to_string_lossy().to_string();
    let dir_name = repo.replace('/', "__");
    let rel = format!("covers/{dir_name}/{name}");
    let dest = out.join(&rel);
    std::fs::create_dir_all(dest.parent()?).ok()?;
    std::fs::copy(root.join(cover), &dest).ok()?;
    Some(rel)
}

/// For tests and previews: number of entries per section.
#[allow(dead_code)]
pub fn summary(data: &Value) -> BTreeMap<&'static str, usize> {
    ["games", "findings", "attempts", "closed_attempts", "problems", "activity"]
        .into_iter()
        .map(|k| (k, data[k].as_array().map_or(0, |a| a.len())))
        .collect()
}

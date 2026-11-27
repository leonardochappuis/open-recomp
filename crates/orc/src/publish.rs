//! Writing findings: validated locally first, then either written straight
//! into a local knowledge base or sent as a pull request from orc's own clone
//! (never from the user's working tree).

use crate::gh;
use crate::project::{data_dir, git, Ctx, Mode};
use anyhow::{bail, Context, Result};
use chrono::{SecondsFormat, Utc};
use orc_env::Environment;
use orc_kb::validate::{check_change, Report};
use orc_kb::{
    finding, Author, Dispute, Evidence, FileSource, Finding, FsSource, Game, Kb, Kind, ReproResult, Reproduction,
    Status, ENVS_DIR, FINDINGS_DIR,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;

pub struct NewFinding {
    pub scope: String,
    pub subject: String,
    pub kind: Kind,
    pub claim: String,
    pub body: String,
    pub evidence: Vec<Evidence>,
    pub depends_on: Vec<String>,
    pub attempt: Option<u64>,
    pub agent: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct Published {
    pub id: String,
    pub status: Status,
    /// Pull request URL, or the file written in local mode.
    pub location: String,
    /// Things a human has to look at before it can merge.
    pub needs_human: Vec<String>,
}

/// Who is writing. GitHub login when available.
pub fn login(ctx: &Ctx) -> Result<String> {
    if let Ok(u) = std::env::var("ORC_USER") {
        return Ok(u);
    }
    match ctx.mode {
        Mode::GitHub { .. } => gh::viewer(),
        Mode::Local { .. } => Ok(gh::viewer().unwrap_or_else(|_| "local".into())),
    }
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// The current machine's env, plus its file if the knowledge base lacks it.
fn env_files(kb: &Kb, files: &mut BTreeMap<String, String>) -> String {
    let env = Environment::collect();
    let id = env.fingerprint();
    if !kb.envs.contains_key(&id) {
        let json = serde_json::to_string_pretty(&env).expect("serializable") + "\n";
        files.insert(format!("{ENVS_DIR}/{id}.json"), json);
    }
    id
}

pub fn propose(ctx: &Ctx, nf: NewFinding) -> Result<Published> {
    let kb = ctx.kb()?;
    let who = login(ctx)?;
    let mut files = BTreeMap::new();
    let env = env_files(&kb, &mut files);
    let created = now();
    let f = Finding {
        id: finding::new_id(&who, &created, &nf.claim),
        scope: nf.scope,
        subject: nf.subject,
        kind: nf.kind,
        claim: nf.claim,
        status: Status::Proposed,
        created,
        depends_on: nf.depends_on,
        attempt: nf.attempt,
        env: Some(env),
        retracted_reason: None,
        author: Author { github: who.clone(), agent: nf.agent, model: nf.model },
        evidence: nf.evidence,
        reproductions: vec![],
        disputes: vec![],
        body: nf.body,
    };
    files.insert(format!("{FINDINGS_DIR}/{}.md", f.id), f.render());
    let title = format!("finding {}: {}", f.id, f.subject);
    let pr_body = format!(
        "New finding (proposed) in `{}`.\n\n> {}\n\n{}",
        f.scope,
        f.claim,
        f.attempt.map(|n| format!("From attempt #{n}.")).unwrap_or_default()
    );
    let out = write(ctx, &kb, &who, &f, files, &title, &pr_body)?;
    if let (Some(n), Mode::GitHub { repo, .. }) = (f.attempt, &ctx.mode) {
        let body = format!("**New finding** `{}` (proposed): {}\n\n{}", f.id, f.claim, out.location);
        if let Err(e) = gh::api("POST", &format!("repos/{repo}/issues/{n}/comments"), Some(&json!({ "body": body }))) {
            eprintln!("orc: could not comment on attempt #{n}: {e}");
        }
        let _ = crate::state::LocalState::update(|s| s.touch(repo, n));
    }
    Ok(out)
}

pub fn reproduce(
    ctx: &Ctx,
    id: &str,
    result: ReproResult,
    notes: Option<String>,
    evidence: Vec<Evidence>,
) -> Result<Published> {
    let kb = ctx.kb()?;
    let who = login(ctx)?;
    let mut f = kb.get(id).with_context(|| format!("no finding {id}"))?.clone();
    if f.author.github.eq_ignore_ascii_case(&who) {
        bail!("you wrote {id}; someone else has to reproduce it");
    }
    let mut files = BTreeMap::new();
    let env = env_files(&kb, &mut files);
    f.reproductions.push(Reproduction { github: who.clone(), result, date: now(), env: Some(env), notes, evidence });
    if result == ReproResult::Reproduced && f.status == Status::Proposed {
        f.status = Status::Reproduced;
    }
    let path = format!("{FINDINGS_DIR}/{id}.md");
    files.insert(path, f.render());
    let title = format!("{} {id}", if result == ReproResult::Reproduced { "reproduce" } else { "could not reproduce" });
    write(ctx, &kb, &who, &f, files, &title, &format!("Reproduction attempt of `{id}`: {result}."))
}

pub fn dispute(ctx: &Ctx, id: &str, reason: String, evidence: Vec<Evidence>) -> Result<Published> {
    let kb = ctx.kb()?;
    let who = login(ctx)?;
    let mut f = kb.get(id).with_context(|| format!("no finding {id}"))?.clone();
    if f.status == Status::Retracted {
        bail!("{id} is already retracted");
    }
    let mut files = BTreeMap::new();
    let env = env_files(&kb, &mut files);
    f.disputes.push(Dispute { github: who.clone(), reason, date: now(), env: Some(env), evidence });
    f.status = Status::Disputed;
    files.insert(format!("{FINDINGS_DIR}/{id}.md"), f.render());
    write(ctx, &kb, &who, &f, files, &format!("dispute {id}"), &format!("Dispute of `{id}`."))
}

pub fn retract(ctx: &Ctx, id: &str, reason: String) -> Result<Published> {
    let kb = ctx.kb()?;
    let who = login(ctx)?;
    let mut f = kb.get(id).with_context(|| format!("no finding {id}"))?.clone();
    f.status = Status::Retracted;
    f.retracted_reason = Some(reason);
    let mut files = BTreeMap::new();
    files.insert(format!("{FINDINGS_DIR}/{id}.md"), f.render());
    write(ctx, &kb, &who, &f, files, &format!("retract {id}"), &format!("Retraction of `{id}`."))
}

/// Validate the change as CI will, then write it.
fn write(
    ctx: &Ctx,
    base: &Kb,
    who: &str,
    f: &Finding,
    files: BTreeMap<String, String>,
    title: &str,
    pr_body: &str,
) -> Result<Published> {
    let files = files.into_iter().map(|(k, v)| (k, v.into_bytes())).collect();
    let primary = format!("{FINDINGS_DIR}/{}.md", f.id);
    let marker = format!("<!-- orc:finding {} -->", f.id);
    let (location, needs_human) = write_files(ctx, base, who, &f.id, &primary, files, title, pr_body, &marker)?;
    Ok(Published { id: f.id.clone(), status: f.status, location, needs_human })
}

/// Validate, then write files locally or open a pull request. Returns where it
/// went and what needs a human.
#[allow(clippy::too_many_arguments)]
fn write_files(
    ctx: &Ctx,
    base: &Kb,
    who: &str,
    key: &str,
    primary: &str,
    files: BTreeMap<String, Vec<u8>>,
    title: &str,
    pr_body: &str,
    marker: &str,
) -> Result<(String, Vec<String>)> {
    let overlay = Overlay { base: FsSource::new(ctx.kb_root()), files: &files };
    let head = Kb::load(&overlay);
    let paths: Vec<String> = files.keys().cloned().collect();
    let report: Report = check_change(base, &head, Some(who), &paths);
    if !report.ok() {
        bail!("the change would be rejected:\n- {}", report.errors.join("\n- "));
    }
    let location = match &ctx.mode {
        Mode::Local { root } => {
            for (p, data) in &files {
                let full = root.join(p);
                std::fs::create_dir_all(full.parent().expect("has parent"))?;
                std::fs::write(&full, data)?;
            }
            root.join(primary).display().to_string()
        }
        Mode::GitHub { repo, clone } => {
            let slug: String =
                key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' }).collect();
            let branch = format!("orc/{slug}-{}", Utc::now().format("%Y%m%d%H%M%S"));
            let body = format!("{pr_body}\n\n{marker}\n_Opened by orc._");
            open_pr(ctx, repo, clone, &branch, &files, title, &body)?
        }
    };
    Ok((location, report.needs_human))
}

/// Add or update a game in the catalog. `cover_file` is a local image copied
/// into `kb/games/covers/`.
pub fn save_game(ctx: &Ctx, mut game: Game, cover_file: Option<&Path>) -> Result<(String, Vec<String>)> {
    let kb = ctx.kb()?;
    let who = login(ctx)?;
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    if let Some(src) = cover_file {
        let ext = src.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
        if !["png", "jpg", "jpeg", "webp"].contains(&ext.as_str()) {
            bail!("cover must be a .png, .jpg or .webp file");
        }
        let data = std::fs::read(src).with_context(|| format!("reading {}", src.display()))?;
        if data.len() > 1024 * 1024 {
            bail!("cover must be at most 1 MiB");
        }
        let path = format!("{}/{}-{}.{ext}", orc_kb::game::COVERS_DIR, game.platform, game.id.to_lowercase());
        files.insert(path.clone(), data);
        game.cover = Some(path);
    }
    let path = game.path();
    files.insert(path.clone(), game.render().into_bytes());
    let title = format!("game: {} ({})", game.name, game.platform);
    let body = format!("Catalog entry for **{}** (`{}`), status {:?}.", game.name, game.scope(), game.status);
    let key = format!("game-{}-{}", game.platform, game.id.to_lowercase());
    write_files(ctx, &kb, &who, &key, &path, files, &title, &body, "<!-- orc:game -->")
}

/// Files on disk plus pending changes.
struct Overlay<'a> {
    base: FsSource,
    files: &'a BTreeMap<String, Vec<u8>>,
}

impl FileSource for Overlay<'_> {
    fn list(&self, dir: &str) -> Vec<String> {
        let mut v = self.base.list(dir);
        let prefix = format!("{dir}/");
        for p in self.files.keys() {
            if p.strip_prefix(&prefix).is_some_and(|r| !r.contains('/')) && !v.contains(p) {
                v.push(p.clone());
            }
        }
        v.sort();
        v
    }
    fn read(&self, path: &str) -> Option<String> {
        match self.files.get(path) {
            Some(data) => String::from_utf8(data.clone()).ok(),
            None => self.base.read(path),
        }
    }
}

/// Use gh as git's credential helper for this one command, so pushing works
/// without `gh auth setup-git`.
const GH_CRED: [&str; 4] = ["-c", "credential.helper=", "-c", "credential.helper=!gh auth git-credential"];

fn open_pr(
    ctx: &Ctx,
    repo: &str,
    clone: &Path,
    branch: &str,
    files: &BTreeMap<String, Vec<u8>>,
    title: &str,
    body: &str,
) -> Result<String> {
    ctx.sync(true)?;
    let who = gh::viewer()?;
    let info = gh::api("GET", &format!("repos/{repo}"), None)?;
    let can_push = info["permissions"]["push"].as_bool().unwrap_or(false);
    let (remote, head) = if can_push {
        ("origin".to_string(), branch.to_string())
    } else {
        let fork = gh::api("POST", &format!("repos/{repo}/forks"), Some(&json!({ "default_branch_only": true })))?;
        let full = fork["full_name"].as_str().context("fork failed")?.to_string();
        gh::check_repo(&full)?;
        let url = format!("https://github.com/{full}.git");
        if git(Some(clone), &["remote", "set-url", "fork", &url]).is_err() {
            git(Some(clone), &["remote", "add", "fork", &url])?;
        }
        ("fork".to_string(), format!("{who}:{branch}"))
    };

    let work = data_dir().join("work").join(branch.replace('/', "_"));
    let work_s = work.to_string_lossy().to_string();
    let _ = git(Some(clone), &["worktree", "remove", "--force", &work_s]);
    git(Some(clone), &["worktree", "add", "--quiet", "-B", branch, &work_s, "origin/HEAD"])?;
    let result = (|| -> Result<String> {
        for (p, data) in files {
            let full = work.join(p);
            std::fs::create_dir_all(full.parent().expect("has parent"))?;
            std::fs::write(full, data)?;
        }
        git(Some(&work), &["add", "--", "kb"])?;
        let name = format!("user.name={who}");
        let email = format!("user.email={who}@users.noreply.github.com");
        git(Some(&work), &["-c", &name, "-c", &email, "commit", "--quiet", "-m", title])?;
        // A new fork can take a few seconds to accept pushes.
        let mut last = None;
        for _ in 0..5 {
            let mut args: Vec<&str> = GH_CRED.to_vec();
            args.extend(["push", "--quiet", "--force", &remote, branch]);
            match git(Some(&work), &args) {
                Ok(_) => {
                    last = None;
                    break;
                }
                Err(e) => {
                    last = Some(e);
                    std::thread::sleep(std::time::Duration::from_secs(3));
                }
            }
        }
        if let Some(e) = last {
            return Err(e.context("push failed"));
        }
        let url = gh::run(&["pr", "create", "--repo", repo, "--head", &head, "--title", title, "--body", body], None)?;
        Ok(url.trim().to_string())
    })();
    let _ = git(Some(clone), &["worktree", "remove", "--force", &work_s]);
    result
}

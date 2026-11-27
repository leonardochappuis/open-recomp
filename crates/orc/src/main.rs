mod attempts;
mod check;
mod context;
mod dashboard;
mod gh;
mod hooks;
mod mcp;
mod project;
mod publish;
mod state;

use anyhow::{bail, Context, Result};
use attempts::{OpenArgs, Outcome};
use clap::{Parser, Subcommand};
use orc_env::Environment;
use orc_kb::search::{search, Query};
use orc_kb::validate::{check, check_change};
use orc_kb::{Evidence, FsSource, GitSource, Kb, Kind, Project, ReproResult, Status};
use project::Ctx;
use std::path::PathBuf;

/// open-recomp: shared knowledge and attempts for recompilation projects.
#[derive(Parser)]
#[command(name = "orc", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print this machine's hardware/software profile (coarse, no identifiers).
    Env {
        #[arg(long)]
        json: bool,
    },
    /// Set up .orc/ in the current repo and create the GitHub labels.
    Init {
        #[arg(long)]
        name: String,
        /// GitHub owner/name of the knowledge repo, or "self".
        #[arg(long, default_value = "self")]
        kb: String,
        #[arg(long)]
        scope: String,
        /// GitHub owner/name of this repo.
        #[arg(long)]
        repo: Option<String>,
        /// Your GitHub login, listed as the first coordinator.
        #[arg(long)]
        coordinator: Option<String>,
    },
    /// Refresh orc's clone of the knowledge repo.
    Sync,
    /// Search findings.
    Search {
        query: Vec<String>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long, value_delimiter = ',')]
        status: Vec<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    /// Show one finding.
    Show { id: String },
    /// Context an agent gets at session start.
    Context {
        #[arg(long)]
        scope: Option<String>,
    },
    /// Who is already working on a problem like this one?
    Check {
        description: Vec<String>,
        #[arg(long)]
        scope: Option<String>,
    },
    /// Write findings.
    #[command(subcommand)]
    Finding(FindingCmd),
    /// Attempts (GitHub issues).
    #[command(subcommand)]
    Attempt(AttemptCmd),
    /// Game catalog.
    #[command(subcommand)]
    Game(GameCmd),
    /// Build the dashboard data (data.json) and site.
    Dashboard {
        /// Output directory.
        #[arg(long, default_value = "_site")]
        out: PathBuf,
        /// Built dashboard site (`npm run build` in web/).
        #[arg(long, default_value = "web/dist")]
        site: PathBuf,
        /// Knowledge repo root (default: current directory).
        #[arg(long)]
        root: Option<PathBuf>,
        /// GitHub owner/name for attempts and links (default: from .orc/project.toml).
        #[arg(long)]
        repo: Option<String>,
        /// Skip GitHub (no attempts, no registry), for local previews.
        #[arg(long)]
        offline: bool,
    },
    /// Validate the knowledge base, or a change to it (CI).
    Validate {
        /// Directory to validate (default: the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
        /// Base git revision; with --head, validates the change between them.
        #[arg(long, requires = "head")]
        base: Option<String>,
        #[arg(long)]
        head: Option<String>,
        /// GitHub login that opened the pull request.
        #[arg(long)]
        pr_author: Option<String>,
        /// Exit 3 unless the change is safe to merge without a human.
        #[arg(long)]
        require_auto: bool,
        #[arg(long)]
        json: bool,
    },
    /// Remind, expire and close attempts (run by the scheduled Action).
    Sweep {
        #[arg(long)]
        repo: String,
        /// Directory with .orc/policy.toml (default: current directory).
        #[arg(long)]
        root: Option<PathBuf>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Local MCP server over stdio.
    Mcp,
    /// Claude Code hooks.
    #[command(subcommand)]
    Hook(HookCmd),
}

#[derive(Subcommand)]
enum FindingCmd {
    /// Propose a new finding.
    New {
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        subject: String,
        #[arg(long)]
        kind: Kind,
        #[arg(long)]
        claim: String,
        #[arg(long, default_value = "")]
        details: String,
        /// type:ref, e.g. commit:repo@abc123 (repeatable)
        #[arg(long)]
        evidence: Vec<String>,
        #[arg(long)]
        depends_on: Vec<String>,
        #[arg(long)]
        attempt: Option<u64>,
        /// Agent that produced it (claude-code, codex, ...).
        #[arg(long, default_value = "orc-cli")]
        agent: String,
        /// Model id, if an agent produced it.
        #[arg(long)]
        model: Option<String>,
    },
    /// Record a reproduction of someone else's finding.
    Reproduce {
        id: String,
        #[arg(long)]
        failed: bool,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        evidence: Vec<String>,
    },
    /// Dispute a finding.
    Dispute {
        id: String,
        #[arg(long)]
        reason: String,
        #[arg(long)]
        evidence: Vec<String>,
    },
    /// Retract your own finding.
    Retract {
        id: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Subcommand)]
enum AttemptCmd {
    Open {
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        problem: String,
        #[arg(long)]
        hypothesis: String,
        #[arg(long)]
        approach: String,
        #[arg(long)]
        problem_issue: Option<u64>,
        #[arg(long)]
        lease_hours: Option<u32>,
    },
    List,
    Renew {
        number: u64,
        #[arg(long)]
        hours: Option<u32>,
    },
    Pause {
        number: u64,
    },
    Note {
        number: u64,
        text: Vec<String>,
    },
    Close {
        number: u64,
        #[arg(long, value_enum)]
        outcome: Outcome,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        finding: Vec<String>,
    },
}

// Parsed once at startup; variant size does not matter.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum GameCmd {
    /// Add or update a game (opens a pull request; a maintainer merges it).
    Add {
        #[arg(long)]
        platform: String,
        /// Title id or serial.
        #[arg(long)]
        id: String,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "planned")]
        status: orc_kb::GameStatus,
        #[arg(long)]
        region: Option<String>,
        #[arg(long)]
        year: Option<u16>,
        /// Recomp repo owner/name (repeatable).
        #[arg(long = "repo")]
        repos: Vec<String>,
        /// Cover image URL (https).
        #[arg(long, conflicts_with = "cover_file")]
        cover: Option<String>,
        /// Local cover image to upload (png/jpg/webp, max 1 MiB).
        #[arg(long)]
        cover_file: Option<PathBuf>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long)]
        description: Option<String>,
    },
    List,
}

#[derive(Subcommand)]
enum HookCmd {
    SessionStart,
    Stop,
}

fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("orc: {e:#}");
        std::process::exit(1);
    }
}

fn ctx() -> Result<Ctx> {
    Ctx::discover(&std::env::current_dir()?)
}

fn parse_evidence(items: &[String]) -> Result<Vec<Evidence>> {
    items
        .iter()
        .map(|e| {
            let (t, r) = e.split_once(':').with_context(|| format!("evidence {e:?} must be type:ref"))?;
            Ok(Evidence { kind: t.parse().map_err(anyhow::Error::msg)?, reference: r.to_string() })
        })
        .collect()
}

fn print_published(p: &publish::Published) {
    println!("{} ({}) -> {}", p.id, p.status, p.location);
    for n in &p.needs_human {
        println!("  needs a human: {n}");
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.cmd {
        Cmd::Env { json } => {
            let env = Environment::collect();
            if json {
                let out = serde_json::json!({ "id": env.fingerprint(), "env": env });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("{}  [{}]", env.summary(), env.fingerprint());
            }
        }
        Cmd::Init { name, kb, scope, repo, coordinator } => init(name, kb, scope, repo, coordinator)?,
        Cmd::Sync => {
            let c = ctx()?;
            c.sync(true)?;
            println!("knowledge base at {}", c.kb_root().display());
        }
        Cmd::Search { query, scope, status, limit } => {
            let kb = ctx()?.kb()?;
            let statuses = status
                .iter()
                .map(|s| {
                    serde_json::from_value::<Status>(serde_json::json!(s))
                        .map_err(|_| anyhow::anyhow!("unknown status {s}"))
                })
                .collect::<Result<Vec<_>>>()?;
            let hits = search(&kb, &Query { text: query.join(" "), scope, statuses, limit });
            print!("{}", context::hits_text(&hits));
        }
        Cmd::Show { id } => {
            let kb = ctx()?.kb()?;
            let f = kb.get(&id).with_context(|| format!("no finding {id}"))?;
            print!("{}", f.render());
        }
        Cmd::Context { scope } => {
            let c = ctx()?;
            let kb = c.kb()?;
            let attempts = c.repo().ok().and_then(|r| attempts::list(r).ok());
            let me = publish::login(&c).ok();
            let scope = scope.unwrap_or_else(|| c.project.default_scope.clone());
            print!("{}", context::for_scope(&kb, &scope, attempts.as_deref(), me.as_deref()));
        }
        Cmd::Check { description, scope } => {
            print!("{}", check::check(&ctx()?, &description.join(" "), scope.as_deref())?);
        }
        Cmd::Finding(f) => finding(f)?,
        Cmd::Attempt(a) => attempt(a)?,
        Cmd::Game(g) => game(g)?,
        Cmd::Dashboard { out, site, root, repo, offline } => {
            let root = root.unwrap_or(std::env::current_dir()?);
            let repo = match repo {
                Some(r) => Some(r),
                None => Ctx::discover(&root).ok().and_then(|c| c.project.kb_repo().map(String::from)),
            };
            dashboard::build(&root, repo.as_deref(), &site, &out, offline)?;
            println!("dashboard written to {}", out.display());
        }
        Cmd::Validate { root, base, head, pr_author, require_auto, json } => {
            let root = root.unwrap_or(std::env::current_dir()?);
            let report = match (&base, &head) {
                (Some(b), Some(h)) => {
                    // Compare against where the change branched off, so findings merged to the
                    // base branch afterwards don't look deleted.
                    let mb = project::git(Some(&root), &["merge-base", b, h])?;
                    let base_kb = Kb::load(&GitSource::new(&root, mb.trim()).map_err(anyhow::Error::msg)?);
                    let head_kb = Kb::load(&GitSource::new(&root, h).map_err(anyhow::Error::msg)?);
                    let range = format!("{b}...{h}");
                    let diff = project::git(Some(&root), &["diff", "--name-only", "--no-renames", &range])?;
                    let paths: Vec<String> = diff.lines().map(String::from).collect();
                    check_change(&base_kb, &head_kb, pr_author.as_deref(), &paths)
                }
                _ => check(&Kb::load(&FsSource::new(&root))),
            };
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                for e in &report.errors {
                    println!("error: {e}");
                }
                for n in &report.needs_human {
                    println!("needs a human: {n}");
                }
                if report.auto_mergeable() {
                    println!("ok");
                }
            }
            if !report.ok() {
                std::process::exit(2);
            }
            if require_auto && !report.auto_mergeable() {
                std::process::exit(3);
            }
        }
        Cmd::Sweep { repo, root, dry_run } => {
            gh::check_repo(&repo)?;
            let root = root.unwrap_or(std::env::current_dir()?);
            let policy = Kb::load(&FsSource::new(root)).policy;
            for line in attempts::sweep(&repo, &policy, dry_run)? {
                println!("{line}");
            }
        }
        Cmd::Mcp => mcp::serve()?,
        Cmd::Hook(HookCmd::SessionStart) => hooks::session_start(),
        Cmd::Hook(HookCmd::Stop) => hooks::stop(),
    }
    Ok(())
}

fn finding(cmd: FindingCmd) -> Result<()> {
    let c = ctx()?;
    let p = match cmd {
        FindingCmd::New { scope, subject, kind, claim, details, evidence, depends_on, attempt, agent, model } => {
            publish::propose(
                &c,
                publish::NewFinding {
                    scope: scope.unwrap_or_else(|| c.project.default_scope.clone()),
                    subject,
                    kind,
                    claim,
                    body: details,
                    evidence: parse_evidence(&evidence)?,
                    depends_on,
                    attempt,
                    agent: Some(agent),
                    model,
                },
            )?
        }
        FindingCmd::Reproduce { id, failed, notes, evidence } => {
            let r = if failed { ReproResult::NotReproduced } else { ReproResult::Reproduced };
            publish::reproduce(&c, &id, r, notes, parse_evidence(&evidence)?)?
        }
        FindingCmd::Dispute { id, reason, evidence } => publish::dispute(&c, &id, reason, parse_evidence(&evidence)?)?,
        FindingCmd::Retract { id, reason } => publish::retract(&c, &id, reason)?,
    };
    print_published(&p);
    Ok(())
}

fn attempt(cmd: AttemptCmd) -> Result<()> {
    let c = ctx()?;
    let repo = c.repo()?.to_string();
    let policy = || c.kb().map(|kb| kb.policy);
    match cmd {
        AttemptCmd::Open { scope, problem, hypothesis, approach, problem_issue, lease_hours } => {
            let env = Environment::collect();
            let scope = scope.unwrap_or_else(|| c.project.default_scope.clone());
            let a = attempts::open(
                &repo,
                &policy()?,
                OpenArgs {
                    scope: &scope,
                    problem: &problem,
                    hypothesis: &hypothesis,
                    approach: &approach,
                    problem_issue,
                    lease_hours,
                    env: Some(&env),
                },
            )?;
            println!("#{} {} (lease until {})", a.number, a.url, a.meta.lease_expires);
        }
        AttemptCmd::List => {
            let all = attempts::list(&repo)?;
            let refs: Vec<&attempts::Attempt> = all.iter().collect();
            print!("{}", if refs.is_empty() { "No open attempts.\n".into() } else { context::attempts_text(&refs) });
        }
        AttemptCmd::Renew { number, hours } => {
            let m = attempts::renew(&repo, number, &policy()?, hours)?;
            println!("#{number} lease until {}", m.lease_expires);
        }
        AttemptCmd::Pause { number } => {
            let m = attempts::pause(&repo, number, &policy()?)?;
            println!("#{number} paused until {}", m.lease_expires);
        }
        AttemptCmd::Note { number, text } => println!("{}", attempts::note(&repo, number, &text.join(" "))?),
        AttemptCmd::Close { number, outcome, summary, finding } => {
            attempts::close(&repo, number, &policy()?, outcome, &summary, &finding)?;
            println!("#{number} closed");
        }
    }
    Ok(())
}

fn game(cmd: GameCmd) -> Result<()> {
    let c = ctx()?;
    match cmd {
        GameCmd::Add { platform, id, name, status, region, year, repos, cover, cover_file, tags, description } => {
            let g = orc_kb::Game { name, platform, id, status, region, year, repos, cover, tags, description };
            let (location, needs_human) = publish::save_game(&c, g, cover_file.as_deref())?;
            println!("{location}");
            for n in needs_human {
                println!("  needs a human: {n}");
            }
        }
        GameCmd::List => {
            let kb = c.kb()?;
            if kb.games.is_empty() {
                println!("No games yet.");
            }
            for (_, g) in kb.games.values() {
                let status = serde_json::to_value(g.status)?.as_str().unwrap_or_default().to_string();
                println!("{:<8} {:<14} {:<12} {}", g.platform, g.id, status, g.name);
            }
        }
    }
    Ok(())
}

fn init(name: String, kb: String, scope: String, repo: Option<String>, coordinator: Option<String>) -> Result<()> {
    if !orc_kb::scope::is_valid(&scope) {
        bail!("invalid scope {scope:?}");
    }
    if kb != "self" {
        gh::check_repo(&kb)?;
    }
    if let Some(r) = &repo {
        gh::check_repo(r)?;
    }
    let root = std::env::current_dir()?;
    let project = Project { name, kb: kb.clone(), default_scope: scope, repo };
    let write_new = |rel: &str, text: String| -> Result<()> {
        let p = root.join(rel);
        if p.exists() {
            println!("kept existing {rel}");
            return Ok(());
        }
        std::fs::create_dir_all(p.parent().expect("has parent"))?;
        std::fs::write(&p, text)?;
        println!("wrote {rel}");
        Ok(())
    };
    write_new(orc_kb::PROJECT_FILE, toml::to_string(&project)?)?;
    if kb == "self" {
        let policy = orc_kb::Policy { coordinators: coordinator.into_iter().collect(), ..Default::default() };
        write_new(orc_kb::POLICY_FILE, toml::to_string(&policy)?)?;
        write_new(orc_kb::MODERATION_FILE, toml::to_string(&orc_kb::Moderation::default())?)?;
        write_new(&format!("{}/.gitkeep", orc_kb::FINDINGS_DIR), String::new())?;
        write_new(&format!("{}/.gitkeep", orc_kb::ENVS_DIR), String::new())?;
    }
    if let Some(r) = project.kb_repo() {
        for (label, color, desc) in attempts::LABELS {
            match gh::run(
                &["label", "create", label, "--repo", r, "--color", color, "--description", desc, "--force"],
                None,
            ) {
                Ok(_) => println!("label {label} ready on {r}"),
                Err(e) => println!("could not create label {label} on {r}: {e}"),
            }
        }
    }
    Ok(())
}

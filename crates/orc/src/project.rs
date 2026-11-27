//! Finding the project, where its knowledge base lives, and local state.

use anyhow::{bail, Context, Result};
use orc_kb::{FsSource, Kb, Project, PROJECT_FILE};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

/// How often reads refresh the managed clone.
const SYNC_EVERY: Duration = Duration::from_secs(10 * 60);

pub struct Ctx {
    pub project: Project,
    pub mode: Mode,
}

pub enum Mode {
    /// Knowledge base is a directory we read and write directly
    /// (no GitHub repo yet, or `ORC_KB_DIR` set). No attempts.
    Local { root: PathBuf },
    /// Knowledge base is a GitHub repo; orc keeps its own clone and writes
    /// through pull requests.
    GitHub { repo: String, clone: PathBuf },
}

impl Ctx {
    /// Walk up from `start` to the nearest `.orc/project.toml`.
    pub fn discover(start: &Path) -> Result<Ctx> {
        let mut dir = Some(start);
        while let Some(d) = dir {
            let file = d.join(PROJECT_FILE);
            if file.is_file() {
                let text = std::fs::read_to_string(&file)?;
                let project: Project = toml::from_str(&text).with_context(|| format!("invalid {}", file.display()))?;
                return Ok(Ctx::new(project, d.to_path_buf()));
            }
            dir = d.parent();
        }
        bail!("no {PROJECT_FILE} found here or in any parent directory (run `orc init`)")
    }

    pub fn new(project: Project, project_dir: PathBuf) -> Ctx {
        let mode = if let Some(dir) = std::env::var_os("ORC_KB_DIR") {
            Mode::Local { root: PathBuf::from(dir) }
        } else if let Some(repo) = project.kb_repo() {
            Mode::GitHub { repo: repo.to_string(), clone: data_dir().join("kb").join(repo) }
        } else {
            Mode::Local { root: project_dir }
        };
        Ctx { project, mode }
    }

    pub fn repo(&self) -> Result<&str> {
        match &self.mode {
            Mode::GitHub { repo, .. } => Ok(repo),
            Mode::Local { .. } => {
                bail!("attempts and pull requests need a GitHub repo: set `repo` (or `kb`) in {PROJECT_FILE}")
            }
        }
    }

    pub fn kb_root(&self) -> &Path {
        match &self.mode {
            Mode::Local { root } => root,
            Mode::GitHub { clone, .. } => clone,
        }
    }

    /// Load the knowledge base, refreshing the clone if it is stale.
    pub fn kb(&self) -> Result<Kb> {
        if let Mode::GitHub { .. } = self.mode {
            self.sync(false)?;
        }
        Ok(Kb::load(&FsSource::new(self.kb_root())))
    }

    /// Clone or update the managed clone. `force` ignores the staleness timer.
    pub fn sync(&self, force: bool) -> Result<()> {
        let Mode::GitHub { repo, clone } = &self.mode else { return Ok(()) };
        crate::gh::check_repo(repo)?;
        if !clone.join(".git").exists() {
            std::fs::create_dir_all(clone.parent().expect("has parent"))?;
            let url = format!("https://github.com/{repo}.git");
            git(None, &["clone", "--quiet", "--depth", "50", &url, &clone.to_string_lossy()])?;
            return Ok(());
        }
        if !force && !is_stale(&clone.join(".git/FETCH_HEAD")) {
            return Ok(());
        }
        git(Some(clone), &["fetch", "--quiet", "origin"])?;
        // The clone is orc's own: always mirror the default branch.
        git(Some(clone), &["reset", "--quiet", "--hard", "origin/HEAD"])?;
        Ok(())
    }
}

fn is_stale(p: &Path) -> bool {
    let modified = std::fs::metadata(p).and_then(|m| m.modified());
    match modified {
        Ok(t) => SystemTime::now().duration_since(t).unwrap_or_default() > SYNC_EVERY,
        Err(_) => true,
    }
}

pub fn git(dir: Option<&Path>, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    if let Some(d) = dir {
        cmd.arg("-C").arg(d);
    }
    let out = cmd.args(args).output().context("could not run git")?;
    if !out.status.success() {
        bail!("git {}: {}", args.first().copied().unwrap_or(""), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `$XDG_DATA_HOME/orc`, `~/.local/share/orc`, or `%LOCALAPPDATA%\orc`.
pub fn data_dir() -> PathBuf {
    base_dir("XDG_DATA_HOME", ".local/share").join("orc")
}

/// `$XDG_STATE_HOME/orc`, `~/.local/state/orc`, or `%LOCALAPPDATA%\orc\state`.
pub fn state_dir() -> PathBuf {
    if cfg!(windows) {
        return data_dir().join("state");
    }
    base_dir("XDG_STATE_HOME", ".local/state").join("orc")
}

fn base_dir(xdg: &str, fallback: &str) -> PathBuf {
    if let Some(p) = std::env::var_os("ORC_HOME") {
        return PathBuf::from(p);
    }
    if cfg!(windows) {
        if let Some(p) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(p);
        }
    }
    if let Some(p) = std::env::var_os(xdg).filter(|p| !p.is_empty()) {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).unwrap_or_default();
    PathBuf::from(home).join(fallback)
}

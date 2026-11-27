//! Where knowledge-base files come from: a directory, or a git revision.
//!
//! Reading a git revision never checks it out, so CI can validate an
//! untrusted pull request's data without placing its files on disk.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

pub trait FileSource {
    /// Repo-relative paths (with `/`) of files directly under `dir`.
    fn list(&self, dir: &str) -> Vec<String>;
    fn read(&self, path: &str) -> Option<String>;
}

pub struct FsSource {
    pub root: PathBuf,
}

impl FsSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

impl FileSource for FsSource {
    fn list(&self, dir: &str) -> Vec<String> {
        let Ok(rd) = std::fs::read_dir(self.root.join(dir)) else { return Vec::new() };
        let mut v: Vec<String> = rd
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|e| format!("{dir}/{}", e.file_name().to_string_lossy()))
            .collect();
        v.sort();
        v
    }

    fn read(&self, path: &str) -> Option<String> {
        std::fs::read_to_string(self.root.join(path)).ok()
    }
}

/// Files of a git revision, read with `git ls-tree` / `git cat-file`.
pub struct GitSource {
    repo: PathBuf,
    rev: String,
}

impl GitSource {
    pub fn new(repo: impl AsRef<Path>, rev: &str) -> Result<Self, String> {
        let s = Self { repo: repo.as_ref().to_path_buf(), rev: rev.to_string() };
        s.git(&["rev-parse", "--verify", &format!("{rev}^{{commit}}")])
            .ok_or_else(|| format!("unknown git revision {rev:?}"))?;
        Ok(s)
    }

    fn git(&self, args: &[&str]) -> Option<String> {
        let out = Command::new("git").arg("-C").arg(&self.repo).args(args).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

impl FileSource for GitSource {
    fn list(&self, dir: &str) -> Vec<String> {
        let spec = format!("{}:{dir}", self.rev);
        let Some(out) = self.git(&["ls-tree", "--name-only", &spec]) else { return Vec::new() };
        let Some(types) = self.git(&["ls-tree", &spec]) else { return Vec::new() };
        // Keep blobs only (skip subtrees).
        let blobs: Vec<&str> = types
            .lines()
            .filter(|l| l.split_whitespace().nth(1) == Some("blob"))
            .filter_map(|l| l.split('\t').nth(1))
            .collect();
        out.lines().filter(|n| blobs.contains(n)).map(|n| format!("{dir}/{n}")).collect()
    }

    fn read(&self, path: &str) -> Option<String> {
        self.git(&["cat-file", "blob", &format!("{}:{path}", self.rev)])
    }
}

/// In-memory files, for tests.
#[derive(Default)]
pub struct MemSource {
    files: BTreeMap<String, String>,
}

impl MemSource {
    pub fn put(&mut self, path: &str, text: &str) {
        self.files.insert(path.to_string(), text.to_string());
    }
}

impl FileSource for MemSource {
    fn list(&self, dir: &str) -> Vec<String> {
        let prefix = format!("{dir}/");
        self.files.keys().filter(|k| k.strip_prefix(&prefix).is_some_and(|rest| !rest.contains('/'))).cloned().collect()
    }

    fn read(&self, path: &str) -> Option<String> {
        self.files.get(path).cloned()
    }
}

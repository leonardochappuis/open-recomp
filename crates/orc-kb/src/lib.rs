//! open-recomp knowledge base.
//!
//! Layout of a knowledge repo:
//!
//! ```text
//! .orc/policy.toml        project policy (coordinators, reproduction and lease rules)
//! kb/moderation.toml      quarantined users and findings
//! kb/findings/F-*.md      one finding per file
//! kb/envs/<id>.json       hardware/software profiles referenced by findings
//! kb/games/<p>-<id>.toml   game catalog (covers in kb/games/covers/)
//! ```

pub mod config;
pub mod finding;
pub mod game;
pub mod scope;
pub mod search;
pub mod source;
pub mod validate;

pub use config::{Moderation, Policy, Project};
pub use finding::{Author, Dispute, Evidence, EvidenceKind, Finding, Kind, ReproResult, Reproduction, Status};
pub use game::{Game, GameStatus, GAMES_DIR};
pub use source::{FileSource, FsSource, GitSource};

use std::collections::{BTreeMap, BTreeSet};

pub const FINDINGS_DIR: &str = "kb/findings";
pub const ENVS_DIR: &str = "kb/envs";
pub const MODERATION_FILE: &str = "kb/moderation.toml";
pub const POLICY_FILE: &str = ".orc/policy.toml";
pub const PROJECT_FILE: &str = ".orc/project.toml";

/// A problem found while loading, tied to a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Kb {
    pub policy: Policy,
    pub moderation: Moderation,
    /// id -> (path, finding)
    pub findings: BTreeMap<String, (String, Finding)>,
    /// env id -> raw JSON
    pub envs: BTreeMap<String, String>,
    /// scope -> (path, game)
    pub games: BTreeMap<String, (String, Game)>,
    pub errors: Vec<LoadError>,
}

impl Kb {
    pub fn load(src: &dyn FileSource) -> Self {
        let mut kb = Kb::default();
        if let Some(text) = src.read(POLICY_FILE) {
            match toml::from_str(&text) {
                Ok(p) => kb.policy = p,
                Err(e) => kb.err(POLICY_FILE, format!("invalid policy: {e}")),
            }
        }
        if let Some(text) = src.read(MODERATION_FILE) {
            match toml::from_str(&text) {
                Ok(m) => kb.moderation = m,
                Err(e) => kb.err(MODERATION_FILE, format!("invalid moderation file: {e}")),
            }
        }
        for path in src.list(ENVS_DIR) {
            if is_placeholder(&path) {
                continue;
            }
            let Some(name) = path.strip_prefix(&format!("{ENVS_DIR}/")) else { continue };
            let Some(id) = name.strip_suffix(".json") else {
                kb.err(&path, "only .json files belong in kb/envs".into());
                continue;
            };
            if let Some(text) = src.read(&path) {
                kb.envs.insert(id.to_string(), text);
            }
        }
        for path in src.list(GAMES_DIR) {
            if is_placeholder(&path) {
                continue;
            }
            if !path.ends_with(".toml") {
                kb.err(&path, "only .toml files belong in kb/games (covers go in kb/games/covers)".into());
                continue;
            }
            let Some(text) = src.read(&path) else { continue };
            match toml::from_str::<Game>(&text) {
                Ok(g) => {
                    let scope = g.scope();
                    if let Some((other, _)) = kb.games.get(&scope) {
                        let msg = format!("duplicate game {scope} (also in {other})");
                        kb.err(&path, msg);
                    } else {
                        kb.games.insert(scope, (path, g));
                    }
                }
                Err(e) => kb.err(&path, format!("invalid game: {e}")),
            }
        }
        for path in src.list(FINDINGS_DIR) {
            if is_placeholder(&path) {
                continue;
            }
            if !path.ends_with(".md") {
                kb.err(&path, "only .md files belong in kb/findings".into());
                continue;
            }
            let Some(text) = src.read(&path) else { continue };
            match Finding::parse(&text) {
                Ok(f) => {
                    if let Some((other, _)) = kb.findings.get(&f.id) {
                        let msg = format!("duplicate id {} (also in {other})", f.id);
                        kb.err(&path, msg);
                    } else {
                        kb.findings.insert(f.id.clone(), (path, f));
                    }
                }
                Err(e) => kb.err(&path, e.0),
            }
        }
        kb
    }

    fn err(&mut self, path: &str, message: String) {
        self.errors.push(LoadError { path: path.to_string(), message });
    }

    pub fn get(&self, id: &str) -> Option<&Finding> {
        self.findings.get(id).map(|(_, f)| f)
    }

    /// Hidden by moderation (the finding or its author is quarantined).
    pub fn is_quarantined(&self, f: &Finding) -> bool {
        self.moderation.quarantined_findings.iter().any(|q| q == &f.id)
            || self.moderation.quarantined_users.iter().any(|u| u.eq_ignore_ascii_case(&f.author.github))
    }

    /// Findings that (transitively) depend on a disputed or retracted one.
    pub fn suspects(&self) -> BTreeSet<String> {
        let bad = |f: &Finding| matches!(f.status, Status::Disputed | Status::Retracted) || self.is_quarantined(f);
        let mut suspect: BTreeSet<String> = BTreeSet::new();
        loop {
            let before = suspect.len();
            for (id, (_, f)) in &self.findings {
                if suspect.contains(id) {
                    continue;
                }
                let tainted = f.depends_on.iter().any(|d| suspect.contains(d) || self.get(d).is_some_and(bad));
                if tainted {
                    suspect.insert(id.clone());
                }
            }
            if suspect.len() == before {
                return suspect;
            }
        }
    }

    /// Visible findings (not quarantined).
    pub fn visible(&self) -> impl Iterator<Item = &Finding> {
        self.findings.values().map(|(_, f)| f).filter(|f| !self.is_quarantined(f))
    }
}

/// Files that keep a directory in git or explain it.
fn is_placeholder(path: &str) -> bool {
    path.ends_with("/README.md") || path.ends_with("/.gitkeep")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::tests::sample;
    use crate::source::MemSource;

    #[test]
    fn load_and_suspects() {
        let mut a = sample();
        a.id = "F-000000000a".into();
        a.status = Status::Retracted;
        a.retracted_reason = Some("wrong".into());
        let mut b = sample();
        b.id = "F-000000000b".into();
        b.depends_on = vec![a.id.clone()];
        let mut c = sample();
        c.id = "F-000000000c".into();
        c.depends_on = vec![b.id.clone()];
        let mut d = sample();
        d.id = "F-000000000d".into();

        let mut src = MemSource::default();
        for f in [&a, &b, &c, &d] {
            src.put(&format!("{FINDINGS_DIR}/{}.md", f.id), &f.render());
        }
        src.put(&format!("{FINDINGS_DIR}/junk.txt"), "x");
        let kb = Kb::load(&src);
        assert_eq!(kb.findings.len(), 4);
        assert_eq!(kb.errors.len(), 1);
        let s = kb.suspects();
        assert!(s.contains("F-000000000b") && s.contains("F-000000000c"));
        assert!(!s.contains("F-000000000d") && !s.contains("F-000000000a"));
    }

    #[test]
    fn quarantine_hides_user() {
        let mut src = MemSource::default();
        let f = sample();
        src.put(&format!("{FINDINGS_DIR}/{}.md", f.id), &f.render());
        src.put(MODERATION_FILE, "quarantined_users = [\"ALICE\"]\n");
        let kb = Kb::load(&src);
        assert_eq!(kb.visible().count(), 0);
    }
}

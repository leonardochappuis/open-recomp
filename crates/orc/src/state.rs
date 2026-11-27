//! Per-machine state: which attempts this machine is working on, so the Stop
//! hook can renew their leases. A machine with no running agent runs no
//! hooks, so its leases run out on their own.

use crate::project::state_dir;
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Leases are renewed at most this often.
pub const RENEW_EVERY: Duration = Duration::hours(1);

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LocalState {
    /// "owner/repo#123" -> tracking info
    #[serde(default)]
    pub attempts: BTreeMap<String, Tracked>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tracked {
    pub last_renew: DateTime<Utc>,
}

fn key(repo: &str, n: u64) -> String {
    format!("{repo}#{n}")
}

impl LocalState {
    fn path() -> std::path::PathBuf {
        state_dir().join("state.json")
    }

    pub fn load() -> LocalState {
        std::fs::read_to_string(Self::path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
    }

    pub fn update(f: impl FnOnce(&mut LocalState)) -> Result<()> {
        let mut s = Self::load();
        f(&mut s);
        let p = Self::path();
        std::fs::create_dir_all(p.parent().expect("has parent"))?;
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&s)?)?;
        std::fs::rename(tmp, p)?;
        Ok(())
    }

    pub fn track(&mut self, repo: &str, n: u64) {
        self.attempts.insert(key(repo, n), Tracked { last_renew: Utc::now() });
    }

    /// Activity on an attempt (a note, a finding) means this machine works on
    /// it: start tracking it (due for renewal at the next Stop hook).
    pub fn touch(&mut self, repo: &str, n: u64) {
        self.attempts.entry(key(repo, n)).or_insert(Tracked { last_renew: Utc::now() - RENEW_EVERY });
    }

    pub fn untrack(&mut self, repo: &str, n: u64) {
        self.attempts.remove(&key(repo, n));
    }

    pub fn due(&self, repo: &str, now: DateTime<Utc>) -> Vec<u64> {
        let prefix = format!("{repo}#");
        self.attempts
            .iter()
            .filter(|(_, t)| now - t.last_renew >= RENEW_EVERY)
            .filter_map(|(k, _)| k.strip_prefix(&prefix)?.parse().ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn due_after_an_hour() {
        let mut s = LocalState::default();
        s.track("o/r", 1);
        s.track("o/other", 2);
        let now = Utc::now();
        assert!(s.due("o/r", now).is_empty());
        assert_eq!(s.due("o/r", now + Duration::minutes(61)), vec![1]);
        s.untrack("o/r", 1);
        assert!(s.due("o/r", now + Duration::days(1)).is_empty());
    }
}

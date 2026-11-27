//! Project, policy and moderation files.

use serde::{Deserialize, Serialize};

/// `.orc/project.toml` in a project's code repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    /// GitHub `owner/name` of the knowledge repo (issues live there too),
    /// or `"self"` when the code repo is also the knowledge repo.
    pub kb: String,
    /// Scope used when the agent doesn't give one.
    pub default_scope: String,
    /// GitHub `owner/name` of this code repo. Needed when `kb = "self"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
}

impl Project {
    /// `owner/name` of the repo holding findings and attempt issues.
    pub fn kb_repo(&self) -> Option<&str> {
        if self.kb == "self" {
            self.repo.as_deref()
        } else {
            Some(&self.kb)
        }
    }
}

/// `.orc/policy.toml` in the knowledge repo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    /// GitHub logins allowed to verify, moderate and close others' attempts.
    pub coordinators: Vec<String>,
    pub findings: FindingPolicy,
    pub attempts: AttemptPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FindingPolicy {
    /// Distinct non-author reproducers needed before `verified`.
    pub min_reproducers: usize,
    pub max_body_chars: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AttemptPolicy {
    pub lease_default_hours: u32,
    pub lease_min_hours: u32,
    pub lease_max_hours: u32,
    pub paused_lease_hours: u32,
    /// Fraction of the lease after which the owner is reminded once.
    pub remind_at: f64,
    /// Days an expired attempt stays open before it is closed.
    pub grace_days: u32,
}

impl Default for FindingPolicy {
    fn default() -> Self {
        Self { min_reproducers: 1, max_body_chars: 8000 }
    }
}

impl Default for AttemptPolicy {
    fn default() -> Self {
        Self {
            lease_default_hours: 72,
            lease_min_hours: 1,
            lease_max_hours: 14 * 24,
            paused_lease_hours: 14 * 24,
            remind_at: 0.75,
            grace_days: 7,
        }
    }
}

impl Policy {
    pub fn is_coordinator(&self, login: &str) -> bool {
        self.coordinators.iter().any(|c| c.eq_ignore_ascii_case(login))
    }

    /// Clamp a requested lease to the policy range.
    pub fn lease_hours(&self, requested: Option<u32>) -> u32 {
        let a = &self.attempts;
        requested.unwrap_or(a.lease_default_hours).clamp(a.lease_min_hours, a.lease_max_hours)
    }
}

/// `kb/moderation.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Moderation {
    pub quarantined_users: Vec<String>,
    pub quarantined_findings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_policy_uses_defaults() {
        let p: Policy = toml::from_str("coordinators = [\"Lead\"]\n[attempts]\ngrace_days = 3\n").unwrap();
        assert!(p.is_coordinator("lead"));
        assert_eq!(p.attempts.grace_days, 3);
        assert_eq!(p.attempts.lease_default_hours, 72);
        assert_eq!(p.findings.min_reproducers, 1);
        assert_eq!(p.lease_hours(Some(10_000)), 336);
        assert_eq!(p.lease_hours(Some(0)), 1);
        assert_eq!(p.lease_hours(None), 72);
    }

    #[test]
    fn project_kb_repo() {
        let p = Project { name: "x".into(), kb: "self".into(), default_scope: "a".into(), repo: Some("o/r".into()) };
        assert_eq!(p.kb_repo(), Some("o/r"));
        let p = Project { kb: "o/kb".into(), ..p };
        assert_eq!(p.kb_repo(), Some("o/kb"));
    }
}

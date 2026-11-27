//! A finding: one Markdown file with TOML front matter (`+++ ... +++`).

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub scope: String,
    pub subject: String,
    pub kind: Kind,
    pub claim: String,
    pub status: Status,
    /// RFC 3339.
    pub created: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Issue number of the attempt this came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<u64>,
    /// Fingerprint of a file in `kb/envs/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retracted_reason: Option<String>,
    pub author: Author,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reproductions: Vec<Reproduction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disputes: Vec<Dispute>,
    /// Markdown after the front matter.
    #[serde(skip)]
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Author {
    pub github: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    #[serde(rename = "type")]
    pub kind: EvidenceKind,
    #[serde(rename = "ref")]
    pub reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reproduction {
    pub github: String,
    pub result: ReproResult,
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dispute {
    pub github: String,
    pub reason: String,
    pub date: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Behavior,
    Structure,
    BugCause,
    Fix,
    FailedApproach,
    Perf,
    Howto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Proposed,
    Reproduced,
    Verified,
    Disputed,
    Retracted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceKind {
    Commit,
    Test,
    Trace,
    Issue,
    Pr,
    Doc,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReproResult {
    Reproduced,
    NotReproduced,
}

macro_rules! display_via_serde {
    ($($t:ty),*) => {$(
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let v = serde_json::to_value(self).map_err(|_| fmt::Error)?;
                f.write_str(v.as_str().unwrap_or_default())
            }
        }
    )*};
}
display_via_serde!(Kind, Status, EvidenceKind, ReproResult);

impl std::str::FromStr for Kind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        serde_json::from_value(serde_json::Value::String(s.into())).map_err(|_| {
            format!("unknown kind {s:?} (behavior, structure, bug-cause, fix, failed-approach, perf, howto)")
        })
    }
}

impl std::str::FromStr for EvidenceKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        serde_json::from_value(serde_json::Value::String(s.into()))
            .map_err(|_| format!("unknown evidence type {s:?} (commit, test, trace, issue, pr, doc, other)"))
    }
}

#[derive(Debug)]
pub struct ParseError(pub String);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

impl Finding {
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text).replace("\r\n", "\n");
        let rest =
            text.strip_prefix("+++\n").ok_or_else(|| ParseError("finding must start with a `+++` line".into()))?;
        let (front, body) = match rest.find("\n+++") {
            Some(i) => {
                let after = &rest[i + 4..];
                let body = after.strip_prefix('\n').unwrap_or(after);
                (&rest[..i + 1], body)
            }
            None => return Err(ParseError("missing closing `+++` line".into())),
        };
        let mut f: Finding = toml::from_str(front).map_err(|e| ParseError(format!("front matter: {e}")))?;
        f.body = body.trim().to_string();
        Ok(f)
    }

    pub fn render(&self) -> String {
        let front = toml::to_string(self).expect("Finding always serializes to TOML");
        let mut out = format!("+++\n{front}+++\n");
        if !self.body.is_empty() {
            out.push('\n');
            out.push_str(self.body.trim());
            out.push('\n');
        }
        out
    }

    /// Distinct GitHub accounts, other than the author, that reproduced it.
    pub fn independent_reproducers(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .reproductions
            .iter()
            .filter(|r| r.result == ReproResult::Reproduced)
            .map(|r| r.github.as_str())
            .filter(|g| !g.eq_ignore_ascii_case(&self.author.github))
            .collect();
        v.sort_unstable_by_key(|s| s.to_ascii_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        v
    }

    /// One line for listings: `F-xxxx [verified] scope: subject - claim`.
    pub fn one_line(&self) -> String {
        format!("{} [{}] {}: {} - {}", self.id, self.status, self.scope, self.subject, self.claim)
    }
}

/// `F-` + 10 lowercase hex digits, derived from author, time and claim.
pub fn new_id(author: &str, created: &str, claim: &str) -> String {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let h = fnv1a(format!("{author}\0{created}\0{claim}\0{nanos}").as_bytes());
    format!("F-{:010x}", h & 0xff_ffff_ffff)
}

pub fn is_valid_id(id: &str) -> bool {
    id.len() == 12 && id.starts_with("F-") && id[2..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn sample() -> Finding {
        Finding {
            id: "F-00000000aa".into(),
            scope: "platform/x360/game/4d5307e6/render".into(),
            subject: "texture untiling, 32bpp".into(),
            kind: Kind::BugCause,
            claim: "Face corruption comes from wrong tile pitch when width < 128".into(),
            status: Status::Proposed,
            created: "2026-11-26T15:04:05Z".into(),
            depends_on: vec![],
            attempt: Some(42),
            env: None,
            retracted_reason: None,
            author: Author { github: "alice".into(), agent: Some("claude-code".into()), model: None },
            evidence: vec![Evidence { kind: EvidenceKind::Commit, reference: "xenonrecomp@abc123".into() }],
            reproductions: vec![],
            disputes: vec![],
            body: "Details here.\n\nMore.".into(),
        }
    }

    #[test]
    fn roundtrip() {
        let f = sample();
        let text = f.render();
        assert!(text.starts_with("+++\nid = \"F-00000000aa\""));
        assert_eq!(Finding::parse(&text).unwrap(), f);
    }

    #[test]
    fn crlf_and_missing_body() {
        let mut f = sample();
        f.body.clear();
        let text = f.render().replace('\n', "\r\n");
        assert_eq!(Finding::parse(&text).unwrap(), f);
    }

    #[test]
    fn rejects_unknown_fields_and_bad_framing() {
        let text = sample().render().replace("kind = ", "evil = 1\nkind = ");
        assert!(Finding::parse(&text).is_err());
        assert!(Finding::parse("id = 1").is_err());
        assert!(Finding::parse("+++\nid = \"x\"\n").is_err());
    }

    #[test]
    fn independent_reproducers_excludes_author_and_dupes() {
        let mut f = sample();
        for (who, res) in [
            ("alice", ReproResult::Reproduced),
            ("Bob", ReproResult::Reproduced),
            ("bob", ReproResult::Reproduced),
            ("carol", ReproResult::NotReproduced),
        ] {
            f.reproductions.push(Reproduction {
                github: who.into(),
                result: res,
                date: "2026-11-27T00:00:00Z".into(),
                env: None,
                notes: None,
                evidence: vec![],
            });
        }
        assert_eq!(f.independent_reproducers(), vec!["Bob"]);
    }

    #[test]
    fn ids() {
        let id = new_id("alice", "2026", "claim");
        assert!(is_valid_id(&id), "{id}");
        assert!(!is_valid_id("F-XYZ"));
        assert!(!is_valid_id("F-00000000AA"));
    }
}

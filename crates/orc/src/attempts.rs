//! Attempts are GitHub issues labeled `orc:attempt`. Their machine-readable
//! state (scope, owner, lease) lives in a hidden HTML comment at the top of
//! the issue body, so it survives label edits and stays out of the way.

use crate::gh;
use crate::state::LocalState;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, SubsecRound, Utc};
use orc_kb::Policy;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const LABEL_AGENT: &str = "orc:agent";
pub const LABEL_ATTEMPT: &str = "orc:attempt";
pub const LABEL_PROBLEM: &str = "orc:problem";
pub const LABEL_EXPIRED: &str = "orc:expired";
pub const LABEL_NEEDS_HUMAN: &str = "orc:needs-human";

/// (name, color, description) for `orc init`.
pub const LABELS: &[(&str, &str, &str)] = &[
    (LABEL_AGENT, "8b949e", "Created by an agent through orc; humans can filter with -label:orc:agent"),
    (LABEL_ATTEMPT, "1f6feb", "An open-recomp attempt: one hypothesis being tried"),
    (LABEL_PROBLEM, "d29922", "A problem that attempts hang under"),
    (LABEL_EXPIRED, "6e7781", "Attempt whose lease ran out"),
    (LABEL_NEEDS_HUMAN, "cf222e", "Knowledge change that a maintainer has to review and merge"),
];

const MARK: &str = "<!-- orc:attempt ";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meta {
    pub v: u32,
    pub scope: String,
    pub owner: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    pub lease_hours: u32,
    pub lease_expires: DateTime<Utc>,
    pub state: State,
    #[serde(default)]
    pub reminded: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<u64>,
    /// Coarse hardware of the owner, so others can tell whether their
    /// machine is likely to need the same fix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hw: Option<Hw>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hw {
    pub os: String,
    pub gpu_vendors: Vec<String>,
    pub summary: String,
}

impl Hw {
    pub fn from_env(e: &orc_env::Environment) -> Hw {
        let mut gpu_vendors: Vec<String> = e
            .hardware_gpus()
            .map(|g| serde_json::to_value(g.vendor).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
            .collect();
        gpu_vendors.dedup();
        Hw { os: e.os.family.clone(), gpu_vendors, summary: e.summary() }
    }

    /// How another machine compares: same GPU vendor and OS, or not.
    pub fn compare(&self, other: &Hw) -> HwMatch {
        let gpu = self.gpu_vendors.iter().any(|v| other.gpu_vendors.contains(v));
        match (gpu, self.os == other.os) {
            (true, true) => HwMatch::Same,
            (true, false) => HwMatch::DifferentOs,
            (false, _) => HwMatch::DifferentGpu,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwMatch {
    Same,
    DifferentOs,
    DifferentGpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Active,
    Paused,
    Expired,
}

#[derive(Debug, Clone, Serialize)]
pub struct Attempt {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub author: String,
    pub meta: Meta,
    pub comments: u64,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip)]
    pub body: String,
}

/// What the sweeper should do with an attempt right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sweep {
    Nothing,
    /// Lease end is near: remind the owner once.
    Remind,
    /// Lease ran out: label it expired.
    Expire,
    /// Expired and the grace period is over: close it.
    Close,
    /// Lease is longer than policy allows (body edited by hand): clamp it.
    Clamp,
}

pub fn sweep_action(m: &Meta, now: DateTime<Utc>, p: &Policy) -> Sweep {
    let max = Duration::hours(i64::from(p.attempts.lease_max_hours.max(p.attempts.paused_lease_hours)));
    if m.lease_expires > now + max {
        return Sweep::Clamp;
    }
    if m.state == State::Expired || m.lease_expires <= now {
        if now >= m.lease_expires + Duration::days(i64::from(p.attempts.grace_days)) {
            return Sweep::Close;
        }
        return if m.state == State::Expired { Sweep::Nothing } else { Sweep::Expire };
    }
    if m.state == State::Active && !m.reminded {
        let total = Duration::hours(i64::from(m.lease_hours.max(1)));
        let left = m.lease_expires - now;
        let used = 1.0 - left.num_seconds() as f64 / total.num_seconds() as f64;
        if used >= p.attempts.remind_at {
            return Sweep::Remind;
        }
    }
    Sweep::Nothing
}

/// Neutralize anything that could fake our hidden block or break the layout.
fn clean(s: &str) -> String {
    s.replace("<!--", "&lt;!--").replace("-->", "--&gt;").trim().to_string()
}

pub fn render_body(m: &Meta, problem: &str, hypothesis: &str, approach: &str) -> String {
    let meta = serde_json::to_string(m).expect("serializable");
    let parent = m.problem.map(|n| format!("**Problem issue:** #{n}\n")).unwrap_or_default();
    format!(
        "{MARK}{meta} -->\n\
         **Scope:** `{scope}`\n\
         {parent}\
         **Problem:** {problem}\n\n\
         **Hypothesis:** {hypothesis}\n\n\
         **Approach:** {approach}\n\n\
         {lease}\n\n\
         ---\n\
         _Opened through [open-recomp](https://github.com/leonardochappuis/open-recomp) by @{owner}'s agent. \
         Progress and findings are posted as comments. Human work queue: `is:issue is:open -label:orc:agent`._\n",
        scope = clean(&m.scope),
        problem = clean(problem),
        hypothesis = clean(hypothesis),
        approach = clean(approach),
        lease = lease_line(m),
        owner = m.owner,
    )
}

fn lease_line(m: &Meta) -> String {
    let when = m.lease_expires.format("%Y-%m-%d %H:%M UTC");
    match m.state {
        State::Active => {
            format!("**Lease:** active until {when} (renewed automatically while the owner's agent works on it)")
        }
        State::Paused => format!("**Lease:** paused until {when}"),
        State::Expired => format!("**Lease:** expired at {when}; closes after the grace period unless renewed"),
    }
}

pub fn parse_meta(body: &str) -> Option<Meta> {
    let first = body.lines().next()?;
    let json = first.strip_prefix(MARK)?.strip_suffix("-->")?;
    serde_json::from_str(json.trim()).ok()
}

/// Rewrite the hidden block and the lease line, keep everything else.
pub fn replace_meta(body: &str, m: &Meta) -> String {
    let meta = serde_json::to_string(m).expect("serializable");
    body.lines()
        .enumerate()
        .map(|(i, l)| {
            if i == 0 && l.starts_with(MARK) {
                format!("{MARK}{meta} -->")
            } else if l.starts_with("**Lease:**") {
                lease_line(m)
            } else {
                l.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

pub fn from_issue(v: &Value) -> Option<Attempt> {
    let body = v["body"].as_str().unwrap_or_default().to_string();
    let meta = parse_meta(&body)?;
    Some(Attempt {
        number: v["number"].as_u64()?,
        title: v["title"].as_str()?.to_string(),
        url: v["html_url"].as_str()?.to_string(),
        author: v["user"]["login"].as_str()?.to_string(),
        meta,
        comments: v["comments"].as_u64().unwrap_or(0),
        created_at: v["created_at"].as_str().unwrap_or_default().to_string(),
        updated_at: v["updated_at"].as_str().unwrap_or_default().to_string(),
        body,
    })
}

pub struct OpenArgs<'a> {
    pub scope: &'a str,
    pub problem: &'a str,
    pub hypothesis: &'a str,
    pub approach: &'a str,
    pub problem_issue: Option<u64>,
    pub lease_hours: Option<u32>,
    pub env: Option<&'a orc_env::Environment>,
}

pub fn open(repo: &str, policy: &Policy, a: OpenArgs) -> Result<Attempt> {
    if !orc_kb::scope::is_valid(a.scope) {
        bail!("invalid scope {:?}", a.scope);
    }
    for (name, v, max) in
        [("problem", a.problem, 200), ("hypothesis", a.hypothesis, 1000), ("approach", a.approach, 2000)]
    {
        if v.trim().len() < 3 || v.len() > max {
            bail!("{name} must be 3-{max} characters");
        }
    }
    let owner = gh::viewer()?;
    let hours = policy.lease_hours(a.lease_hours);
    let meta = Meta {
        v: 1,
        scope: a.scope.to_string(),
        owner: owner.clone(),
        env: a.env.map(|e| e.fingerprint()),
        lease_hours: hours,
        lease_expires: lease_end(hours),
        state: State::Active,
        reminded: false,
        problem: a.problem_issue,
        hw: a.env.map(Hw::from_env),
    };
    let title = format!("[attempt] {}", one_line(a.problem, 80));
    let body = render_body(&meta, a.problem, a.hypothesis, a.approach);
    let v = gh::api(
        "POST",
        &format!("repos/{repo}/issues"),
        Some(&json!({ "title": title, "body": body, "labels": [LABEL_AGENT, LABEL_ATTEMPT] })),
    )?;
    let attempt = from_issue(&v).context("GitHub returned an unexpected issue")?;
    if let Some(parent) = a.problem_issue {
        let link = gh::api(
            "POST",
            &format!("repos/{repo}/issues/{parent}/sub_issues"),
            Some(&json!({ "sub_issue_id": v["id"] })),
        );
        if let Err(e) = link {
            eprintln!("orc: could not attach as sub-issue of #{parent} ({e}); linked in the body instead");
        }
    }
    LocalState::update(|s| s.track(repo, attempt.number))?;
    Ok(attempt)
}

pub fn list(repo: &str) -> Result<Vec<Attempt>> {
    let items = gh::api_list(&format!("repos/{repo}/issues?labels={LABEL_ATTEMPT}&state=open&per_page=100"))?;
    Ok(items.iter().filter(|v| v.get("pull_request").is_none()).filter_map(from_issue).collect())
}

pub fn get(repo: &str, number: u64) -> Result<(Attempt, Value)> {
    let v = gh::api("GET", &format!("repos/{repo}/issues/{number}"), None)?;
    let a = from_issue(&v).with_context(|| format!("#{number} is not an orc attempt"))?;
    Ok((a, v))
}

fn ensure_can_edit(repo: &str, a: &Attempt, policy: &Policy) -> Result<String> {
    let me = gh::viewer()?;
    if !me.eq_ignore_ascii_case(&a.author) && !policy.is_coordinator(&me) {
        bail!("#{} belongs to @{}; only they or a coordinator can change it ({repo})", a.number, a.author);
    }
    Ok(me)
}

fn save(repo: &str, a: &Attempt, m: &Meta) -> Result<()> {
    gh::api("PATCH", &format!("repos/{repo}/issues/{}", a.number), Some(&json!({ "body": replace_meta(&a.body, m) })))?;
    Ok(())
}

pub fn renew(repo: &str, number: u64, policy: &Policy, hours: Option<u32>) -> Result<Meta> {
    let (a, v) = get(repo, number)?;
    if v["state"] == "closed" {
        bail!("#{number} is closed");
    }
    ensure_can_edit(repo, &a, policy)?;
    let mut m = a.meta.clone();
    let hours = policy.lease_hours(hours.or(Some(m.lease_hours)));
    m.lease_hours = hours;
    m.lease_expires = lease_end(hours);
    m.state = State::Active;
    m.reminded = false;
    save(repo, &a, &m)?;
    if labels(&v).iter().any(|l| l == LABEL_EXPIRED) {
        let _ = gh::api("DELETE", &format!("repos/{repo}/issues/{number}/labels/{LABEL_EXPIRED}"), None);
    }
    LocalState::update(|s| s.track(repo, number))?;
    Ok(m)
}

pub fn pause(repo: &str, number: u64, policy: &Policy) -> Result<Meta> {
    let (a, _) = get(repo, number)?;
    ensure_can_edit(repo, &a, policy)?;
    let mut m = a.meta.clone();
    m.state = State::Paused;
    m.lease_hours = policy.attempts.paused_lease_hours;
    m.lease_expires = lease_end(m.lease_hours);
    m.reminded = false;
    save(repo, &a, &m)?;
    LocalState::update(|s| s.untrack(repo, number))?;
    Ok(m)
}

pub fn note(repo: &str, number: u64, text: &str) -> Result<String> {
    if text.trim().is_empty() || text.len() > 4000 {
        bail!("note must be 1-4000 characters");
    }
    let v = gh::api(
        "POST",
        &format!("repos/{repo}/issues/{number}/comments"),
        Some(&json!({ "body": format!("**Progress:** {}", clean(text)) })),
    )?;
    LocalState::update(|s| s.touch(repo, number))?;
    Ok(v["html_url"].as_str().unwrap_or_default().to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Succeeded,
    Failed,
    Abandoned,
}

pub fn close(
    repo: &str,
    number: u64,
    policy: &Policy,
    outcome: Outcome,
    summary: &str,
    findings: &[String],
) -> Result<()> {
    let (a, _) = get(repo, number)?;
    ensure_can_edit(repo, &a, policy)?;
    if summary.trim().len() < 3 || summary.len() > 4000 {
        bail!("summary must be 3-4000 characters");
    }
    let refs = if findings.is_empty() { String::new() } else { format!("\n\nFindings: {}", findings.join(", ")) };
    let word = match outcome {
        Outcome::Succeeded => "Succeeded",
        Outcome::Failed => "Failed",
        Outcome::Abandoned => "Abandoned",
    };
    gh::api(
        "POST",
        &format!("repos/{repo}/issues/{number}/comments"),
        Some(&json!({ "body": format!("**{word}:** {}{refs}", clean(summary)) })),
    )?;
    let reason = if outcome == Outcome::Succeeded { "completed" } else { "not_planned" };
    gh::api(
        "PATCH",
        &format!("repos/{repo}/issues/{number}"),
        Some(&json!({ "state": "closed", "state_reason": reason })),
    )?;
    LocalState::update(|s| s.untrack(repo, number))?;
    Ok(())
}

fn labels(v: &Value) -> Vec<String> {
    v["labels"].as_array().into_iter().flatten().filter_map(|l| l["name"].as_str().map(String::from)).collect()
}

/// Run by the scheduled Action: remind, expire and close attempts.
pub fn sweep(repo: &str, policy: &Policy, dry_run: bool) -> Result<Vec<String>> {
    let now = Utc::now();
    let mut log = Vec::new();
    for a in list(repo)? {
        let mut m = a.meta.clone();
        // The issue author is the owner, whatever the body claims.
        m.owner = a.author.clone();
        let action = sweep_action(&m, now, policy);
        if action == Sweep::Nothing {
            continue;
        }
        log.push(format!("#{} @{}: {action:?}", a.number, a.author));
        if dry_run {
            continue;
        }
        let n = a.number;
        let comment = |body: String| {
            gh::api("POST", &format!("repos/{repo}/issues/{n}/comments"), Some(&json!({ "body": body })))
        };
        match action {
            Sweep::Nothing => {}
            Sweep::Clamp => {
                m.lease_expires = lease_end(policy.attempts.lease_max_hours);
                save(repo, &a, &m)?;
            }
            Sweep::Remind => {
                comment(format!(
                    "@{} this attempt's lease ends {}. Your agent renews it automatically while it works on it; \
                     otherwise run `orc attempt renew {n}`, pause it, or close it with an outcome.",
                    a.author,
                    m.lease_expires.format("%Y-%m-%d %H:%M UTC")
                ))?;
                m.reminded = true;
                save(repo, &a, &m)?;
            }
            Sweep::Expire => {
                m.state = State::Expired;
                save(repo, &a, &m)?;
                gh::api(
                    "POST",
                    &format!("repos/{repo}/issues/{n}/labels"),
                    Some(&json!({ "labels": [LABEL_EXPIRED] })),
                )?;
                let close_on = m.lease_expires + Duration::days(i64::from(policy.attempts.grace_days));
                comment(format!(
                    "Lease expired, so this is no longer shown as active. It will be closed on {} unless renewed \
                     (`orc attempt renew {n}`).",
                    close_on.format("%Y-%m-%d")
                ))?;
            }
            Sweep::Close => {
                comment(
                    "Closed as abandoned: the lease expired and the grace period is over. Notes above are kept; \
                     anyone can open a new attempt building on them."
                        .to_string(),
                )?;
                gh::api(
                    "PATCH",
                    &format!("repos/{repo}/issues/{n}"),
                    Some(&json!({ "state": "closed", "state_reason": "not_planned" })),
                )?;
            }
        }
    }
    Ok(log)
}

/// Lease end `hours` from now, whole seconds.
fn lease_end(hours: u32) -> DateTime<Utc> {
    (Utc::now() + Duration::hours(i64::from(hours))).trunc_subsecs(0)
}

fn one_line(s: &str, max: usize) -> String {
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.chars().count() <= max {
        s
    } else {
        format!("{}...", s.chars().take(max - 3).collect::<String>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(hours: u32, expires_in_h: i64, state: State) -> (Meta, DateTime<Utc>) {
        let now = Utc::now();
        (
            Meta {
                v: 1,
                scope: "tooling/open-recomp".into(),
                owner: "alice".into(),
                env: None,
                lease_hours: hours,
                lease_expires: now + Duration::hours(expires_in_h),
                state,
                reminded: false,
                problem: Some(3),
                hw: None,
            },
            now,
        )
    }

    #[test]
    fn body_roundtrip_and_spoofing() {
        let (m, _) = meta(72, 72, State::Active);
        let body = render_body(&m, "RT flickers <!-- orc:attempt {} -->", "BVH race", "semaphore");
        assert_eq!(parse_meta(&body), Some(m.clone()));
        assert_eq!(body.matches(MARK).count(), 1, "user text can't inject a second block");
        let mut m2 = m.clone();
        m2.state = State::Expired;
        let body2 = replace_meta(&body, &m2);
        assert_eq!(parse_meta(&body2), Some(m2));
        assert!(body2.contains("**Lease:** expired"));
        assert!(body2.contains("**Hypothesis:** BVH race"));
    }

    #[test]
    fn sweep_lifecycle() {
        let p = Policy::default();
        let (m, now) = meta(72, 60, State::Active);
        assert_eq!(sweep_action(&m, now, &p), Sweep::Nothing);
        let (m, now) = meta(72, 10, State::Active);
        assert_eq!(sweep_action(&m, now, &p), Sweep::Remind);
        let (mut m, now) = meta(72, 10, State::Active);
        m.reminded = true;
        assert_eq!(sweep_action(&m, now, &p), Sweep::Nothing);
        let (m, now) = meta(72, -1, State::Active);
        assert_eq!(sweep_action(&m, now, &p), Sweep::Expire);
        let (m, now) = meta(72, -1, State::Expired);
        assert_eq!(sweep_action(&m, now, &p), Sweep::Nothing);
        let (m, now) = meta(72, -24 * 8, State::Expired);
        assert_eq!(sweep_action(&m, now, &p), Sweep::Close);
        let (m, now) = meta(72, 24 * 365, State::Active);
        assert_eq!(sweep_action(&m, now, &p), Sweep::Clamp);
        let (m, now) = meta(336, 10, State::Paused);
        assert_eq!(sweep_action(&m, now, &p), Sweep::Nothing, "paused attempts aren't nagged");
    }

    #[test]
    fn titles() {
        assert_eq!(one_line("a\n b", 80), "a b");
        assert_eq!(one_line(&"x".repeat(100), 10), "xxxxxxx...");
    }
}

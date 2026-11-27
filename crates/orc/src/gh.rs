//! GitHub access through the `gh` CLI, so orc never handles tokens itself:
//! people use their own `gh auth login`, Actions use `GH_TOKEN`.

use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn run(args: &[&str], input: Option<&str>) -> Result<String> {
    let mut cmd = Command::new("gh");
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut child = cmd.spawn().context("could not run `gh` (install the GitHub CLI and run `gh auth login`)")?;
    if let Some(text) = input {
        child.stdin.take().expect("piped").write_all(text.as_bytes())?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("gh {}: {}", args.first().copied().unwrap_or(""), err.trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// `gh api -X METHOD path` with an optional JSON body.
pub fn api(method: &str, path: &str, body: Option<&Value>) -> Result<Value> {
    let input = body.map(|b| b.to_string());
    let mut args = vec!["api", "-X", method, path, "-H", "Accept: application/vnd.github+json"];
    if input.is_some() {
        args.extend(["--input", "-"]);
    }
    let out = run(&args, input.as_deref())?;
    if out.trim().is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&out).with_context(|| format!("unexpected response from {path}"))
}

/// GET every page of a list endpoint, one JSON value per item.
pub fn api_list(path: &str) -> Result<Vec<Value>> {
    let out = run(&["api", "--paginate", path, "--jq", ".[]"], None)?;
    out.lines().filter(|l| !l.trim().is_empty()).map(|l| Ok(serde_json::from_str(l)?)).collect()
}

/// Login of whoever `gh` is authenticated as.
pub fn viewer() -> Result<String> {
    let v = api("GET", "user", None)?;
    v["login"].as_str().map(String::from).context("gh is not logged in (run `gh auth login`)")
}

/// `owner/name` must look like a GitHub repo, so it is safe in URLs.
pub fn check_repo(repo: &str) -> Result<()> {
    let ok = repo.split('/').count() == 2
        && repo.split('/').all(|p| {
            !p.is_empty() && p.len() <= 100 && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        });
    if !ok {
        bail!("{repo:?} is not a GitHub owner/name");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn repo_names() {
        assert!(super::check_repo("leonardochappuis/open-recomp").is_ok());
        assert!(super::check_repo("a/b/c").is_err());
        assert!(super::check_repo("a/b?x=1").is_err());
        assert!(super::check_repo("/b").is_err());
    }
}

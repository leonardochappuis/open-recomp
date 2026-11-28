//! "Add a game" issue-form submissions (label `orc:game-request`). They show
//! under Unfiltered on the dashboard until a maintainer labels them
//! `approved`; an Action then writes the catalog entry.
//!
//! Issue bodies are untrusted: fields are parsed strictly and the resulting
//! game goes through the same validation as any catalog entry.

use orc_kb::{Game, GameStatus};
use std::collections::BTreeMap;

pub const LABEL_REQUEST: &str = "orc:game-request";
pub const LABEL_APPROVED: &str = "approved";

/// `### Label\n\nvalue` sections of a GitHub issue form body.
pub fn parse_form(body: &str) -> BTreeMap<String, String> {
    let body = body.replace("\r\n", "\n");
    let mut out = BTreeMap::new();
    let mut cur: Option<String> = None;
    let mut buf = String::new();
    let flush = |cur: &Option<String>, buf: &mut String, out: &mut BTreeMap<String, String>| {
        if let Some(k) = cur {
            let v = buf.trim();
            if v != "_No response_" && !v.is_empty() {
                out.insert(k.clone(), v.to_string());
            }
        }
        buf.clear();
    };
    for line in body.lines() {
        if let Some(h) = line.strip_prefix("### ") {
            flush(&cur, &mut buf, &mut out);
            cur = Some(h.trim().to_string());
        } else if cur.is_some() {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    flush(&cur, &mut buf, &mut out);
    out
}

/// "Xbox 360 (x360)" -> "x360".
fn slug_in_parens(v: &str) -> Option<String> {
    let open = v.rfind('(')?;
    let close = v[open..].find(')')? + open;
    Some(v[open + 1..close].trim().to_string())
}

/// First https URL in a field: bare, Markdown image/link, or `<img src="...">`.
fn first_https(v: &str) -> Option<String> {
    let start = v.find("https://")?;
    let rest = &v[start..];
    let end =
        rest.find(|c: char| c.is_whitespace() || matches!(c, ')' | '"' | '\'' | '<' | '>' | ']')).unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// "owner/name" or "https://github.com/owner/name(.git)".
fn repo_of(v: &str) -> String {
    let v = v.trim().trim_end_matches('/');
    let v = v.strip_prefix("https://github.com/").unwrap_or(v);
    v.strip_suffix(".git").unwrap_or(v).to_string()
}

pub fn game_from_form(f: &BTreeMap<String, String>) -> Result<Game, Vec<String>> {
    let mut errs = Vec::new();
    let get = |k: &str| f.get(k).map(|s| s.trim().to_string());
    let name = get("Original game name").unwrap_or_default();
    if name.is_empty() {
        errs.push("missing the original game name".to_string());
    }
    let platform = get("Console").and_then(|c| slug_in_parens(&c)).unwrap_or_default();
    if platform.is_empty() {
        errs.push("missing the console".to_string());
    }
    let id = get("Title id or serial").unwrap_or_default();
    if id.is_empty() {
        errs.push("missing the title id or serial".to_string());
    }
    let status = match get("Status").and_then(|s| slug_in_parens(&s)) {
        Some(s) => s.parse::<GameStatus>().unwrap_or_else(|e| {
            errs.push(e);
            GameStatus::Planned
        }),
        None => GameStatus::Planned,
    };
    let targets = get("Runs on")
        .map(|t| {
            t.lines()
                .filter_map(|l| l.trim().strip_prefix("- [X] ").or_else(|| l.trim().strip_prefix("- [x] ")))
                .filter_map(|l| match l.trim() {
                    "Windows" => Some("windows"),
                    "Linux" => Some("linux"),
                    "macOS" => Some("macos"),
                    "Android" => Some("android"),
                    "Browser" => Some("web"),
                    _ => None,
                })
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let cover = get("Cover image").and_then(|c| first_https(&c));
    let year = match get("Year") {
        Some(y) => match y.parse::<u16>() {
            Ok(n) => Some(n),
            Err(_) => {
                errs.push(format!("year {y:?} is not a number"));
                None
            }
        },
        None => None,
    };
    let game = Game {
        name,
        title: get("Recomp project name"),
        platform,
        id,
        status,
        region: None,
        year,
        repos: get("Recomp repository").map(|r| vec![repo_of(&r)]).unwrap_or_default(),
        cover,
        targets,
        tags: vec![],
        description: get("Description"),
    };
    errs.extend(orc_kb::validate::game_errors(&game));
    if errs.is_empty() {
        Ok(game)
    } else {
        Err(errs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BODY: &str = "### Original game name\n\nSky Garden\n\n### Recomp project name\n\nSky Garden HD\n\n### Console\n\nNintendo 64 (n64)\n\n### Title id or serial\n\nNSGE\n\n### Status\n\nPlayable (playable)\n\n### Runs on\n\n- [X] Windows\n- [ ] Linux\n- [X] macOS\n- [ ] Android\n- [ ] Browser\n\n### Recomp repository\n\nhttps://github.com/someone/sky-garden-hd.git\n\n### Cover image\n\n![box](https://github.com/user-attachments/assets/abc-123)\n\n### Year\n\n1998\n\n### Description\n\n_No response_\n\n### Rules\n\n- [X] This contains no ROMs";

    #[test]
    fn parses_issue_form() {
        let g = game_from_form(&parse_form(BODY)).unwrap();
        assert_eq!(g.name, "Sky Garden");
        assert_eq!(g.title.as_deref(), Some("Sky Garden HD"));
        assert_eq!(g.platform, "n64");
        assert_eq!(g.status, GameStatus::Playable);
        assert_eq!(g.targets, vec!["windows", "macos"]);
        assert_eq!(g.repos, vec!["someone/sky-garden-hd"]);
        assert_eq!(g.cover.as_deref(), Some("https://github.com/user-attachments/assets/abc-123"));
        assert_eq!(g.year, Some(1998));
        assert_eq!(g.description, None);
    }

    #[test]
    fn newer_consoles() {
        for (label, slug) in [
            ("Xbox Series X|S (xseries)", "xseries"),
            ("Nintendo Switch 2 (switch2)", "switch2"),
            ("PlayStation 5 (ps5)", "ps5"),
            ("Nintendo 3DS (3ds)", "3ds"),
            ("Other (say which in the description) (other)", "other"),
        ] {
            let body = BODY.replace("Nintendo 64 (n64)", label);
            assert_eq!(game_from_form(&parse_form(&body)).unwrap().platform, slug, "{label}");
        }
    }

    #[test]
    fn rejects_bad_input() {
        let body = BODY.replace("NSGE", "bad id!").replace("1998", "soon").replace("someone/sky-garden-hd.git", "x");
        let errs = game_from_form(&parse_form(&body)).unwrap_err();
        let all = errs.join("\n");
        assert!(all.contains("year") && all.contains("id must be") && all.contains("repos"), "{all}");
        let body = BODY
            .replace("<", "")
            .replace("![box](https://github.com/user-attachments/assets/abc-123)", "javascript:alert(1)");
        assert!(game_from_form(&parse_form(&body)).unwrap().cover.is_none());
    }
}

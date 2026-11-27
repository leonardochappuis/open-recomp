//! Local keyword search. The whole knowledge base is small enough (thousands
//! of findings) to score in memory on every query.

use crate::{scope, Finding, Kb, Status};

#[derive(Debug, Clone, Default)]
pub struct Query {
    pub text: String,
    /// Only findings whose scope is related to this one (ancestor, same, descendant).
    pub scope: Option<String>,
    /// Only these statuses. Empty means everything except retracted.
    pub statuses: Vec<Status>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct Hit<'a> {
    pub finding: &'a Finding,
    pub score: f64,
    /// Depends on a disputed, retracted or quarantined finding.
    pub suspect: bool,
}

pub fn search<'a>(kb: &'a Kb, q: &Query) -> Vec<Hit<'a>> {
    let terms: Vec<String> = tokens(&q.text).into_iter().filter(|t| !STOPWORDS.contains(&t.as_str())).collect();
    let suspects = kb.suspects();
    let mut hits: Vec<Hit> = kb
        .visible()
        .filter(|f| match q.statuses.is_empty() {
            true => f.status != Status::Retracted,
            false => q.statuses.contains(&f.status),
        })
        .filter(|f| q.scope.as_deref().is_none_or(|s| scope::related(&f.scope, s)))
        .filter_map(|f| {
            let score = if terms.is_empty() { 1.0 } else { score(f, &terms) };
            (score > 0.0).then(|| {
                let suspect = suspects.contains(&f.id);
                let boost = match f.status {
                    Status::Verified => 1.5,
                    Status::Reproduced => 1.2,
                    Status::Proposed => 1.0,
                    Status::Disputed => 0.5,
                    Status::Retracted => 0.2,
                } * if suspect { 0.5 } else { 1.0 };
                Hit { finding: f, score: score * boost, suspect }
            })
        })
        .collect();
    hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| b.finding.created.cmp(&a.finding.created)));
    hits.truncate(if q.limit == 0 { 20 } else { q.limit });
    hits
}

fn score(f: &Finding, terms: &[String]) -> f64 {
    let fields: [(f64, Vec<String>); 5] = [
        (3.0, tokens(&f.subject)),
        (2.0, tokens(&f.claim)),
        (2.0, tokens(&f.scope)),
        (1.0, tokens(&f.kind.to_string())),
        (1.0, tokens(&f.body)),
    ];
    let mut total = 0.0;
    let mut matched = 0;
    for t in terms {
        let mut best: f64 = 0.0;
        for (w, toks) in &fields {
            if toks.iter().any(|x| x == t) {
                best = best.max(*w);
            } else if t.len() >= 4 && toks.iter().any(|x| x.starts_with(t.as_str())) {
                best = best.max(w * 0.5);
            }
        }
        if best > 0.0 {
            matched += 1;
        }
        total += best;
    }
    // Reward matching more of the query, not just one term many times.
    total * (matched as f64 / terms.len() as f64)
}

/// Words that carry no meaning in a query ("the faces of the characters").
pub const STOPWORDS: &[&str] = &[
    "a", "an", "the", "of", "on", "in", "at", "to", "for", "with", "and", "or", "is", "are", "was", "were", "be", "it",
    "its", "this", "that", "these", "my", "i", "we", "our", "when", "some", "all", "very", "so", "but", "not", "no",
    "do", "does", "doing", "get", "gets", "got", "have", "has", "there", "they", "them", "oh", "like", "just",
];

/// Lowercase alphanumeric words, plus hex addresses kept whole (`0x8201a4f0`).
pub fn tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty()).map(|w| w.to_lowercase()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finding::tests::sample;
    use crate::source::MemSource;
    use crate::FINDINGS_DIR;

    fn kb() -> Kb {
        let mut src = MemSource::default();
        let mut a = sample();
        a.id = "F-000000000a".into();
        let mut b = sample();
        b.id = "F-000000000b".into();
        b.scope = "platform/ps2".into();
        b.subject = "VU1 microcode upload".into();
        b.claim = "Uploads happen through VIF1 DIRECT".into();
        b.status = Status::Verified;
        b.reproductions.push(crate::Reproduction {
            github: "bob".into(),
            result: crate::ReproResult::Reproduced,
            date: "2026-11-27T00:00:00Z".into(),
            env: None,
            notes: None,
            evidence: vec![],
        });
        let mut c = sample();
        c.id = "F-000000000c".into();
        c.status = Status::Retracted;
        for f in [&a, &b, &c] {
            src.put(&format!("{FINDINGS_DIR}/{}.md", f.id), &f.render());
        }
        Kb::load(&src)
    }

    #[test]
    fn finds_by_subject_and_scope() {
        let kb = kb();
        let hits = search(&kb, &Query { text: "untiling pitch".into(), ..Default::default() });
        assert_eq!(hits.len(), 1, "retracted excluded");
        assert_eq!(hits[0].finding.id, "F-000000000a");

        let hits =
            search(&kb, &Query { text: "microcode".into(), scope: Some("platform/x360".into()), ..Default::default() });
        assert!(hits.is_empty());
        let hits = search(&kb, &Query { text: "micro".into(), ..Default::default() });
        assert_eq!(hits[0].finding.id, "F-000000000b", "prefix match");
    }

    #[test]
    fn stopwords_do_not_match() {
        let kb = kb();
        let hits =
            search(&kb, &Query { text: "oh, the faces of the characters are black".into(), ..Default::default() });
        assert!(hits.is_empty(), "{:?}", hits.iter().map(|h| &h.finding.id).collect::<Vec<_>>());
    }

    #[test]
    fn status_filter_and_empty_query() {
        let kb = kb();
        let hits = search(&kb, &Query { statuses: vec![Status::Verified], ..Default::default() });
        assert_eq!(hits.len(), 1);
        let all = search(&kb, &Query { statuses: vec![Status::Retracted], ..Default::default() });
        assert_eq!(all[0].finding.id, "F-000000000c");
    }
}

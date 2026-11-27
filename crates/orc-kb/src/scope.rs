//! Scope paths like `platform/x360/game/4d5307e6/render`.

/// Lowercase `a-z0-9._-` segments separated by `/`, at most 12 deep.
pub fn is_valid(scope: &str) -> bool {
    !scope.is_empty()
        && scope.len() <= 200
        && scope.split('/').count() <= 12
        && scope.split('/').all(|seg| {
            !seg.is_empty()
                && seg != "."
                && seg != ".."
                && seg.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
        })
}

/// `a` equals `b`, or one contains the other.
pub fn related(a: &str, b: &str) -> bool {
    a == b || is_within(a, b) || is_within(b, a)
}

/// `inner` is strictly inside `outer`.
pub fn is_within(inner: &str, outer: &str) -> bool {
    inner.len() > outer.len() && inner.starts_with(outer) && inner.as_bytes()[outer.len()] == b'/'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validity() {
        assert!(is_valid("platform/x360/game/4d5307e6"));
        assert!(is_valid("tooling/open-recomp"));
        assert!(!is_valid("Platform/x360"));
        assert!(!is_valid("a//b"));
        assert!(!is_valid("a/../b"));
        assert!(!is_valid(""));
    }

    #[test]
    fn relations() {
        assert!(related("platform/x360", "platform/x360/game/a"));
        assert!(related("platform/x360/game/a", "platform/x360"));
        assert!(!related("platform/x360", "platform/x3600"));
        assert!(!related("platform/ps2", "platform/x360"));
    }
}

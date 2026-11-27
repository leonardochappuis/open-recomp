//! Game catalog entries: `kb/games/<platform>-<id>.toml`.
//!
//! A game's scope is `platform/<platform>/game/<id lowercase>`, which ties it
//! to every finding and attempt about it.

use serde::{Deserialize, Serialize};

pub const GAMES_DIR: &str = "kb/games";
pub const COVERS_DIR: &str = "kb/games/covers";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Game {
    pub name: String,
    /// Platform slug: n64, ps1, ps2, ps3, psp, gc, wii, x360, xbox, switch, ...
    pub platform: String,
    /// Title id or serial, e.g. 4D5307E6 or SLUS-20062.
    pub id: String,
    pub status: GameStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<u16>,
    /// Recomp repos (`owner/name`) working on this game.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repos: Vec<String>,
    /// `https://...` image, or a file under `kb/games/covers/`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cover: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameStatus {
    /// Someone wants to start.
    Planned,
    /// Recompilation work under way, doesn't boot yet.
    InProgress,
    /// Boots, gets in game with problems.
    Boots,
    /// Playable start to finish with known issues.
    Playable,
    /// Done.
    Complete,
}

impl std::str::FromStr for GameStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        serde_json::from_value(serde_json::Value::String(s.into()))
            .map_err(|_| format!("unknown status {s:?} (planned, in-progress, boots, playable, complete)"))
    }
}

impl Game {
    pub fn scope(&self) -> String {
        format!("platform/{}/game/{}", self.platform, self.id.to_lowercase())
    }

    pub fn path(&self) -> String {
        format!("{GAMES_DIR}/{}-{}.toml", self.platform, self.id.to_lowercase())
    }

    pub fn render(&self) -> String {
        toml::to_string(self).expect("Game always serializes")
    }
}

pub fn is_platform(s: &str) -> bool {
    (2..=12).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

pub fn is_game_id(s: &str) -> bool {
    (1..=32).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Cover is an https URL or a `kb/games/covers/<name>.{png,jpg,jpeg,webp}` path.
pub fn is_cover(s: &str) -> bool {
    if let Some(rest) = s.strip_prefix("https://") {
        return s.len() <= 300 && !rest.is_empty() && !s.contains(char::is_whitespace) && !s.contains('"');
    }
    let Some(name) = s.strip_prefix(&format!("{COVERS_DIR}/")) else { return false };
    let ok_ext = [".png", ".jpg", ".jpeg", ".webp"].iter().any(|e| name.to_ascii_lowercase().ends_with(e));
    ok_ext
        && name.len() <= 80
        && !name.starts_with('.')
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn sample() -> Game {
        Game {
            name: "Example Racer".into(),
            platform: "x360".into(),
            id: "4D5307E6".into(),
            status: GameStatus::InProgress,
            region: Some("NTSC-U".into()),
            year: Some(2007),
            repos: vec!["someone/example-racer-recomp".into()],
            cover: Some("kb/games/covers/x360-4d5307e6.png".into()),
            tags: vec!["racing".into()],
            description: None,
        }
    }

    #[test]
    fn roundtrip_and_paths() {
        let g = sample();
        assert_eq!(toml::from_str::<Game>(&g.render()).unwrap(), g);
        assert_eq!(g.scope(), "platform/x360/game/4d5307e6");
        assert_eq!(g.path(), "kb/games/x360-4d5307e6.toml");
    }

    #[test]
    fn covers() {
        assert!(is_cover("https://example.com/a.png"));
        assert!(is_cover("kb/games/covers/x360-4d5307e6.webp"));
        assert!(!is_cover("http://example.com/a.png"));
        assert!(!is_cover("kb/games/covers/../../secret.png"));
        assert!(!is_cover("kb/games/covers/a.svg"));
        assert!(!is_cover("javascript:alert(1)"));
    }

    #[test]
    fn ids() {
        assert!(is_platform("x360") && is_platform("n64") && !is_platform("X360"));
        assert!(is_game_id("SLUS-20062") && !is_game_id("a b"));
    }
}

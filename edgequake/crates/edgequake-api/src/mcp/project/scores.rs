//! Score type declaration — never mix unlabeled scales (SPEC-152).

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreType {
    UnitInterval,
    Raw,
}

impl ScoreType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnitInterval => "unit_interval",
            Self::Raw => "raw",
        }
    }
}

/// If any score is outside [0,1], the list is `raw`; else `unit_interval`.
pub fn score_type_for(scores: impl IntoIterator<Item = f64>) -> ScoreType {
    for s in scores {
        if !(0.0..=1.0).contains(&s) {
            return ScoreType::Raw;
        }
    }
    ScoreType::UnitInterval
}

pub fn scores_from_hits(hits: &[Value]) -> ScoreType {
    score_type_for(
        hits.iter()
            .filter_map(|h| h.get("score").and_then(|v| v.as_f64())),
    )
}

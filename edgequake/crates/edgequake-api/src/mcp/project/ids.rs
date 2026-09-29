//! Agent-visible id forms (SPEC-152 object model).

/// Storage / ContextEntity id → agent `ent:{workspace}:{slug}`.
pub fn agent_entity_id(workspace: &str, name_or_id: &str) -> String {
    let slug = entity_slug(name_or_id);
    let ws = if workspace.is_empty() {
        "default"
    } else {
        workspace
    };
    format!("ent:{ws}:{slug}")
}

/// Accept `ent:ws:slug`, `ent:NAME`, bare NAME, or `{ws}::NAME`.
pub fn resolve_entity_lookup(entity_id: &str) -> String {
    let id = entity_id.trim();
    if let Some(rest) = id.strip_prefix("ent:") {
        // ent:ws:slug or ent:NAME
        if let Some((_, slug)) = rest.split_once(':') {
            return slug.to_ascii_uppercase().replace(' ', "_");
        }
        return rest.to_ascii_uppercase().replace(' ', "_");
    }
    if let Some((_, name)) = id.split_once("::") {
        return name.to_ascii_uppercase().replace(' ', "_");
    }
    id.to_ascii_uppercase().replace(' ', "_")
}

pub fn entity_slug(name_or_id: &str) -> String {
    let raw = name_or_id
        .strip_prefix("ent:")
        .map(|rest| rest.split_once(':').map(|(_, slug)| slug).unwrap_or(rest))
        .unwrap_or(name_or_id);
    let raw = raw.rsplit("::").next().unwrap_or(raw);
    raw.trim()
        .to_ascii_uppercase()
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

/// Title Case display from ALL_CAPS slug.
pub fn title_case_label(slug: &str) -> String {
    slug.split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(f) => {
                    let mut s = f.to_uppercase().collect::<String>();
                    s.push_str(&chars.as_str().to_lowercase());
                    s
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}

pub fn is_artifact_type(entity_type: &str) -> bool {
    let t = entity_type.to_ascii_uppercase();
    matches!(
        t.as_str(),
        "ARTIFACT" | "DRAWING" | "FIGURE" | "IMAGE" | "CONTENT"
    ) || t.contains("DRAWING")
        || t.contains("FIGURE")
}

pub fn map_entity_type(raw: &str) -> &'static str {
    match raw.to_ascii_uppercase().as_str() {
        "PERSON" | "PEOPLE" => "Person",
        "ORGANIZATION" | "ORG" | "COMPANY" => "Organization",
        "CONCEPT" => "Concept",
        "METHOD" | "TECH" => "Method",
        "SYSTEM" => "System",
        "METRIC" | "DATA" => "Metric",
        "EVENT" => "Event",
        "ARTIFACT" | "DRAWING" | "CONTENT" | "FIGURE" => "Artifact",
        "LOCATION" | "GEO" => "Location",
        _ => "Concept",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_slug() {
        assert_eq!(entity_slug("ent:ws:action_fusion"), "ACTION_FUSION");
        assert_eq!(entity_slug("ACTION_FUSION"), "ACTION_FUSION");
        assert_eq!(title_case_label("ACTION_FUSION"), "Action Fusion");
    }
}

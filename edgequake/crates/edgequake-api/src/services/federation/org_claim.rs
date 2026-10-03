//! Keycloak `organization` claim parsing (SPEC-158 / 05-keycloak-integration).
//!
//! Keycloak's Organization Membership mapper emits different shapes depending on version and
//! the "Add organization id" toggle:
//! - `["acme"]`                          (aliases only)
//! - `{"acme": {"id": "…"}}`            (alias → attributes, 26.x with `add-id`)
//! - `"acme"`                            (single-valued)
//! - `[{"alias": "acme", "id": "…"}]`   (defensive: array of objects)

use serde_json::Value;

/// One organization the subject belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgRef {
    pub alias: String,
    pub id: Option<String>,
}

/// Parse any supported shape. Empty aliases are dropped; aliases are lower-cased
/// (tenant slugs are lower-case, LAW-158-5) and de-duplicated preserving order.
pub fn parse_organization_claim(value: &Value) -> Vec<OrgRef> {
    let mut out: Vec<OrgRef> = Vec::new();
    match value {
        Value::String(alias) => push(&mut out, alias, None),
        Value::Array(items) => {
            for item in items {
                match item {
                    Value::String(alias) => push(&mut out, alias, None),
                    Value::Object(map) => {
                        let alias = map.get("alias").and_then(Value::as_str);
                        let id = map.get("id").and_then(Value::as_str);
                        if let Some(alias) = alias {
                            push(&mut out, alias, id);
                        }
                    }
                    _ => {}
                }
            }
        }
        Value::Object(map) => {
            for (alias, attrs) in map {
                let id = attrs.get("id").and_then(Value::as_str);
                push(&mut out, alias, id);
            }
        }
        _ => {}
    }
    out
}

fn push(out: &mut Vec<OrgRef>, alias: &str, id: Option<&str>) {
    let alias = alias.trim().to_ascii_lowercase();
    if alias.is_empty() || out.iter().any(|o| o.alias == alias) {
        return;
    }
    out.push(OrgRef {
        alias,
        id: id.map(ToOwned::to_owned),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_alias_array() {
        let orgs = parse_organization_claim(&json!(["Acme", "globex"]));
        assert_eq!(
            orgs.iter().map(|o| o.alias.as_str()).collect::<Vec<_>>(),
            ["acme", "globex"]
        );
    }

    #[test]
    fn parses_object_map_with_ids() {
        let orgs = parse_organization_claim(&json!({"acme": {"id": "abc"}}));
        assert_eq!(
            orgs,
            vec![OrgRef {
                alias: "acme".into(),
                id: Some("abc".into())
            }]
        );
    }

    #[test]
    fn parses_single_string_and_object_array() {
        assert_eq!(parse_organization_claim(&json!("acme")).len(), 1);
        let orgs = parse_organization_claim(&json!([{"alias": "acme", "id": "1"}]));
        assert_eq!(orgs[0].id.as_deref(), Some("1"));
    }

    #[test]
    fn drops_empty_duplicates_and_garbage() {
        assert!(parse_organization_claim(&json!(["", "  "])).is_empty());
        assert!(parse_organization_claim(&json!(42)).is_empty());
        assert_eq!(parse_organization_claim(&json!(["a", "A"])).len(), 1);
    }
}

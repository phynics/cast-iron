//! Upstream record shapes, parsed at the boundary.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// One upstream record or nested section. The same shape serves top-level records and `sections`
/// children. Fields that vary by record type stay in `rest`, in upstream order.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Node {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<Node>,
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

impl Node {
    /// Parses and validates a top-level record.
    pub fn parse_record(bytes: &[u8]) -> Result<Node, String> {
        let node: Node =
            serde_json::from_slice(bytes).map_err(|e| format!("malformed JSON: {e}"))?;
        for (field, value) in [
            ("name", &node.name),
            ("url", &node.url),
            ("source", &node.source),
            ("type", &node.kind),
        ] {
            if value.as_deref().is_none_or(|v| v.trim().is_empty()) {
                return Err(format!("missing or empty required field `{field}`"));
            }
        }
        if crate::slug::url_slug(node.url.as_deref().unwrap_or_default()).is_none() {
            return Err(format!(
                "url {:?} is not a pfsrd:// url with a path",
                node.url.as_deref().unwrap_or_default()
            ));
        }
        Ok(node)
    }

    pub fn str_field(&self, key: &str) -> Option<&str> {
        self.rest.get(key).and_then(Value::as_str)
    }

    pub fn title(&self) -> &str {
        self.name.as_deref().unwrap_or_default()
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_record_and_keeps_extra_fields_in_order() {
        let json = br#"{"name":"A","url":"pfsrd://B/A","source":"B","type":"spell","zeta":"1","alpha":"2","sections":[{"type":"section","body":"<p>x</p>"}]}"#;
        let node = Node::parse_record(json).unwrap();
        assert_eq!(node.rest.keys().collect::<Vec<_>>(), ["zeta", "alpha"]);
        assert_eq!(node.sections.len(), 1);
    }

    #[test]
    fn rejects_malformed_records_with_a_reason() {
        let missing =
            Node::parse_record(br#"{"name":"A","source":"B","type":"spell"}"#).unwrap_err();
        assert!(missing.contains("`url`"), "{missing}");
        let bad_type =
            Node::parse_record(br#"{"name":3,"url":"pfsrd://B/A","source":"B","type":"x"}"#)
                .unwrap_err();
        assert!(bad_type.contains("malformed JSON"), "{bad_type}");
        let bad_url =
            Node::parse_record(br#"{"name":"A","url":"http://x","source":"B","type":"x"}"#)
                .unwrap_err();
        assert!(bad_url.contains("pfsrd://"), "{bad_url}");
        assert!(Node::parse_record(b"{not json").is_err());
    }
}

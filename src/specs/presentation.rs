//! Human reading order only. Hashes continue to use canonical serde serialization.
use serde::{
    Serialize, Serializer,
    ser::{SerializeMap, SerializeSeq},
};
use serde_json::Value;

struct Readable<'a>(&'a Value);
impl Serialize for Readable<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.0 {
            Value::Object(object) => {
                const ORDER: &[&str] = &[
                    "id",
                    "title",
                    "tags",
                    "goal",
                    "rationale",
                    "scope",
                    "questions",
                    "depends_on",
                    "related_to",
                    "operations",
                    "action",
                    "requirement",
                    "statement",
                    "scenarios",
                    "given",
                    "when",
                    "then",
                    "verification",
                    "tasks",
                    "description",
                    "done",
                    "checks",
                ];
                const SERVICE: &[&str] = &[
                    "imports",
                    "baseline",
                    "scoped_baseline",
                    "revision_tags",
                    "created_at",
                    "archived",
                    "abandoned_reason",
                    "evidence",
                    "runs",
                    "review",
                    "acceptance",
                    "history",
                ];
                let mut map = serializer.serialize_map(Some(object.len()))?;
                for key in ORDER {
                    if let Some(value) = object.get(*key) {
                        map.serialize_entry(key, &Readable(value))?;
                    }
                }
                for (key, value) in object {
                    if !ORDER.contains(&key.as_str()) && !SERVICE.contains(&key.as_str()) {
                        map.serialize_entry(key, &Readable(value))?;
                    }
                }
                for key in SERVICE {
                    if let Some(value) = object.get(*key) {
                        map.serialize_entry(key, &Readable(value))?;
                    }
                }
                map.end()
            }
            Value::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(&Readable(value))?;
                }
                seq.end()
            }
            value => value.serialize(serializer),
        }
    }
}

pub fn pretty_value(value: &Value) -> serde_json::Result<String> {
    serde_json::to_string_pretty(&Readable(value))
}

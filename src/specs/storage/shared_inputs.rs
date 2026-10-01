//! Catalog v5 interns complete evidence input sets within each results document.
//! Evidence and history object hashes are always those of the expanded v3 model.
use super::*;

type Object = serde_json::Map<String, Value>;
const TABLE: &str = "input_manifests";
const REFERENCE: &str = "input_manifest_sha256";

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct InputManifest {
    files: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    input_paths: Option<BTreeSet<String>>,
}

pub(super) fn compact(results: &mut Object) -> Result<()> {
    let mut manifests = Object::new();
    let mut intern = |value: &mut Value| -> Result<()> {
        let evidence = value.as_object_mut().ok_or("InvalidEvidence")?;
        let mut manifest = Object::new();
        manifest.insert(
            "files".into(),
            evidence.remove("files").ok_or("InvalidEvidence")?,
        );
        if let Some(paths) = evidence.remove("input_paths") {
            manifest.insert("input_paths".into(), paths);
        }
        let manifest = Value::Object(manifest);
        let key = digest(&manifest);
        manifests.entry(key.clone()).or_insert(manifest);
        evidence.insert(REFERENCE.into(), json!(key));
        Ok(())
    };
    for evidence in results
        .get_mut("evidence")
        .and_then(Value::as_object_mut)
        .ok_or("InvalidEvidence")?
        .values_mut()
    {
        intern(evidence)?;
    }
    for value in history_objects(results)?.values_mut() {
        if value.get("files").is_some() {
            intern(value)?;
        }
    }
    results.insert(TABLE.into(), Value::Object(manifests));
    Ok(())
}

fn history_objects(results: &mut Object) -> Result<&mut Object> {
    results
        .get_mut("history")
        .and_then(|history| history.get_mut("objects"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "InvalidCompactHistory".into())
}

pub(super) fn expand(results: &mut Object) -> Result<()> {
    let manifests = results.remove(TABLE).ok_or("InputManifestsMissing")?;
    let manifests = manifests.as_object().ok_or("InvalidInputManifests")?;
    // Validate the entire table, including unused entries: no corrupt payload is
    // silently carried through a successful read or transaction recovery.
    for (key, value) in manifests {
        serde_json::from_value::<InputManifest>(value.clone())
            .map_err(|e| format!("InvalidInputManifest: {e}"))?;
        if digest(value) != *key {
            return Err("InputManifestHashMismatch".into());
        }
    }
    for evidence in results
        .get_mut("evidence")
        .and_then(Value::as_object_mut)
        .ok_or("InvalidEvidence")?
        .values_mut()
    {
        expand_evidence(evidence, manifests)?;
    }
    for value in history_objects(results)?.values_mut() {
        // Null and Review are the only non-evidence history objects. Checking
        // the typed review also rejects injected reference/inline input fields.
        if serde_json::from_value::<Option<Review>>(value.clone()).is_err() {
            expand_evidence(value, manifests)?;
        }
    }
    Ok(())
}

fn expand_evidence(value: &mut Value, manifests: &Object) -> Result<()> {
    let evidence = value.as_object_mut().ok_or("InvalidEvidence")?;
    if evidence.contains_key("files") || evidence.contains_key("input_paths") {
        return Err("InlineEvidenceInputsUnsupported: catalog v5 requires references".into());
    }
    let reference = evidence
        .remove(REFERENCE)
        .ok_or("EvidenceInputReferenceMissing")?;
    let key = reference.as_str().ok_or("InvalidEvidenceInputReference")?;
    let manifest = manifests.get(key).ok_or("InputManifestMissing")?;
    // Manifests have already been validated as objects with only input fields.
    evidence.extend(manifest.as_object().unwrap().clone());
    serde_json::from_value::<Evidence>(value.clone())
        .map_err(|e| format!("InvalidEvidence: {e}"))?;
    Ok(())
}

pub(super) fn schema(results: &Value) -> Value {
    let mut schema = results.clone();
    let mut evidence = schema["$defs"]["Evidence"].clone();
    let fields = evidence["properties"].as_object_mut().unwrap();
    fields.remove("files");
    fields.remove("input_paths");
    fields.insert(
        REFERENCE.into(),
        json!({"type":"string","pattern":"^[0-9a-f]{64}$"}),
    );
    let required = evidence["required"].as_array_mut().unwrap();
    required.retain(|v| v != "files" && v != "input_paths");
    required.push(json!(REFERENCE));
    schema["$defs"]["Evidence"] = evidence;
    schema["$defs"]["InputManifest"] = json!(schemars::schema_for!(InputManifest));
    let history = json!(schemars::schema_for!(CompactHistory));
    schema["$defs"]["HistoryEntry"] = history["$defs"]["HistoryEntry"].clone();
    let mut history = history;
    history.as_object_mut().unwrap().remove("$defs");
    history["properties"]["objects"]["additionalProperties"] = json!({"anyOf":[
        {"$ref":"#/$defs/Evidence"},{"$ref":"#/$defs/Review"},{"type":"null"}
    ]});
    schema["properties"]["history"] = history;
    schema["properties"][TABLE] = json!({"type":"object","propertyNames":{"pattern":"^[0-9a-f]{64}$"},"additionalProperties":{"$ref":"#/$defs/InputManifest"}});
    schema["required"]
        .as_array_mut()
        .unwrap()
        .push(json!(TABLE));
    schema
}

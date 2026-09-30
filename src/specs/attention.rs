//! Explicit human handoffs. Content agreement never grants execution authority.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Question,
    Requirements,
    Result,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub sequence: u64,
    pub decision: String,
    pub author: String,
    pub comment: String,
    pub at: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub id: String,
    pub kind: Kind,
    pub reason: String,
    pub content_sha256: String,
    pub change_sha256: String,
    pub inputs_sha256: String,
    pub at: u64,
    pub response: Option<Response>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Attention {
    pub requests: Vec<Request>,
    pub snapshots: BTreeMap<String, Value>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Input {
    Request {
        request_id: String,
        kind: Kind,
        reason: String,
        content_sha256: String,
    },
    Respond {
        request_id: String,
        content_sha256: String,
        decision: String,
        author: String,
        comment: String,
        #[serde(default)]
        verified_revision: String,
    },
}
pub(super) fn content(c: &Change) -> Value {
    json!({"title":c.title,"goal":c.goal,"rationale":c.rationale,"scope":c.scope,
        "operations":c.operations,"depends_on":c.depends_on,
        "tasks":c.tasks.iter().map(|t| json!({"id":t.id,"description":t.description})).collect::<Vec<_>>()})
}
pub(super) fn supported(s: &Store, c: &Change) -> bool {
    s.schema_version >= 3
        && c.scoped_baseline == Some(true)
        && c.created_at.is_some()
        && !c.title.trim().is_empty()
        && !c.operations.is_empty()
}
fn ready(root: &Path, s: &Store, c: &Change) -> bool {
    let mut report = Report::new("spec-check");
    readiness(root, s, c, &mut report, true);
    report.exit_code() == 0
}
pub(super) fn validate(s: &Store) -> Result<()> {
    for c in s.changes.values() {
        if let Some(a) = &c.attention {
            let mut ids = BTreeSet::new();
            let mut sequences = BTreeSet::new();
            for (sha, value) in &a.snapshots {
                if &digest(value) != sha {
                    return Err("AttentionCorrupt: snapshot hash".into());
                }
            }
            for r in &a.requests {
                if r.id.trim().is_empty()
                    || r.reason.trim().is_empty()
                    || !ids.insert(&r.id)
                    || !a.snapshots.contains_key(&r.content_sha256)
                {
                    return Err("AttentionCorrupt: invalid request".into());
                }
                if let Some(response) = &r.response {
                    if response.sequence == 0 || !sequences.insert(response.sequence) {
                        return Err("AttentionCorrupt: response sequence".into());
                    }
                    valid_response(
                        &r.kind,
                        &response.decision,
                        &response.author,
                        &response.comment,
                    )?;
                }
            }
        }
    }
    Ok(())
}
fn valid_response(kind: &Kind, decision: &str, author: &str, comment: &str) -> Result<()> {
    if author.trim().is_empty()
        || (matches!(decision, "answer" | "needs_changes") && comment.trim().is_empty())
        || !match kind {
            Kind::Question => decision == "answer",
            _ => matches!(decision, "accepted" | "needs_changes"),
        }
    {
        return Err("AttentionResponseIncomplete".into());
    }
    Ok(())
}
pub(super) fn record_result(
    c: &mut Change,
    revision: &str,
    inputs: &str,
    decision: &str,
    author: &str,
    comment: &str,
    at: u64,
) -> Result<()> {
    if let Some(a) = &mut c.attention {
        let mut sequence = a
            .requests
            .iter()
            .filter_map(|r| r.response.as_ref().map(|v| v.sequence))
            .max()
            .unwrap_or(0);
        for r in &mut a.requests {
            if r.kind == Kind::Result
                && r.response.is_none()
                && r.change_sha256 == revision
                && r.inputs_sha256 == inputs
            {
                sequence = sequence
                    .checked_add(1)
                    .ok_or("AttentionSequenceExhausted")?;
                r.response = Some(Response {
                    sequence,
                    decision: decision.into(),
                    author: author.into(),
                    comment: comment.into(),
                    at,
                });
            }
        }
    }
    Ok(())
}
pub(super) fn mutate_value(root: &Path, s: &mut Store, id: &str, value: Value) -> Result<()> {
    let input: Input =
        serde_json::from_value(value).map_err(|e| format!("InvalidAttention: {e}"))?;
    let c = s.changes.get(id).ok_or("ChangeMissing")?;
    if !supported(s, c) {
        return Err("UiProfileUnsupported".into());
    }
    if c.abandoned_reason.is_some() {
        return Err("AbandonedChange".into());
    }
    let snapshot = content(c);
    let sha = digest(&snapshot);
    let at = progress::now()?;
    match input {
        Input::Request {
            request_id,
            kind,
            reason,
            content_sha256,
        } => {
            if content_sha256 != sha {
                return Err("DecisionStale: content".into());
            }
            if request_id.trim().is_empty() || reason.trim().is_empty() {
                return Err("AttentionRequestIncomplete".into());
            }
            if let Some(old) = c
                .attention
                .as_ref()
                .and_then(|a| a.requests.iter().find(|r| r.id == request_id))
            {
                if old.kind == kind && old.reason == reason && old.content_sha256 == sha {
                    return Ok(());
                }
                return Err("AttentionRequestConflict".into());
            }
            if kind == Kind::Requirements {
                let mut report = Report::new("spec-validate");
                readiness(root, s, c, &mut report, false);
                if report.exit_code() != 0 {
                    return Err("RequirementsNotPrepared".into());
                }
            }
            if kind == Kind::Result && !ready(root, s, c) {
                return Err("DecisionNotReady".into());
            }
            let request = Request {
                id: request_id,
                kind: kind.clone(),
                reason,
                content_sha256: sha.clone(),
                change_sha256: progress::revision(c),
                inputs_sha256: if kind == Kind::Result {
                    progress::inputs_hash(root, c)?
                } else {
                    String::new()
                },
                at,
                response: None,
            };
            let a = s
                .changes
                .get_mut(id)
                .unwrap()
                .attention
                .get_or_insert_with(Attention::default);
            a.snapshots.insert(sha, snapshot);
            a.requests.push(request);
        }
        Input::Respond {
            request_id,
            content_sha256,
            decision,
            author,
            comment,
            verified_revision,
        } => {
            let r = c
                .attention
                .as_ref()
                .and_then(|a| a.requests.iter().find(|r| r.id == request_id))
                .ok_or("AttentionRequestMissing")?
                .clone();
            if sha != content_sha256 || sha != r.content_sha256 {
                return Err("DecisionStale: content".into());
            }
            valid_response(&r.kind, &decision, &author, &comment)?;
            if let Some(previous) = &r.response {
                if previous.decision == decision
                    && previous.author == author
                    && previous.comment == comment
                {
                    return Ok(());
                }
                return Err("AttentionAlreadyAnswered".into());
            }
            if r.kind == Kind::Result {
                progress::decide_value(
                    root,
                    s,
                    json!({"decisions":[{"id":id,"decision":decision,"decided_by":author,
                    "comment":comment,"verified_revision":verified_revision,"change_sha256":r.change_sha256,"inputs_sha256":r.inputs_sha256}]}),
                )?;
                return Ok(());
            }
            let a = s.changes.get_mut(id).unwrap().attention.as_mut().unwrap();
            let sequence = a
                .requests
                .iter()
                .filter_map(|r| r.response.as_ref().map(|v| v.sequence))
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or("AttentionSequenceExhausted")?;
            a.requests
                .iter_mut()
                .find(|r| r.id == request_id)
                .unwrap()
                .response = Some(Response {
                sequence,
                decision,
                author,
                comment,
                at,
            });
        }
    }
    Ok(())
}
pub(super) fn project(root: &Path, s: &Store) -> Result<Value> {
    let mut rows = Vec::new();
    let mut excluded = Vec::new();
    let mut counts = BTreeMap::from([
        ("needs_decision", 0),
        ("in_work", 0),
        ("completed", 0),
        ("historical", 0),
        ("cancelled", 0),
        ("unknown", 0),
    ]);
    for c in s.changes.values() {
        if !supported(s, c) {
            excluded.push(json!({"id":c.id,"reason":"Requires directory catalog, scoped baseline, creation time, title and operations"}));
            continue;
        }
        let current = content(c);
        let sha = digest(&current);
        let technical_ready = ready(root, s, c);
        let inputs = progress::inputs_hash(root, c);
        let requests: Vec<_> = c
            .attention
            .as_ref()
            .map(|a| a.requests.iter().collect())
            .unwrap_or_default();
        let approved = requests
            .iter()
            .filter(|r| {
                r.kind == Kind::Requirements
                    && r.response
                        .as_ref()
                        .is_some_and(|v| v.decision == "accepted")
            })
            .max_by_key(|r| r.response.as_ref().unwrap().sequence);
        let last_decision = requests
            .iter()
            .filter(|r| r.kind == Kind::Requirements && r.response.is_some())
            .max_by_key(|r| r.response.as_ref().unwrap().sequence);
        let mut open: Vec<_> = requests
            .iter()
            .copied()
            .filter(|r| {
                r.response.is_none()
                    && r.content_sha256 == sha
                    && (r.kind != Kind::Result
                        || (technical_ready
                            && r.change_sha256 == progress::revision(c)
                            && inputs.as_ref().is_ok_and(|h| h == &r.inputs_sha256)))
            })
            .collect();
        open.sort_by(|a, b| (&a.kind, a.at, &a.id).cmp(&(&b.kind, b.at, &b.id)));
        if c.abandoned_reason.is_some() {
            open.clear();
        }
        let human = progress::human_state(root, c, technical_ready);
        let category = if c.abandoned_reason.is_some() {
            "cancelled"
        } else if !open.is_empty() {
            "needs_decision"
        } else if inputs.is_err() {
            "unknown"
        } else if technical_ready && human == "accepted" {
            "completed"
        } else if c.archived {
            "historical"
        } else {
            "in_work"
        };
        *counts.get_mut(category).unwrap() += 1;
        let baseline =
            approved.and_then(|r| c.attention.as_ref()?.snapshots.get(&r.content_sha256));
        let changes: Vec<_> = baseline
            .map(|b| {
                current
                    .as_object()
                    .unwrap()
                    .iter()
                    .filter(|(k, v)| b.get(*k) != Some(*v))
                    .map(|(k, v)| json!({"field":k,"before":b.get(k),"after":v}))
                    .collect()
            })
            .unwrap_or_default();
        rows.push(json!({"id":c.id,"title":c.title,"tags":c.tags,"created_at":c.created_at,"category":category,"content_sha256":sha,
            "requirements_agreement":match last_decision {None=>"pending",Some(r) if r.content_sha256==sha=>r.response.as_ref().unwrap().decision.as_str(),Some(_)=>"stale"},
            "primary_action":open.first(),"requests":open,"history":requests,"changes_since_agreement":if baseline.is_some(){Some(changes)}else{None},
            "diagnostic":inputs.err(),"editable":!c.archived,"integrated":c.archived&&c.abandoned_reason.is_none(),"technical_ready":technical_ready,"human":human}));
    }
    rows.sort_by_key(|r| {
        (
            match r["primary_action"]["kind"].as_str() {
                Some("question") => 0,
                Some("requirements") => 1,
                Some("result") => 2,
                _ => 3,
            },
            r["primary_action"]["at"].as_u64().unwrap_or(u64::MAX),
            r["id"].as_str().unwrap().to_string(),
        )
    });
    Ok(
        json!({"profile":"directory-scoped-v1","changes":rows,"counts":counts,"excluded_count":excluded.len(),"excluded":excluded}),
    )
}

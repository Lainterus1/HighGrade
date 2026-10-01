// Read-only characterization helper linked to the built candidate library.
// Debug covers every Store field; BTree containers and the fixed probe binary
// make its before/after digest deterministic. No catalog writes occur here.
fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).expect("project root"));
    let (store, _) = highgrade::specs::load(&root).expect("readable catalog");
    let model = highgrade::hash(format!("{store:?}").as_bytes());
    let trace = highgrade::specs::catalog_hash(&store).expect("catalog hash");
    let change = &store.changes["HG-0063"];
    let passed = change.evidence.values().filter(|e| e.outcome == highgrade::specs::Outcome::Passed).count();
    let unknown = change.evidence.values().filter(|e| e.outcome == highgrade::specs::Outcome::Unknown).count();
    println!("{{\"model_sha256\":\"{model}\",\"trace_sha256\":\"{trace}\",\"changes\":{},\"requirements\":{},\"next_number\":{},\"hg0063_evidence\":{},\"hg0063_passed_observations\":{passed},\"hg0063_unknown_observations\":{unknown},\"hg0063_history_snapshots\":{}}}",store.changes.len(),store.requirements.len(),store.next_number,change.evidence.len(),change.history.len());
}

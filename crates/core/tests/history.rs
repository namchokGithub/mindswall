use mindswall_core::history::{History, HistoryEntry};

#[test]
fn history_limits_entries_and_persists() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.json");
    let mut history = History::default();
    for n in 0..105 {
        history.push(HistoryEntry {
            path: format!("{n}.jpg").into(),
            timestamp: n,
            monitor: None,
        });
    }
    assert_eq!(history.entries.len(), 100);
    history.save(&path).unwrap();
    let restored = History::load(&path).unwrap();
    assert_eq!(restored.entries.first().unwrap().timestamp, 104);
    assert_eq!(restored.entries.last().unwrap().timestamp, 5);
}

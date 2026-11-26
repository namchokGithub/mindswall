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
    assert_eq!(history.entries.len(), 10);
    history.save(&path).unwrap();
    let restored = History::load(&path).unwrap();
    assert_eq!(restored.entries.first().unwrap().timestamp, 104);
    assert_eq!(restored.entries.last().unwrap().timestamp, 95);
}

#[test]
fn legacy_history_loads_only_the_ten_most_recent_and_saves_that_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.json");
    let entries: Vec<_> = (0..100)
        .rev()
        .map(|timestamp| HistoryEntry {
            path: format!("{timestamp}.jpg").into(),
            timestamp,
            monitor: None,
        })
        .collect();
    std::fs::write(&path, serde_json::to_vec(&History { entries }).unwrap()).unwrap();
    let history = History::load(&path).unwrap();
    assert_eq!(history.entries.len(), 10);
    assert_eq!(history.entries.first().unwrap().timestamp, 99);
    assert_eq!(history.entries.last().unwrap().timestamp, 90);
    history.save(&path).unwrap();
    let saved: History = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(saved.entries.len(), 10);
}

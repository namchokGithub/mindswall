use mindswall_core::{scheduler::Scheduler, slideshow::Slideshow};
use std::{
    collections::HashSet,
    path::PathBuf,
    time::{Duration, Instant},
};

#[test]
fn shuffle_has_no_repeats_per_cycle_or_at_boundary() {
    let images: Vec<PathBuf> = ["a", "b", "c", "d"].iter().map(PathBuf::from).collect();
    let mut show = Slideshow::new(images.clone(), true).unwrap();
    let first: Vec<_> = (0..4).map(|_| show.next().to_owned()).collect();
    let second: Vec<_> = (0..4).map(|_| show.next().to_owned()).collect();
    assert_eq!(first.iter().collect::<HashSet<_>>().len(), 4);
    assert_eq!(second.iter().collect::<HashSet<_>>().len(), 4);
    assert_ne!(first.last(), second.first());
}

#[test]
fn history_replays_before_advancing_queue() {
    let mut show = Slideshow::new(vec!["a".into(), "b".into(), "c".into()], false).unwrap();
    assert_eq!(show.next(), PathBuf::from("a"));
    assert_eq!(show.next(), PathBuf::from("b"));
    assert_eq!(show.previous().unwrap(), PathBuf::from("a"));
    assert_eq!(show.next(), PathBuf::from("b"));
    assert_eq!(show.next(), PathBuf::from("c"));
}

#[test]
fn scheduler_pauses_resumes_and_handles_elapsed_deadline() {
    let now = Instant::now();
    let mut timer = Scheduler::new(Duration::from_secs(60), now);
    assert!(!timer.is_due(now + Duration::from_secs(59)));
    timer.pause(now + Duration::from_secs(20));
    assert!(!timer.is_due(now + Duration::from_secs(100)));
    timer.resume(now + Duration::from_secs(100));
    assert!(timer.is_due(now + Duration::from_secs(140)));
    timer.reset(now + Duration::from_secs(1000));
    assert!(!timer.is_due(now + Duration::from_secs(1000)));
}

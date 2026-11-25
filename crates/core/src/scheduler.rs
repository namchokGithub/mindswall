use std::time::{Duration, Instant};

pub struct Scheduler {
    interval: Duration,
    deadline: Instant,
    remaining: Option<Duration>,
}
impl Scheduler {
    pub fn new(interval: Duration, now: Instant) -> Self {
        Self {
            interval,
            deadline: now + interval,
            remaining: None,
        }
    }
    pub fn is_due(&self, now: Instant) -> bool {
        self.remaining.is_none() && now >= self.deadline
    }
    pub fn wait(&self, now: Instant) -> Option<Duration> {
        if self.remaining.is_some() {
            None
        } else {
            Some(self.deadline.saturating_duration_since(now))
        }
    }
    pub fn reset(&mut self, now: Instant) {
        self.deadline = now + self.interval;
        if self.remaining.is_some() {
            self.remaining = Some(self.interval);
        }
    }
    pub fn pause(&mut self, now: Instant) {
        if self.remaining.is_none() {
            self.remaining = Some(self.deadline.saturating_duration_since(now));
        }
    }
    pub fn resume(&mut self, now: Instant) {
        if let Some(remaining) = self.remaining.take() {
            self.deadline = now + remaining;
        }
    }
    pub fn is_paused(&self) -> bool {
        self.remaining.is_some()
    }
}

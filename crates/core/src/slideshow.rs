use crate::Error;
use rand::seq::SliceRandom;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct Slideshow {
    images: Vec<PathBuf>,
    queue: Vec<PathBuf>,
    index: usize,
    shuffle: bool,
    history: Vec<PathBuf>,
    cursor: Option<usize>,
}

impl Slideshow {
    pub fn new(images: Vec<PathBuf>, shuffle: bool) -> Result<Self, Error> {
        if images.is_empty() {
            return Err(Error::NoImages);
        }
        Ok(Self {
            images,
            queue: Vec::new(),
            index: 0,
            shuffle,
            history: Vec::new(),
            cursor: None,
        })
    }
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> &Path {
        if let Some(cursor) = self.cursor
            && cursor + 1 < self.history.len()
        {
            self.cursor = Some(cursor + 1);
            return &self.history[cursor + 1];
        }
        if self.index >= self.queue.len() {
            self.queue = self.images.clone();
            if self.shuffle {
                self.queue.shuffle(&mut rand::rng());
                if self.queue.len() > 1 && self.history.last() == self.queue.first() {
                    self.queue.swap(0, 1);
                }
            }
            self.index = 0;
        }
        self.history.push(self.queue[self.index].clone());
        self.index += 1;
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.cursor = Some(self.history.len() - 1);
        self.history.last().unwrap()
    }
    pub fn previous(&mut self) -> Option<&Path> {
        let cursor = self.cursor?;
        if cursor == 0 {
            return None;
        }
        self.cursor = Some(cursor - 1);
        Some(&self.history[cursor - 1])
    }
    pub fn current(&self) -> Option<&Path> {
        self.cursor.map(|i| self.history[i].as_path())
    }
}

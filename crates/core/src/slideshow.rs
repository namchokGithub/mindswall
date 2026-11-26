use crate::Error;
use rand::seq::SliceRandom;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Slideshow {
    images: Vec<PathBuf>,
    queue: Vec<PathBuf>,
    index: usize,
    shuffle: bool,
    history: Vec<(PathBuf, usize)>,
    cycle_positions: HashMap<PathBuf, usize>,
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
            cycle_positions: HashMap::new(),
            cursor: None,
        })
    }
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> &Path {
        if let Some(cursor) = self.cursor
            && cursor + 1 < self.history.len()
        {
            self.cursor = Some(cursor + 1);
            return &self.history[cursor + 1].0;
        }
        if self.index >= self.queue.len() {
            self.queue = self.images.clone();
            if self.shuffle {
                self.queue.shuffle(&mut rand::rng());
                if self.queue.len() > 1
                    && self.history.last().map(|entry| &entry.0) == self.queue.first()
                {
                    self.queue.swap(0, 1);
                }
            }
            self.index = 0;
            self.cycle_positions.clear();
        }
        let path = self.queue[self.index].clone();
        let position = self.cycle_positions.len() + 1;
        self.cycle_positions.insert(path.clone(), position);
        self.history.push((path, position));
        self.index += 1;
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.cursor = Some(self.history.len() - 1);
        &self.history.last().unwrap().0
    }
    pub fn previous(&mut self) -> Option<&Path> {
        let cursor = self.cursor?;
        if cursor == 0 {
            return None;
        }
        self.cursor = Some(cursor - 1);
        Some(&self.history[cursor - 1].0)
    }
    pub fn current(&self) -> Option<&Path> {
        self.cursor.map(|i| self.history[i].0.as_path())
    }
    pub fn position(&self) -> Option<usize> {
        self.cursor.map(|i| self.history[i].1)
    }
    pub fn total(&self) -> usize {
        self.images.len()
    }
    pub fn select(&mut self, path: PathBuf) -> Result<(), Error> {
        if !self.images.contains(&path) {
            return Err(Error::InvalidFile(path));
        }
        if self.queue.is_empty() {
            self.queue = self.images.clone();
            if self.shuffle {
                self.queue.shuffle(&mut rand::rng());
            }
        }
        if let Some(position) = self
            .queue
            .iter()
            .enumerate()
            .skip(self.index)
            .find_map(|(i, image)| (image == &path).then_some(i))
        {
            self.queue.remove(position);
        }
        if let Some(cursor) = self.cursor {
            self.history.truncate(cursor + 1);
        }
        let next_position = self.cycle_positions.len() + 1;
        let position = *self
            .cycle_positions
            .entry(path.clone())
            .or_insert(next_position);
        self.history.push((path, position));
        if self.history.len() > 100 {
            self.history.remove(0);
        }
        self.cursor = Some(self.history.len() - 1);
        Ok(())
    }
}

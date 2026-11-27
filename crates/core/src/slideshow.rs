use crate::Error;
use rand::seq::SliceRandom;
use std::{
    collections::{HashMap, HashSet},
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
    pub fn images(&self) -> &[PathBuf] {
        &self.images
    }
    pub fn contains(&self, path: &Path) -> bool {
        self.images.iter().any(|image| image == path)
    }

    /// Advance to the next image that `apply` accepts. Images that fail are
    /// removed so they cannot block the slideshow. When no image can be
    /// applied, the slideshow is left unchanged and the last error returned.
    pub fn next_available<E>(
        &mut self,
        mut apply: impl FnMut(&Path) -> Result<(), E>,
    ) -> Result<PathBuf, E> {
        self.next_available_with(|path, _| apply(path).map(|()| Vec::new()))
    }

    /// Like [`Slideshow::next_available`], but `apply` also receives the
    /// unplayed images of the current cycle in order, which may be empty for
    /// the last image of a cycle, or `None` when moving forward through
    /// history after Previous (the image was shown before). Images it returns
    /// were shown together with the new image and count as played in this
    /// cycle.
    pub fn next_available_with<E>(
        &mut self,
        mut apply: impl FnMut(&Path, Option<&[PathBuf]>) -> Result<Vec<PathBuf>, E>,
    ) -> Result<PathBuf, E> {
        let mut candidate = self.clone();
        loop {
            let replay = candidate
                .cursor
                .is_some_and(|cursor| cursor + 1 < candidate.history.len());
            let path = candidate.next().to_owned();
            let upcoming = if replay {
                None
            } else {
                Some(&candidate.queue[candidate.index.min(candidate.queue.len())..])
            };
            match apply(path.as_path(), upcoming) {
                Ok(used) => {
                    candidate.mark_played(&used);
                    *self = candidate;
                    return Ok(path);
                }
                Err(error) => {
                    candidate.remove(&path);
                    if candidate.images.is_empty() {
                        return Err(error);
                    }
                }
            }
        }
    }

    /// Go back to the most recent earlier image that `apply` accepts,
    /// removing earlier images that fail. Returns `None` when there is no
    /// earlier image. On failure the slideshow is left unchanged.
    pub fn previous_available<E>(
        &mut self,
        mut apply: impl FnMut(&Path) -> Result<(), E>,
    ) -> Option<Result<PathBuf, E>> {
        let mut candidate = self.clone();
        let mut path = candidate.previous()?.to_owned();
        loop {
            match apply(path.as_path()) {
                Ok(()) => {
                    *self = candidate;
                    return Some(Ok(path));
                }
                Err(error) => {
                    let cursor = candidate.cursor.unwrap_or(0);
                    let earlier = candidate.history[..cursor]
                        .iter()
                        .any(|(entry, _)| entry != &path);
                    candidate.remove(&path);
                    if !earlier || candidate.images.is_empty() {
                        return Some(Err(error));
                    }
                    // The cursor now points at the closest surviving earlier entry.
                    path = candidate.current()?.to_owned();
                }
            }
        }
    }

    /// Count unplayed images of the current cycle as played, so they are not
    /// shown again before the cycle ends. Other images are ignored.
    fn mark_played(&mut self, paths: &[PathBuf]) {
        for path in paths {
            let start = self.index.min(self.queue.len());
            if let Some(offset) = self.queue[start..].iter().position(|image| image == path) {
                let image = self.queue.remove(start + offset);
                self.queue.insert(start, image);
                self.index = start + 1;
                let position = self.cycle_positions.len() + 1;
                self.cycle_positions.insert(path.clone(), position);
            }
        }
    }

    /// Remove an image from the collection, the current queue, and history.
    pub fn remove(&mut self, path: &Path) {
        let mut paths = HashSet::new();
        paths.insert(path.to_owned());
        self.remove_all(&paths);
    }

    fn remove_all(&mut self, paths: &HashSet<PathBuf>) {
        if paths.is_empty() {
            return;
        }
        self.images.retain(|image| !paths.contains(image));
        let played = self.index.min(self.queue.len());
        let removed_played = self.queue[..played]
            .iter()
            .filter(|image| paths.contains(*image))
            .count();
        self.queue.retain(|image| !paths.contains(image));
        self.index = played - removed_played;
        for path in paths {
            self.cycle_positions.remove(path);
        }
        let removed_before_cursor = self.cursor.map(|cursor| {
            self.history[..=cursor]
                .iter()
                .filter(|(entry, _)| paths.contains(entry))
                .count()
        });
        self.history.retain(|(entry, _)| !paths.contains(entry));
        self.cursor = match (self.cursor, removed_before_cursor) {
            _ if self.history.is_empty() => None,
            (Some(cursor), Some(removed)) => Some((cursor + 1 - removed).saturating_sub(1)),
            _ => None,
        };
    }

    /// Replace the collection while keeping the current cycle. Removed images
    /// leave the queue and history, new images join the unplayed part of the
    /// queue, and only the unplayed part is reordered for the shuffle setting.
    pub fn update(&mut self, images: Vec<PathBuf>, shuffle: bool) -> Result<(), Error> {
        if images.is_empty() {
            return Err(Error::NoImages);
        }
        let wanted: HashSet<&PathBuf> = images.iter().collect();
        let removed: HashSet<PathBuf> = self
            .images
            .iter()
            .filter(|image| !wanted.contains(image))
            .cloned()
            .collect();
        self.remove_all(&removed);
        let known: HashSet<&PathBuf> = self.images.iter().collect();
        let added: Vec<PathBuf> = images
            .iter()
            .filter(|image| !known.contains(image))
            .cloned()
            .collect();
        let reorder = !added.is_empty() || shuffle != self.shuffle;
        if !self.queue.is_empty() {
            self.queue.extend(added);
            if reorder {
                let index = self.index.min(self.queue.len());
                let unplayed = &mut self.queue[index..];
                if shuffle {
                    unplayed.shuffle(&mut rand::rng());
                } else {
                    unplayed.sort();
                }
            }
        }
        self.images = images;
        self.shuffle = shuffle;
        Ok(())
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

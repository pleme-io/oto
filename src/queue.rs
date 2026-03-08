use std::collections::VecDeque;

/// Ordered playlist queue.
///
/// Tracks are strings (names or paths). Push to back or front,
/// pop from front, reorder, shuffle.
pub struct Queue {
    tracks: VecDeque<String>,
}

impl Queue {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracks: VecDeque::new(),
        }
    }

    /// Add a track to the back of the queue.
    pub fn push(&mut self, track: String) {
        self.tracks.push_back(track);
    }

    /// Add a track to the front of the queue (play next).
    pub fn push_front(&mut self, track: String) {
        self.tracks.push_front(track);
    }

    /// Remove and return the next track from the front.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<String> {
        self.tracks.pop_front()
    }

    /// Peek at the next track without removing it.
    #[must_use]
    pub fn peek(&self) -> Option<&str> {
        self.tracks.front().map(String::as_str)
    }

    /// Remove a track by index, returning it if valid.
    pub fn remove(&mut self, index: usize) -> Option<String> {
        self.tracks.remove(index)
    }

    /// Remove all tracks.
    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    /// Shuffle the queue order.
    ///
    /// Uses a simple Fisher-Yates-like swap with a basic seeded approach.
    /// For a truly random shuffle, callers can provide external randomness
    /// and use `move_track`.
    pub fn shuffle(&mut self) {
        let len = self.tracks.len();
        if len <= 1 {
            return;
        }
        // Simple deterministic shuffle for the state machine.
        // Real randomness would come from an external RNG.
        let tracks = self.tracks.make_contiguous();
        for i in (1..len).rev() {
            // Simple hash-based index for reproducibility in tests
            let j = (i * 2_654_435_761) % (i + 1);
            tracks.swap(i, j);
        }
    }

    /// Number of tracks in the queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// View the queue contents as a slice.
    #[must_use]
    pub fn items(&self) -> Vec<&str> {
        self.tracks.iter().map(String::as_str).collect()
    }

    /// Move a track from one position to another.
    pub fn move_track(&mut self, from: usize, to: usize) {
        let len = self.tracks.len();
        if from >= len || to >= len || from == to {
            return;
        }
        let track = self.tracks.remove(from).expect("from index was bounds-checked");
        // After removal, if `to` > `from`, the effective index shifts.
        // VecDeque::insert handles this correctly since we removed first.
        let insert_at = if to > len - 1 { len - 1 } else { to };
        self.tracks.insert(insert_at, track);
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

/// Repeat mode for queue playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatMode {
    Off,
    One,
    All,
}

/// Queue manager that wraps a [`Queue`] with repeat mode and history.
///
/// `advance()` pops from the inner queue and respects the repeat mode.
/// `previous()` returns tracks from the history stack.
pub struct QueueManager {
    queue: Queue,
    repeat: RepeatMode,
    history: Vec<String>,
    current: Option<String>,
}

impl QueueManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: Queue::new(),
            repeat: RepeatMode::Off,
            history: Vec::new(),
            current: None,
        }
    }

    /// Access the inner queue for adding/removing tracks.
    pub fn queue_mut(&mut self) -> &mut Queue {
        &mut self.queue
    }

    /// Access the inner queue for inspection.
    #[must_use]
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// Advance to the next track, respecting repeat mode.
    ///
    /// - `RepeatMode::Off`: pops from queue, returns `None` when empty.
    /// - `RepeatMode::One`: returns the current track again (does not pop).
    /// - `RepeatMode::All`: pops from queue; when empty, does NOT auto-refill
    ///   (the caller is responsible for re-queuing).
    pub fn advance(&mut self) -> Option<String> {
        match self.repeat {
            RepeatMode::One => {
                // Repeat the current track
                self.current.clone()
            }
            RepeatMode::Off | RepeatMode::All => {
                // Push current to history before advancing
                if let Some(ref cur) = self.current {
                    self.history.push(cur.clone());
                }
                let next = self.queue.next();
                if next.is_none() && self.repeat == RepeatMode::All {
                    // Queue exhausted under All mode — return None
                    // Caller should re-populate if looping is desired
                    self.current = None;
                    return None;
                }
                self.current.clone_from(&next);
                next
            }
        }
    }

    /// Go back to the previous track from history.
    ///
    /// If there is a current track, it gets pushed to the front of the queue.
    pub fn previous(&mut self) -> Option<String> {
        let prev = self.history.pop()?;
        // Put current back at the front of the queue
        if let Some(cur) = self.current.take() {
            self.queue.push_front(cur);
        }
        self.current = Some(prev.clone());
        Some(prev)
    }

    /// Set the repeat mode.
    pub fn set_repeat(&mut self, mode: RepeatMode) {
        self.repeat = mode;
    }

    /// Current repeat mode.
    #[must_use]
    pub fn repeat_mode(&self) -> RepeatMode {
        self.repeat
    }

    /// The currently playing track, if any.
    #[must_use]
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }
}

impl Default for QueueManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Queue tests ---

    #[test]
    fn queue_new_is_empty() {
        let q = Queue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn queue_push_and_next() {
        let mut q = Queue::new();
        q.push("a.flac".into());
        q.push("b.flac".into());
        assert_eq!(q.len(), 2);
        assert_eq!(q.next(), Some("a.flac".into()));
        assert_eq!(q.next(), Some("b.flac".into()));
        assert_eq!(q.next(), None);
    }

    #[test]
    fn queue_push_front() {
        let mut q = Queue::new();
        q.push("a.flac".into());
        q.push_front("urgent.flac".into());
        assert_eq!(q.next(), Some("urgent.flac".into()));
        assert_eq!(q.next(), Some("a.flac".into()));
    }

    #[test]
    fn queue_peek() {
        let mut q = Queue::new();
        assert!(q.peek().is_none());
        q.push("track.mp3".into());
        assert_eq!(q.peek(), Some("track.mp3"));
        // peek does not remove
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn queue_remove() {
        let mut q = Queue::new();
        q.push("a".into());
        q.push("b".into());
        q.push("c".into());
        let removed = q.remove(1);
        assert_eq!(removed, Some("b".into()));
        assert_eq!(q.items(), vec!["a", "c"]);
    }

    #[test]
    fn queue_remove_out_of_bounds() {
        let mut q = Queue::new();
        q.push("a".into());
        assert!(q.remove(5).is_none());
    }

    #[test]
    fn queue_clear() {
        let mut q = Queue::new();
        q.push("a".into());
        q.push("b".into());
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn queue_items() {
        let mut q = Queue::new();
        q.push("x".into());
        q.push("y".into());
        q.push("z".into());
        assert_eq!(q.items(), vec!["x", "y", "z"]);
    }

    #[test]
    fn queue_move_track_forward() {
        let mut q = Queue::new();
        q.push("a".into());
        q.push("b".into());
        q.push("c".into());
        q.push("d".into());
        q.move_track(0, 2);
        assert_eq!(q.items(), vec!["b", "c", "a", "d"]);
    }

    #[test]
    fn queue_move_track_backward() {
        let mut q = Queue::new();
        q.push("a".into());
        q.push("b".into());
        q.push("c".into());
        q.push("d".into());
        q.move_track(3, 1);
        assert_eq!(q.items(), vec!["a", "d", "b", "c"]);
    }

    #[test]
    fn queue_move_track_same_index() {
        let mut q = Queue::new();
        q.push("a".into());
        q.push("b".into());
        q.move_track(1, 1);
        assert_eq!(q.items(), vec!["a", "b"]);
    }

    #[test]
    fn queue_move_track_out_of_bounds() {
        let mut q = Queue::new();
        q.push("a".into());
        q.move_track(0, 10);
        assert_eq!(q.items(), vec!["a"]);
    }

    #[test]
    fn queue_shuffle_changes_order() {
        let mut q = Queue::new();
        for i in 0..10 {
            q.push(format!("track_{i}"));
        }
        let before: Vec<String> = q.items().iter().map(|s| (*s).to_owned()).collect();
        q.shuffle();
        let after: Vec<String> = q.items().iter().map(|s| (*s).to_owned()).collect();
        // With 10 items, the shuffle should change at least something
        assert_eq!(before.len(), after.len());
        // It's theoretically possible but extremely unlikely for them to be the same
        assert_ne!(before, after, "shuffle should change order for 10 items");
    }

    #[test]
    fn queue_shuffle_single_item() {
        let mut q = Queue::new();
        q.push("only.mp3".into());
        q.shuffle();
        assert_eq!(q.items(), vec!["only.mp3"]);
    }

    #[test]
    fn queue_len_and_is_empty() {
        let mut q = Queue::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);
        q.push("a".into());
        assert!(!q.is_empty());
        assert_eq!(q.len(), 1);
    }

    // --- QueueManager tests ---

    #[test]
    fn manager_advance_off_drains() {
        let mut mgr = QueueManager::new();
        mgr.queue_mut().push("a".into());
        mgr.queue_mut().push("b".into());

        assert_eq!(mgr.advance(), Some("a".into()));
        assert_eq!(mgr.current(), Some("a"));
        assert_eq!(mgr.advance(), Some("b".into()));
        assert_eq!(mgr.current(), Some("b"));
        assert_eq!(mgr.advance(), None);
    }

    #[test]
    fn manager_advance_repeat_one() {
        let mut mgr = QueueManager::new();
        mgr.queue_mut().push("loop.flac".into());
        mgr.set_repeat(RepeatMode::One);

        // First advance pops from queue (because current is None, falls through to Off/All branch)
        // Actually, RepeatMode::One returns current.clone() which is None first time
        // So we need to seed it first:
        let first = mgr.advance(); // current is None, so One returns None
        // Let's set it up properly: advance once in Off mode, then switch
        let mut mgr = QueueManager::new();
        mgr.queue_mut().push("loop.flac".into());
        mgr.queue_mut().push("next.flac".into());

        // Advance once normally to set current
        mgr.set_repeat(RepeatMode::Off);
        assert_eq!(mgr.advance(), Some("loop.flac".into()));

        // Now switch to repeat one
        mgr.set_repeat(RepeatMode::One);
        assert_eq!(mgr.advance(), Some("loop.flac".into()));
        assert_eq!(mgr.advance(), Some("loop.flac".into()));
        assert_eq!(mgr.advance(), Some("loop.flac".into()));

        // Queue should still have "next.flac" since One doesn't pop
        assert_eq!(mgr.queue().len(), 1);
        let _ = first;
    }

    #[test]
    fn manager_advance_repeat_all_exhausts() {
        let mut mgr = QueueManager::new();
        mgr.queue_mut().push("a".into());
        mgr.queue_mut().push("b".into());
        mgr.set_repeat(RepeatMode::All);

        assert_eq!(mgr.advance(), Some("a".into()));
        assert_eq!(mgr.advance(), Some("b".into()));
        // Queue is now empty — returns None even in All mode
        assert_eq!(mgr.advance(), None);
    }

    #[test]
    fn manager_previous_returns_history() {
        let mut mgr = QueueManager::new();
        mgr.queue_mut().push("first.mp3".into());
        mgr.queue_mut().push("second.mp3".into());
        mgr.queue_mut().push("third.mp3".into());

        mgr.advance(); // current = first
        mgr.advance(); // current = second, history = [first]
        mgr.advance(); // current = third, history = [first, second]

        let prev = mgr.previous();
        assert_eq!(prev, Some("second.mp3".into()));
        // "third" should be back at front of queue
        assert_eq!(mgr.queue().peek(), Some("third.mp3"));
    }

    #[test]
    fn manager_previous_empty_history() {
        let mut mgr = QueueManager::new();
        assert!(mgr.previous().is_none());
    }

    #[test]
    fn manager_set_repeat() {
        let mut mgr = QueueManager::new();
        assert_eq!(mgr.repeat_mode(), RepeatMode::Off);
        mgr.set_repeat(RepeatMode::All);
        assert_eq!(mgr.repeat_mode(), RepeatMode::All);
        mgr.set_repeat(RepeatMode::One);
        assert_eq!(mgr.repeat_mode(), RepeatMode::One);
    }
}

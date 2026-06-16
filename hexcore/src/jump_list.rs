pub struct JumpList {
    entries: Vec<u64>,
    index: usize,
    max_size: usize,
    saved_target: Option<u64>,
}

impl JumpList {
    pub fn new(max_size: usize) -> Self {
        JumpList {
            entries: Vec::new(),
            index: 0,
            max_size,
            saved_target: None,
        }
    }

    pub fn push(&mut self, offset: u64) {
        self.entries.truncate(self.index + 1);
        if self.entries.last() != Some(&offset) {
            self.entries.push(offset);
            if self.entries.len() > self.max_size {
                self.entries.remove(0);
            }
        }
        self.index = self.entries.len();
        self.saved_target = None;
    }

    pub fn back(&mut self, current: u64) -> Option<u64> {
        if self.index == self.entries.len() && self.saved_target.is_none() {
            self.saved_target = Some(current);
        }
        if self.index > 0 {
            self.index -= 1;
            Some(self.entries[self.index])
        } else {
            None
        }
    }

    pub fn forward(&mut self) -> Option<u64> {
        if self.index + 1 < self.entries.len() {
            self.index += 1;
            Some(self.entries[self.index])
        } else if self.index + 1 == self.entries.len() {
            self.saved_target.take().map(|pos| {
                self.index += 1;
                pos
            })
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.index = 0;
        self.saved_target = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_back_forward_cycle() {
        let mut jl = JumpList::new(100);
        jl.push(10);
        jl.push(20);
        jl.push(30);
        assert_eq!(jl.back(99), Some(30));
        assert_eq!(jl.back(0), Some(20));
        assert_eq!(jl.back(0), Some(10));
        assert_eq!(jl.back(0), None);
        assert_eq!(jl.forward(), Some(20));
        assert_eq!(jl.forward(), Some(30));
        assert_eq!(jl.forward(), Some(99));
        assert_eq!(jl.forward(), None);
    }

    #[test]
    fn test_back_returns_none_when_empty() {
        let mut jl = JumpList::new(100);
        assert_eq!(jl.back(0), None);
    }

    #[test]
    fn test_forward_returns_none_at_newest() {
        let mut jl = JumpList::new(100);
        jl.push(10);
        jl.push(20);
        assert_eq!(jl.forward(), None);
    }

    #[test]
    fn test_duplicate_not_added() {
        let mut jl = JumpList::new(100);
        jl.push(10);
        jl.push(10);
        assert_eq!(jl.entries.len(), 1);
    }

    #[test]
    fn test_new_push_after_back_truncates() {
        let mut jl = JumpList::new(100);
        jl.push(10);
        jl.push(20);
        jl.push(30);
        jl.back(0);
        jl.back(0);
        jl.push(99);
        assert_eq!(jl.back(0), Some(99));
        assert_eq!(jl.back(0), Some(20));
        assert_eq!(jl.back(0), Some(10));
        assert_eq!(jl.forward(), Some(20));
        assert_eq!(jl.forward(), Some(99));
        assert_eq!(jl.forward(), Some(0));
        assert_eq!(jl.forward(), None);
    }

    #[test]
    fn test_max_size_enforced() {
        let mut jl = JumpList::new(3);
        jl.push(10);
        jl.push(20);
        jl.push(30);
        jl.push(40);
        assert_eq!(jl.entries.len(), 3);
        assert_eq!(jl.entries, vec![20, 30, 40]);
    }

    #[test]
    fn test_clear() {
        let mut jl = JumpList::new(100);
        jl.push(10);
        jl.push(20);
        jl.clear();
        assert_eq!(jl.back(0), None);
        assert_eq!(jl.forward(), None);
    }

    #[test]
    fn test_saved_target_forward_after_full_back() {
        let mut jl = JumpList::new(100);
        jl.push(10);
        jl.push(20);
        // entries = [10, 20], index = 2. Cursor is at 500.
        jl.back(500);  // saved = Some(500), index = 1, returns 20
        jl.back(0);    // index = 0, returns 10
        assert_eq!(jl.forward(), Some(20));  // forward through entries
        assert_eq!(jl.forward(), Some(500)); // forward to saved target
        assert_eq!(jl.forward(), None);
    }
}

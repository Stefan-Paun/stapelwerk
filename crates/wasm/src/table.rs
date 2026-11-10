//! Handle table (`docs/ABI.md`, "Handles"): slot index plus generation counter.
//!
//! A handle is `(generation << 16) | (slot + 1)`. It is never 0 (the low half is at least 1) and
//! never `0xFFFFFFFF` (at most [`MAX_SLOTS`] slots, so the low half stays below `0xFFFF`). Freeing
//! a slot bumps its generation, so an old handle stays invalid when the slot is reused (until the
//! generation wraps after 65536 reuses of the same slot).

/// Maximum number of live entries.
pub const MAX_SLOTS: usize = 0xFFFE;

#[derive(Debug)]
struct Slot<T> {
    generation: u16,
    value: Option<T>,
}

/// A generational slot table.
#[derive(Debug)]
pub struct Table<T> {
    slots: Vec<Slot<T>>,
    free: Vec<usize>,
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Table {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }
}

const fn encode(index: usize, generation: u16) -> u32 {
    ((generation as u32) << 16) | (index as u32 + 1)
}

/// Slot index and generation of a handle; `None` for 0 and other impossible values.
const fn decode(handle: u32) -> Option<(usize, u16)> {
    let low = handle & 0xFFFF;
    if low == 0 || low as usize > MAX_SLOTS {
        return None;
    }
    Some(((low - 1) as usize, (handle >> 16) as u16))
}

impl<T> Table<T> {
    /// Stores `value` and returns its handle; `None` if the table is full.
    pub fn insert(&mut self, value: T) -> Option<u32> {
        if let Some(index) = self.free.pop() {
            let slot = self.slots.get_mut(index)?;
            slot.value = Some(value);
            return Some(encode(index, slot.generation));
        }
        let index = self.slots.len();
        if index >= MAX_SLOTS {
            return None;
        }
        self.slots.push(Slot {
            generation: 0,
            value: Some(value),
        });
        Some(encode(index, 0))
    }

    fn slot(&self, handle: u32) -> Option<&Slot<T>> {
        let (index, generation) = decode(handle)?;
        self.slots
            .get(index)
            .filter(|s| s.generation == generation && s.value.is_some())
    }

    /// The entry of a live handle.
    pub fn get(&self, handle: u32) -> Option<&T> {
        self.slot(handle).and_then(|s| s.value.as_ref())
    }

    /// The entry of a live handle, mutably.
    pub fn get_mut(&mut self, handle: u32) -> Option<&mut T> {
        let (index, generation) = decode(handle)?;
        self.slots
            .get_mut(index)
            .filter(|s| s.generation == generation)
            .and_then(|s| s.value.as_mut())
    }

    /// Removes a live entry if `pred` accepts it; the handle becomes invalid.
    pub fn remove_if(&mut self, handle: u32, pred: impl FnOnce(&T) -> bool) -> Option<T> {
        let (index, generation) = decode(handle)?;
        let slot = self
            .slots
            .get_mut(index)
            .filter(|s| s.generation == generation)?;
        if !slot.value.as_ref().is_some_and(pred) {
            return None;
        }
        let value = slot.value.take();
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(index);
        value
    }

    /// Number of live entries.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.slots.len() - self.free.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_are_generational() {
        let mut t = Table::default();
        let a = t.insert('a').unwrap();
        let b = t.insert('b').unwrap();
        assert_ne!(a, 0);
        assert_ne!(a, b);
        assert_eq!(t.get(a), Some(&'a'));
        assert_eq!(t.remove_if(a, |_| true), Some('a'));
        assert_eq!(t.get(a), None);
        assert_eq!(t.remove_if(a, |_| true), None);
        let c = t.insert('c').unwrap();
        assert_eq!(c & 0xFFFF, a & 0xFFFF, "slot is reused");
        assert_ne!(c, a, "with a new generation");
        assert_eq!(t.get(a), None);
        assert_eq!(t.get(c), Some(&'c'));
        assert_eq!(t.len(), 2);
        assert_eq!(t.remove_if(b, |v| *v == 'x'), None, "predicate refuses");
        assert_eq!(t.get(b), Some(&'b'));
    }

    #[test]
    fn invalid_handles() {
        let mut t: Table<u8> = Table::default();
        assert_eq!(t.get(0), None);
        assert_eq!(t.get(u32::MAX), None);
        assert_eq!(t.get_mut(1), None);
        let h = t.insert(1).unwrap();
        assert_eq!(t.get(h ^ 0x1_0000), None, "wrong generation");
    }

    #[test]
    fn never_all_ones() {
        assert_eq!(decode(u32::MAX), None);
        assert_ne!(encode(MAX_SLOTS - 1, u16::MAX), u32::MAX);
    }
}

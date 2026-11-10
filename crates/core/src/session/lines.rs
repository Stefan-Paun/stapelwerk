//! Line counters: executions per source line of the player's file (file 0).

use crate::consts::TEXT_START;
use crate::machine::RunObserver;
use crate::program::Program;

/// Counts how often each line of the player's file was executed.
///
/// A line counts once per execution of its **first** instruction word, so `li` (two words) counts
/// once per pass. Library lines and the trap word are not counted. Also usable on its own as a
/// [`RunObserver`] (the test runner needs `hottest_line` for `run.budget_exceeded`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineCounter {
    /// Per instruction word from [`TEXT_START`]: the line it starts, 0 = none.
    slots: Vec<u32>,
    /// Executions per line (index = 1-based line number; index 0 unused).
    counts: Vec<u64>,
}

impl LineCounter {
    /// Counter for `program`, all counts 0.
    pub fn new(program: &Program) -> Self {
        let words = (program.text_len / 4) as usize;
        let mut slots = vec![0u32; words];
        let mut max_line = 0u32;
        let mut prev: Option<(u32, u32)> = None; // (addr, line) of the previous player entry
        for e in program.player_lines() {
            let first = !matches!(prev, Some((a, l)) if a + 4 == e.addr && l == e.line);
            prev = Some((e.addr, e.line));
            if !first {
                continue;
            }
            let i = (e.addr.wrapping_sub(TEXT_START) / 4) as usize;
            if let Some(s) = slots.get_mut(i) {
                *s = e.line;
                max_line = max_line.max(e.line);
            }
        }
        LineCounter {
            slots,
            counts: vec![0; max_line as usize + 1],
        }
    }

    /// Counts one execution of the instruction at `pc`.
    #[inline]
    pub fn count(&mut self, pc: u32) {
        let i = (pc.wrapping_sub(TEXT_START) / 4) as usize;
        if let Some(&line) = self.slots.get(i)
            && line != 0
            && let Some(c) = self.counts.get_mut(line as usize)
        {
            *c += 1;
        }
    }

    /// Sets all counts to 0.
    pub fn clear(&mut self) {
        self.counts.fill(0);
    }

    /// `[(line, count)]` for every line executed at least once, ascending by line.
    pub fn line_counts(&self) -> Vec<(u32, u64)> {
        self.counts
            .iter()
            .enumerate()
            .filter(|(_, c)| **c > 0)
            .map(|(l, c)| (l as u32, *c))
            .collect()
    }

    /// Executions of `line` so far.
    pub fn count_of(&self, line: u32) -> u64 {
        self.counts.get(line as usize).copied().unwrap_or(0)
    }

    /// The most executed line (smallest line on ties), 0 if no line ran
    /// (`run.budget_exceeded {hottest_line}`).
    pub fn hottest_line(&self) -> u32 {
        let mut best = (0u32, 0u64);
        for (l, &c) in self.counts.iter().enumerate() {
            if c > best.1 {
                best = (l as u32, c);
            }
        }
        best.0
    }

    /// Raw counts (for snapshots).
    pub(crate) fn raw(&self) -> &[u64] {
        &self.counts
    }

    /// Restores raw counts saved with [`LineCounter::raw`].
    pub(crate) fn restore_raw(&mut self, counts: &[u64]) {
        self.counts.clear();
        self.counts.extend_from_slice(counts);
    }
}

impl RunObserver for LineCounter {
    #[inline]
    fn executed(&mut self, pc: u32) {
        self.count(pc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::LineEntry;

    #[test]
    fn first_word_of_a_line_counts() {
        let entry = |addr, file, line| LineEntry { addr, file, line };
        let p = Program {
            words: vec![1; 5],
            text_len: 20,
            linemap: vec![
                entry(0x100, 0, 2),
                entry(0x104, 0, 3), // li: two words on line 3
                entry(0x108, 0, 3),
                entry(0x10C, 1, 1), // library
            ],
            ..Program::default()
        };
        let mut c = LineCounter::new(&p);
        for pc in [0x100, 0x104, 0x108, 0x10C, 0x104, 0x108, 0x110, 0x4000] {
            c.count(pc);
        }
        assert_eq!(c.line_counts(), vec![(2, 1), (3, 2)]);
        assert_eq!(c.hottest_line(), 3);
        c.clear();
        assert_eq!(c.hottest_line(), 0);
    }
}

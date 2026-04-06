use std::collections::VecDeque;

/// Input count: 256 bytes + 1 EOI sentinel.
pub(crate) const INPUT_COUNT: usize = 257;

/// A minimal DFA for set-algebraic operations.
///
/// States are numbered 0..num_states. State 0 is the dead state
/// (all transitions loop to itself, not accepting by default).
#[derive(Debug, Clone)]
pub struct Dfa {
    /// Flat transition table: trans[state * INPUT_COUNT + input] = next_state.
    /// Input 0-255 = byte values, input 256 = EOI.
    pub(crate) trans: Vec<u32>,
    pub(crate) num_states: u32,
    pub(crate) start: u32,
    /// Whether each state is accepting (checked after all input + EOI).
    pub(crate) is_match: Vec<bool>,
}

impl Dfa {
    /// Check if the DFA accepts any string at all.
    /// BFS from start state; returns false if any reachable state is accepting.
    pub fn is_empty(&self) -> bool {
        let mut visited = vec![false; self.num_states as usize];
        let mut queue = VecDeque::new();

        visited[self.start as usize] = true;
        queue.push_back(self.start);

        // Check start state.
        if self.is_match[self.start as usize] {
            return false;
        }

        while let Some(state) = queue.pop_front() {
            // Explore byte transitions (0..256, not EOI).
            for input in 0..256usize {
                let next = self.trans[state as usize * INPUT_COUNT + input];
                if !visited[next as usize] {
                    visited[next as usize] = true;
                    if self.is_match[next as usize] {
                        return false;
                    }
                    queue.push_back(next);
                }
            }
        }

        true
    }

    /// Check if a byte string is accepted by this DFA.
    pub fn matches(&self, input: &[u8]) -> bool {
        let mut state = self.start;
        for &byte in input {
            state = self.trans[state as usize * INPUT_COUNT + byte as usize];
        }
        self.is_match[state as usize]
    }
}

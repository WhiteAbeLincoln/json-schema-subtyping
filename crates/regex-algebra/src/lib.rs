mod dfa;

use regex_automata::{
    dfa::{dense, Automaton},
    Anchored, Input,
};
use std::collections::{HashMap, VecDeque};

pub use dfa::Dfa;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to compile regex pattern: {0}")]
    Compile(String),
    #[error("unsupported regex feature: {0}")]
    Unsupported(String),
}

/// A compiled regex suitable for set-algebraic operations.
///
/// Backed by a minimal DFA with full-string (anchored) matching semantics.
/// All operations (intersection, complement, union, subset, emptiness)
/// are exact — no approximations.
#[derive(Debug, Clone)]
pub struct CompiledRegex {
    dfa: Dfa,
}

/// Compile a regex pattern into a DFA for set operations.
///
/// The pattern is automatically anchored for full-string matching:
/// `pattern` becomes `^(?:pattern)$`. If you need substring matching,
/// wrap the pattern yourself (e.g., `".*pattern.*"`).
pub fn compile(pattern: &str) -> Result<CompiledRegex, Error> {
    // Anchor the pattern for full-string matching.
    // (?s) enables dotall so `.` matches newlines.
    let anchored = format!("(?s)^(?:{pattern})$");

    let ra_dfa = dense::DFA::new(&anchored).map_err(|e| Error::Compile(e.to_string()))?;

    let dfa = convert_from_regex_automata(&ra_dfa)?;
    Ok(CompiledRegex { dfa })
}

/// Intersection of two regexes: L(result) = L(a) ∩ L(b).
pub fn intersect(a: &CompiledRegex, b: &CompiledRegex) -> CompiledRegex {
    CompiledRegex {
        dfa: product(&a.dfa, &b.dfa, |a_match, b_match| a_match && b_match),
    }
}

/// Union of two regexes: L(result) = L(a) ∪ L(b).
pub fn union(a: &CompiledRegex, b: &CompiledRegex) -> CompiledRegex {
    CompiledRegex {
        dfa: product(&a.dfa, &b.dfa, |a_match, b_match| a_match || b_match),
    }
}

/// Complement of a regex: L(result) = Σ* \ L(a).
pub fn complement(a: &CompiledRegex) -> CompiledRegex {
    let mut dfa = a.dfa.clone();
    // Flip accepting states (except the dead state, which stays non-accepting
    // for the purpose of our DFA — but actually for complement, dead states
    // should become accepting since they represent strings the original rejects).
    for accept in &mut dfa.is_match {
        *accept = !*accept;
    }
    // The dead state (0) in complement should be accepting (it means
    // "the original DFA had no valid transition" = "original rejects").
    // This is already handled by the flip above.
    CompiledRegex { dfa }
}

/// Check if L(a) ⊆ L(b): every string matched by `a` is also matched by `b`.
pub fn is_subset(a: &CompiledRegex, b: &CompiledRegex) -> bool {
    // L(a) ⊆ L(b) iff L(a) ∩ L(¬b) = ∅
    let b_comp = complement(b);
    let diff = intersect(a, &b_comp);
    is_empty(&diff)
}

/// Check if the regex matches no strings at all.
pub fn is_empty(a: &CompiledRegex) -> bool {
    a.dfa.is_empty()
}

/// Check if a regex matches a specific string.
pub fn matches(a: &CompiledRegex, s: &str) -> bool {
    a.dfa.matches(s.as_bytes())
}

/// Build a pattern string for length constraints: matches strings of length [min, max].
pub fn length_pattern(min: u64, max: Option<u64>) -> String {
    match max {
        Some(m) => format!(".{{{min},{m}}}"),
        None => format!(".{{{min},}}"),
    }
}

// --- Internal: convert regex-automata DFA to our format ---

/// Input count: 256 bytes + 1 EOI symbol.
const INPUT_COUNT: usize = 257;
const EOI_INPUT: usize = 256;

fn convert_from_regex_automata(
    ra_dfa: &dense::DFA<Vec<u32>>,
) -> Result<Dfa, Error> {
    // Get the anchored start state.
    let input = Input::new("").anchored(Anchored::Yes);
    let ra_start = ra_dfa
        .start_state_forward(&input)
        .map_err(|e| Error::Compile(e.to_string()))?;

    // BFS to discover all reachable states.
    let mut state_map: HashMap<regex_automata::util::primitives::StateID, u32> = HashMap::new();
    let mut queue: VecDeque<regex_automata::util::primitives::StateID> = VecDeque::new();

    // Reserve state 0 as our dead state.
    let dead_id: u32 = 0;
    let mut next_id: u32 = 1;

    // Map the start state.
    let start_id = next_id;
    state_map.insert(ra_start, start_id);
    queue.push_back(ra_start);
    next_id += 1;

    // We'll collect transitions as we go: (our_state, input, our_next_state)
    // But it's easier to build the table after we know all states.
    // First pass: discover all reachable states.
    let mut transitions_raw: Vec<(u32, usize, u32)> = Vec::new();
    let mut match_states: Vec<(u32, bool)> = Vec::new();

    // Dead state transitions: all go to dead state, not accepting.
    for i in 0..INPUT_COUNT {
        transitions_raw.push((dead_id, i, dead_id));
    }
    match_states.push((dead_id, false));

    while let Some(ra_state) = queue.pop_front() {
        let our_state = state_map[&ra_state];

        // Record if this is a match state (after EOI).
        let eoi_state = ra_dfa.next_eoi_state(ra_state);
        let is_match = ra_dfa.is_match_state(eoi_state);
        match_states.push((our_state, is_match));

        // Compute transitions for all 256 bytes.
        for byte in 0u16..=255 {
            let ra_next = ra_dfa.next_state(ra_state, byte as u8);
            let next_id_val = if ra_dfa.is_dead_state(ra_next) {
                dead_id
            } else {
                *state_map.entry(ra_next).or_insert_with(|| {
                    let id = next_id;
                    next_id += 1;
                    queue.push_back(ra_next);
                    id
                })
            };
            transitions_raw.push((our_state, byte as usize, next_id_val));
        }

        // EOI transition: after EOI, go to a state that reflects match status.
        // For our algebra, EOI transitions lead to "terminal" states.
        // We handle this by checking match status at the current state after EOI.
        // The EOI transition in our DFA goes to dead (we only care about is_match above).
        transitions_raw.push((our_state, EOI_INPUT, dead_id));
    }

    let num_states = next_id;
    let mut trans = vec![0u32; num_states as usize * INPUT_COUNT];
    for (state, input, next) in &transitions_raw {
        trans[*state as usize * INPUT_COUNT + input] = *next;
    }

    let mut is_match_vec = vec![false; num_states as usize];
    for (state, is_m) in &match_states {
        is_match_vec[*state as usize] = *is_m;
    }

    Ok(Dfa {
        trans,
        num_states,
        start: start_id,
        is_match: is_match_vec,
    })
}

// --- Internal: DFA product construction ---

fn product(a: &Dfa, b: &Dfa, accept: impl Fn(bool, bool) -> bool) -> Dfa {
    // Product state = (state_a, state_b).
    // BFS from (a.start, b.start).
    let mut state_map: HashMap<(u32, u32), u32> = HashMap::new();
    let mut queue: VecDeque<(u32, u32)> = VecDeque::new();

    // Dead state 0.
    let dead_id: u32 = 0;
    state_map.insert((0, 0), dead_id);

    let start_pair = (a.start, b.start);
    let start_id: u32 = 1;
    let mut next_id: u32 = 2;
    state_map.insert(start_pair, start_id);
    queue.push_back(start_pair);

    let mut trans_data: Vec<(u32, usize, u32)> = Vec::new();
    let mut match_data: Vec<(u32, bool)> = Vec::new();

    // Dead state transitions.
    let dead_accept = accept(
        a.is_match.first().copied().unwrap_or(false),
        b.is_match.first().copied().unwrap_or(false),
    );
    match_data.push((dead_id, dead_accept));
    for i in 0..INPUT_COUNT {
        trans_data.push((dead_id, i, dead_id));
    }

    while let Some((sa, sb)) = queue.pop_front() {
        let our_id = state_map[&(sa, sb)];
        let a_match = a.is_match[sa as usize];
        let b_match = b.is_match[sb as usize];
        match_data.push((our_id, accept(a_match, b_match)));

        // Only compute byte transitions (not EOI — we use is_match directly).
        for input in 0..256usize {
            let na = a.trans[sa as usize * INPUT_COUNT + input];
            let nb = b.trans[sb as usize * INPUT_COUNT + input];

            // If both reach dead, product is dead.
            let next_pair = (na, nb);
            let next_id_val = *state_map.entry(next_pair).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                queue.push_back(next_pair);
                id
            });
            trans_data.push((our_id, input, next_id_val));
        }

        // EOI goes to dead (match status already recorded).
        trans_data.push((our_id, EOI_INPUT, dead_id));
    }

    let num_states = next_id;
    let mut trans = vec![0u32; num_states as usize * INPUT_COUNT];
    for (state, input, next) in &trans_data {
        trans[*state as usize * INPUT_COUNT + input] = *next;
    }

    let mut is_match = vec![false; num_states as usize];
    for (state, m) in &match_data {
        is_match[*state as usize] = *m;
    }

    Dfa {
        trans,
        num_states,
        start: start_id,
        is_match,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_match() {
        let r = compile("abc").unwrap();
        assert!(matches(&r, "abc"));
        assert!(!matches(&r, "ab"));
        assert!(!matches(&r, "abcd"));
        assert!(!matches(&r, ""));
    }

    #[test]
    fn dotstar() {
        let r = compile(".*").unwrap();
        assert!(matches(&r, ""));
        assert!(matches(&r, "anything"));
    }

    #[test]
    fn character_class() {
        let r = compile("[a-z]+").unwrap();
        assert!(matches(&r, "abc"));
        assert!(!matches(&r, "ABC"));
        assert!(!matches(&r, ""));
        assert!(!matches(&r, "abc123"));
    }

    #[test]
    fn test_intersection() {
        let a = compile(".{3,}").unwrap();
        let b = compile("[a-z]+").unwrap();
        let inter = intersect(&a, &b);
        assert!(matches(&inter, "abc"));
        assert!(!matches(&inter, "ab")); // too short
        assert!(!matches(&inter, "ABC")); // wrong chars
        assert!(!matches(&inter, "abc1")); // non-alpha
    }

    #[test]
    fn test_complement() {
        let a = compile("abc").unwrap();
        let comp = complement(&a);
        assert!(!matches(&comp, "abc"));
        assert!(matches(&comp, "def"));
        assert!(matches(&comp, "ab"));
        assert!(matches(&comp, ""));
    }

    #[test]
    fn test_union() {
        let a = compile("abc").unwrap();
        let b = compile("def").unwrap();
        let u = union(&a, &b);
        assert!(matches(&u, "abc"));
        assert!(matches(&u, "def"));
        assert!(!matches(&u, "ghi"));
    }

    #[test]
    fn test_subset() {
        let sub = compile("[a-z]{3}").unwrap();
        let sup = compile("[a-z]+").unwrap();
        assert!(is_subset(&sub, &sup));
        assert!(!is_subset(&sup, &sub));
    }

    #[test]
    fn test_empty() {
        // Intersection of disjoint patterns is empty.
        let a = compile("abc").unwrap();
        let b = compile("def").unwrap();
        let inter = intersect(&a, &b);
        assert!(is_empty(&inter));
    }

    #[test]
    fn test_non_empty() {
        let a = compile("[a-z]+").unwrap();
        assert!(!is_empty(&a));
    }

    #[test]
    fn test_length_pattern() {
        let p = length_pattern(2, Some(5));
        let r = compile(&p).unwrap();
        assert!(!matches(&r, "a"));
        assert!(matches(&r, "ab"));
        assert!(matches(&r, "abcde"));
        assert!(!matches(&r, "abcdef"));
    }

    #[test]
    fn test_length_unbounded() {
        let p = length_pattern(3, None);
        let r = compile(&p).unwrap();
        assert!(!matches(&r, "ab"));
        assert!(matches(&r, "abc"));
        assert!(matches(&r, "abcdefghij"));
    }

    #[test]
    fn complement_complement_is_identity() {
        let a = compile("[0-9]+").unwrap();
        let cc = complement(&complement(&a));
        assert!(matches(&cc, "123"));
        assert!(!matches(&cc, "abc"));
    }

    #[test]
    fn intersection_with_complement_is_difference() {
        let all_alpha = compile("[a-z]+").unwrap();
        let three_chars = compile(".{3}").unwrap();
        // [a-z]+ minus .{3} = [a-z]+ that's NOT exactly 3 chars
        let diff = intersect(&all_alpha, &complement(&three_chars));
        assert!(matches(&diff, "ab"));
        assert!(!matches(&diff, "abc")); // exactly 3 chars, excluded
        assert!(matches(&diff, "abcd"));
    }

    #[test]
    fn de_morgan_intersection() {
        // ¬(A ∪ B) = ¬A ∩ ¬B
        let a = compile("abc").unwrap();
        let b = compile("def").unwrap();
        let lhs = complement(&union(&a, &b));
        let rhs = intersect(&complement(&a), &complement(&b));

        // Test on a sample of strings.
        for s in &["abc", "def", "ghi", "", "ab", "abcdef"] {
            assert_eq!(
                matches(&lhs, s),
                matches(&rhs, s),
                "De Morgan failed for {s:?}"
            );
        }
    }
}

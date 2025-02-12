/// Construct an Index.
use crate::prelude::{State, TransitionKey};
use crate::regex::{get_vocabulary_transition_keys, state_scan_tokens};
use crate::vocabulary::Vocabulary;
use std::collections::{HashMap, HashSet};

pub type Result<T, E = crate::Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct FSMInfo {
    pub(crate) initial: State,
    pub(crate) finals: HashSet<State>,
    pub(crate) transitions: HashMap<(State, TransitionKey), State>,
    pub(crate) alphabet_anything_value: TransitionKey,
    pub(crate) alphabet_symbol_mapping: HashMap<String, TransitionKey>,
}

impl FSMInfo {
    pub fn new(
        initial: State,
        finals: HashSet<State>,
        transitions: HashMap<(State, TransitionKey), State>,
        alphabet_anything_value: TransitionKey,
        alphabet_symbol_mapping: HashMap<String, TransitionKey>,
    ) -> Self {
        Self {
            initial,
            finals,
            transitions,
            alphabet_anything_value,
            alphabet_symbol_mapping,
        }
    }
}

#[derive(Debug)]
pub struct Index {
    pub(crate) initial: u32,
    finals: HashSet<u32>,
    states_to_token_subsets: HashMap<u32, HashMap<u32, u32>>,
    eos_token_id: u32,
}

impl Index {
    pub fn new(
        fsm_info: &FSMInfo,
        vocabulary: &Vocabulary,
        eos_token_id: u32,
        frozen_tokens: HashSet<String>,
    ) -> Result<Self> {
        let mut states_to_token_subsets: HashMap<u32, HashMap<u32, u32>> = HashMap::new();
        let mut seen: HashSet<State> = HashSet::new();
        let mut next_states: HashSet<State> = HashSet::from([fsm_info.initial]);

        let vocabulary_transition_keys = get_vocabulary_transition_keys(
            &fsm_info.alphabet_symbol_mapping,
            fsm_info.alphabet_anything_value,
            vocabulary,
            &frozen_tokens,
        );

        while let Some(start_state) = next_states.iter().cloned().next() {
            next_states.remove(&start_state);

            let token_ids_end_states = state_scan_tokens(
                &fsm_info.transitions,
                fsm_info.initial,
                &fsm_info.finals,
                vocabulary,
                &vocabulary_transition_keys,
                start_state,
            );

            for (token_id, end_state) in &token_ids_end_states {
                let inner_map = states_to_token_subsets.entry(start_state).or_default();
                inner_map.insert(*token_id, *end_state);

                if !seen.contains(end_state) {
                    next_states.insert(*end_state);
                }
            }

            if fsm_info.finals.contains(&start_state) && !token_ids_end_states.is_empty() {
                let inner_map = states_to_token_subsets.entry(start_state).or_default();
                inner_map.insert(eos_token_id, start_state);
            }

            seen.insert(start_state);
        }

        let is_valid = states_to_token_subsets
            .values()
            .flat_map(|token_id_end_states| token_id_end_states.values())
            .any(|end_state| fsm_info.finals.contains(end_state));

        if is_valid {
            Ok(Self {
                initial: fsm_info.initial,
                finals: fsm_info.finals.clone(),
                states_to_token_subsets,
                eos_token_id,
            })
        } else {
            Err(crate::Error::IndexError)
        }
    }

    pub(crate) fn allowed_tokens(&self, state: u32) -> Option<Vec<u32>> {
        self.states_to_token_subsets
            .get(&state)
            .map_or_else(|| None, |res| Some(res.keys().cloned().collect()))
    }

    pub(crate) fn next_state(&self, state: u32, token_id: u32) -> Option<u32> {
        if token_id == self.eos_token_id {
            return None;
        }
        Some(*self.states_to_token_subsets.get(&state)?.get(&token_id)?)
    }

    pub(crate) fn initial(&self) -> u32 {
        self.initial
    }

    pub(crate) fn is_final(&self, state: u32) -> bool {
        self.finals.contains(&state)
    }

    pub(crate) fn index(&self) -> &HashMap<u32, HashMap<u32, u32>> {
        &self.states_to_token_subsets
    }
}

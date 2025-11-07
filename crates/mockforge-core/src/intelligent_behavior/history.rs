//! Undo/redo history manager for state machine edits
//!
//! Provides history tracking for state machine modifications, enabling undo/redo
//! functionality in visual editors.

use crate::intelligent_behavior::rules::StateMachine;
use std::collections::VecDeque;

/// History manager for state machine edits
///
/// Maintains undo and redo stacks for state machine modifications.
/// Supports configurable maximum history size to limit memory usage.
#[derive(Debug, Clone)]
pub struct HistoryManager {
    /// Stack of previous states (for undo)
    undo_stack: VecDeque<StateMachine>,

    /// Stack of future states (for redo)
    redo_stack: VecDeque<StateMachine>,

    /// Maximum number of history entries to keep
    max_history: usize,

    /// Current state (not yet committed to history)
    current: Option<StateMachine>,
}

impl HistoryManager {
    /// Create a new history manager with default max history (50)
    pub fn new() -> Self {
        Self::with_max_history(50)
    }

    /// Create a new history manager with specified max history
    pub fn with_max_history(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
            max_history,
            current: None,
        }
    }

    /// Push a new state to the history
    ///
    /// If there's a current state, it's moved to the undo stack.
    /// The new state becomes the current state.
    pub fn push_state(&mut self, state: StateMachine) {
        // If we have a current state, move it to undo stack
        if let Some(current) = self.current.take() {
            // Limit undo stack size
            if self.undo_stack.len() >= self.max_history {
                self.undo_stack.pop_front();
            }
            self.undo_stack.push_back(current);
        }

        // Clear redo stack when new state is pushed
        self.redo_stack.clear();

        // Set new current state
        self.current = Some(state);
    }

    /// Undo the last change
    ///
    /// Moves the current state to the redo stack and restores the previous
    /// state from the undo stack. Returns the restored state if available.
    pub fn undo(&mut self) -> Option<StateMachine> {
        if self.undo_stack.is_empty() {
            return None;
        }

        // Move current to redo stack
        if let Some(current) = self.current.take() {
            // Limit redo stack size
            if self.redo_stack.len() >= self.max_history {
                self.redo_stack.pop_front();
            }
            self.redo_stack.push_back(current);
        }

        // Restore from undo stack
        let restored = self.undo_stack.pop_back()?;
        self.current = Some(restored.clone());
        Some(restored)
    }

    /// Redo the last undone change
    ///
    /// Moves the current state to the undo stack and restores the next
    /// state from the redo stack. Returns the restored state if available.
    pub fn redo(&mut self) -> Option<StateMachine> {
        if self.redo_stack.is_empty() {
            return None;
        }

        // Move current to undo stack
        if let Some(current) = self.current.take() {
            // Limit undo stack size
            if self.undo_stack.len() >= self.max_history {
                self.undo_stack.pop_front();
            }
            self.undo_stack.push_back(current);
        }

        // Restore from redo stack
        let restored = self.redo_stack.pop_back()?;
        self.current = Some(restored.clone());
        Some(restored)
    }

    /// Check if undo is possible
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is possible
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the current state
    pub fn current(&self) -> Option<&StateMachine> {
        self.current.as_ref()
    }

    /// Get the current state mutably
    pub fn current_mut(&mut self) -> Option<&mut StateMachine> {
        self.current.as_mut()
    }

    /// Set the current state without adding to history
    ///
    /// Useful for temporary edits that shouldn't be tracked.
    pub fn set_current(&mut self, state: StateMachine) {
        self.current = Some(state);
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current = None;
    }

    /// Get the number of undo steps available
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of redo steps available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Get the maximum history size
    pub fn max_history(&self) -> usize {
        self.max_history
    }

    /// Set the maximum history size
    ///
    /// If the new max is smaller than current history, older entries are removed.
    pub fn set_max_history(&mut self, max_history: usize) {
        self.max_history = max_history;

        // Trim undo stack if needed
        while self.undo_stack.len() > max_history {
            self.undo_stack.pop_front();
        }

        // Trim redo stack if needed
        while self.redo_stack.len() > max_history {
            self.redo_stack.pop_front();
        }
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligent_behavior::rules::{StateMachine, StateTransition};

    fn create_test_machine(name: &str) -> StateMachine {
        StateMachine::new(
            format!("resource_{}", name),
            vec!["state1".to_string(), "state2".to_string()],
            "state1",
        )
        .add_transition(StateTransition::new("state1", "state2"))
    }

    #[test]
    fn test_history_manager_creation() {
        let manager = HistoryManager::new();
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_push_and_undo() {
        let mut manager = HistoryManager::new();
        let state1 = create_test_machine("1");
        let state2 = create_test_machine("2");

        manager.push_state(state1.clone());
        manager.push_state(state2.clone());

        assert!(manager.can_undo());
        let restored = manager.undo().unwrap();
        assert_eq!(restored.resource_type, state1.resource_type);
    }

    #[test]
    fn test_undo_redo() {
        let mut manager = HistoryManager::new();
        let state1 = create_test_machine("1");
        let state2 = create_test_machine("2");

        manager.push_state(state1.clone());
        manager.push_state(state2.clone());

        // Undo
        let restored = manager.undo().unwrap();
        assert_eq!(restored.resource_type, state1.resource_type);

        // Redo
        let restored = manager.redo().unwrap();
        assert_eq!(restored.resource_type, state2.resource_type);
    }

    #[test]
    fn test_max_history() {
        let mut manager = HistoryManager::with_max_history(3);

        for i in 0..5 {
            manager.push_state(create_test_machine(&i.to_string()));
        }

        // Should only keep last 3 in undo stack
        assert_eq!(manager.undo_count(), 3);
    }

    #[test]
    fn test_clear_redo_on_new_push() {
        let mut manager = HistoryManager::new();
        let state1 = create_test_machine("1");
        let state2 = create_test_machine("2");
        let state3 = create_test_machine("3");

        manager.push_state(state1);
        manager.push_state(state2);
        manager.undo(); // Now we can redo
        assert!(manager.can_redo());

        manager.push_state(state3); // This should clear redo
        assert!(!manager.can_redo());
    }
}

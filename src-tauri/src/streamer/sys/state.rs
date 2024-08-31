use std::fmt::Debug;

use gstreamer_sys::GstState;

#[derive(Debug)]
pub(crate) struct State {
    old_state: GstState,
    new_state: GstState,
    pending_state: GstState,
}

impl State {
    pub(crate) fn new(old_state: GstState, new_state: GstState, pending_state: GstState) -> Self {
        Self {
            old_state,
            new_state,
            pending_state,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn old_state(&self) -> GstState {
        self.old_state
    }

    #[allow(dead_code)]
    pub(crate) fn new_state(&self) -> GstState {
        self.new_state
    }

    #[allow(dead_code)]
    pub(crate) fn pending_state(&self) -> GstState {
        self.pending_state
    }
}

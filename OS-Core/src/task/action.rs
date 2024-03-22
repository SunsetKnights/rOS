use super::signal::{SignalFlags, MAX_SIG};

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct SignalAction {
    pub handler: usize,
    pub mask: SignalFlags,
}
impl SignalAction {
    pub fn default() -> Self {
        Self {
            handler: 0,
            mask: SignalFlags::from_bits(40).unwrap(), //SIGQUIT SIGTRAP
        }
    }
}

pub struct SignalActions {
    pub table: [SignalAction; MAX_SIG + 1],
}

impl SignalActions {
    pub fn default() -> Self {
        Self {
            table: [SignalAction::default(); MAX_SIG + 1],
        }
    }
}

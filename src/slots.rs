//! Panel slot identifiers.

/// Identifies one of the four panel slots.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[repr(usize)]
pub(crate) enum SlotId {
    Left = 0,
    Center = 1,
    Tools = 2,
    Bottom = 3,
}

/// Inserts text at the cursor of the focused app on the computer.
///
/// The real implementation (clipboard save, set text, simulated Cmd+V / Ctrl+V, clipboard restore)
/// arrives with the paste ticket; tests use [`crate::testing::RecordingPaster`].
pub trait Paster: Send + Sync {
    fn paste(&self, text: &str);
}

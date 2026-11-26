//! Pasting: inserting an Invocation at the cursor of the focused app on the computer.
//!
//! Every paste goes through one [`PasteQueue`], so taps from several phones are pasted one at
//! a time in arrival order and never interleave.

use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::Duration;

/// Inserts text at the cursor of the focused app on the computer.
///
/// The real implementation is [`SystemPaster`]; tests use [`crate::testing::RecordingPaster`].
pub trait Paster: Send + Sync {
    fn paste(&self, text: &str);

    /// Presses Enter in the focused app. Used after a paste when "Send Enter" is on.
    fn press_enter(&self) {}
}

/// The Pad window as seen by the paste flow. The shell implements it so the Pad gets out of
/// the way before a paste, letting focus return to the user's app.
pub trait PadWindow: Send + Sync {
    /// Hides the Pad if it is visible. Returns whether it was visible.
    fn hide_if_visible(&self) -> bool;
}

/// Used until the shell registers its window, and in tests.
pub(crate) struct NoPadWindow;

impl PadWindow for NoPadWindow {
    fn hide_if_visible(&self) -> bool {
        false
    }
}

/// How long to wait after hiding the Pad for focus to return to the previous app.
const FOCUS_RETURN_DELAY: Duration = Duration::from_millis(150);

struct Job {
    text: String,
    enter: bool,
    done: Sender<()>,
}

/// One worker thread that performs pastes in arrival order.
pub(crate) struct PasteQueue {
    jobs: Mutex<Sender<Job>>,
}

impl PasteQueue {
    pub fn start(paster: Arc<dyn Paster>, pad: Arc<RwLock<Arc<dyn PadWindow>>>) -> Self {
        let (jobs, rx): (Sender<Job>, Receiver<Job>) = mpsc::channel();
        thread::Builder::new()
            .name("agent-pad-paste".into())
            .spawn(move || {
                // Ends when the queue (and so the last sender) is dropped.
                for job in rx {
                    let pad = pad.read().unwrap().clone();
                    if pad.hide_if_visible() {
                        thread::sleep(FOCUS_RETURN_DELAY);
                    }
                    paster.paste(&job.text);
                    if job.enter {
                        paster.press_enter();
                    }
                    let _ = job.done.send(());
                }
            })
            .expect("spawn paste thread");
        Self {
            jobs: Mutex::new(jobs),
        }
    }

    /// Queues `text` (followed by Enter if `enter`) and blocks until it has been pasted.
    pub fn paste(&self, text: String, enter: bool) {
        let (done, wait) = mpsc::channel();
        let sent = self.jobs.lock().unwrap().send(Job { text, enter, done });
        if sent.is_ok() {
            let _ = wait.recv();
        }
    }
}

/// Pastes through the system clipboard and a simulated Cmd+V (macOS) or Ctrl+V (Windows),
/// then restores the previous clipboard text.
///
/// Only text clipboard contents are restored; an image on the clipboard is lost.
/// On macOS this needs Accessibility permission (ticket 06 guides the user to grant it).
pub struct SystemPaster;

/// Time for the focused app to read the clipboard before it is restored.
const CLIPBOARD_SETTLE: Duration = Duration::from_millis(120);

impl Paster for SystemPaster {
    fn paste(&self, text: &str) {
        if let Err(err) = system_paste(text) {
            eprintln!("agent-pad: paste failed: {err}");
        }
    }

    fn press_enter(&self) {
        use enigo::{Direction, Enigo, Key, Keyboard, Settings};
        let pressed = Enigo::new(&Settings::default())
            .map_err(|e| e.to_string())
            .and_then(|mut e| {
                e.key(Key::Return, Direction::Click)
                    .map_err(|e| e.to_string())
            });
        if let Err(err) = pressed {
            eprintln!("agent-pad: Enter failed: {err}");
        }
    }
}

fn system_paste(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut clipboard = arboard::Clipboard::new()?;
    let previous = clipboard.get_text().ok();
    clipboard.set_text(text)?;

    let modifier = if cfg!(target_os = "macos") {
        Key::Meta
    } else {
        Key::Control
    };
    let mut enigo = Enigo::new(&Settings::default())?;
    enigo.key(modifier, Direction::Press)?;
    let clicked = enigo.key(Key::Unicode('v'), Direction::Click);
    enigo.key(modifier, Direction::Release)?;
    clicked?;

    thread::sleep(CLIPBOARD_SETTLE);
    if let Some(previous) = previous {
        clipboard.set_text(previous)?;
    }
    Ok(())
}

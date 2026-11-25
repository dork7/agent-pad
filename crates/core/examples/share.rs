//! Manual check for ticket 04: share real skills to a phone and paste for real.
//!
//! Run `cargo run -p skills-pad-core --example share`, scan the printed URL (or open it on the
//! phone), focus a text field on the computer and tap a skill. On macOS the terminal running
//! this needs Accessibility permission (System Settings > Privacy & Security > Accessibility).

use std::sync::Arc;

use skills_pad_core::{Core, CoreConfig, SystemClock, SystemPaster};

fn main() -> std::io::Result<()> {
    let home = std::env::var_os("HOME").expect("HOME is set").into();
    let core = Core::start(CoreConfig {
        home,
        paster: Arc::new(SystemPaster),
        clock: Arc::new(SystemClock),
    })?;
    let info = core.start_sharing()?;
    println!("Open on your phone: {}", info.url);
    println!("Sharing stops after 15 minutes without phone activity. Ctrl+C to quit.");
    while core.sharing_status().is_some() {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    println!("Sharing stopped.");
    Ok(())
}

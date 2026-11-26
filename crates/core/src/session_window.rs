//! Conservative window-level matching. A process may own several tabs or panes;
//! only a single window belonging to an ancestor host can be raised safely.

use crate::WindowTarget;

pub(crate) fn raise(target: &WindowTarget) -> bool {
    if target.process_id == 0
        || !target.project_folder.is_absolute()
        || !target.project_folder.is_dir()
    {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        return macos::raise(target.process_id);
    }
    #[cfg(target_os = "windows")]
    {
        return windows::raise(target.process_id);
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::process::Command;

    pub(super) fn raise(mut pid: u32) -> bool {
        for _ in 0..16 {
            // System Events addresses the window by the host process's Unix PID.
            // Never choose among several windows of one terminal or editor.
            let script = format!(
                "tell application \"System Events\"\n\
                 set matches to every process whose unix id is {pid}\n\
                 if (count of matches) is 1 then\n\
                   set host to item 1 of matches\n\
                   if (count of windows of host) is 1 then\n\
                     perform action \"AXRaise\" of window 1 of host\n\
                     set frontmost of host to true\n\
                     return \"raised\"\n\
                   end if\n\
                 end if\n\
                 end tell"
            );
            if Command::new("/usr/bin/osascript")
                .arg("-e")
                .arg(script)
                .output()
                .is_ok_and(|output| {
                    output.status.success()
                        && String::from_utf8_lossy(&output.stdout).trim() == "raised"
                })
            {
                return true;
            }
            let Ok(output) = Command::new("/bin/ps")
                .args(["-o", "ppid=", "-p", &pid.to_string()])
                .output()
            else {
                break;
            };
            let Some(parent) = String::from_utf8_lossy(&output.stdout)
                .trim()
                .parse::<u32>()
                .ok()
            else {
                break;
            };
            if parent <= 1 || parent == pid {
                break;
            }
            pid = parent;
        }
        false
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::process::Command;

    pub(super) fn raise(pid: u32) -> bool {
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                include_str!("session_window.ps1"),
                &pid.to_string(),
            ])
            .output()
            .is_ok_and(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).trim() == "raised"
            })
    }
}

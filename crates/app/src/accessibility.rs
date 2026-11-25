#[tauri::command]
pub fn accessibility_granted() -> bool {
    #[cfg(target_os = "macos")]
    {
        // The C wrapper calls AXIsProcessTrusted without prompting.
        unsafe { skills_pad_accessibility_granted() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub fn require_access(granted: bool) -> Result<(), String> {
    if granted {
        Ok(())
    } else {
        Err("Grant Accessibility access in the Pad before sharing.".into())
    }
}

#[tauri::command]
pub fn open_accessibility_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if !accessibility_granted() {
            // Prompt only on request so macOS adds the app to the Accessibility list.
            unsafe { skills_pad_prompt_for_accessibility() };
        }

        let status = std::process::Command::new("/usr/bin/open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err(format!("System Settings could not be opened ({status})."));
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn skills_pad_accessibility_granted() -> bool;
    fn skills_pad_prompt_for_accessibility();
}

#[cfg(test)]
mod tests {
    use super::require_access;

    #[test]
    fn sharing_requires_accessibility_access() {
        assert!(require_access(true).is_ok());
        assert!(require_access(false).unwrap_err().contains("Accessibility"));
    }
}

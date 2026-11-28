use std::path::Path;

pub const DEFAULT_VOLUME: u8 = 10;
pub const MAX_VOLUME: u8 = 25;

const ALIAS: &str = "sunline_menu_music";

#[derive(Default)]
pub struct MenuMusic {
    open: bool,
    volume: u8,
}

impl MenuMusic {
    pub fn is_playing(&self) -> bool {
        self.open
    }

    pub fn play(&mut self, path: &Path, volume: u8) -> Result<(), u32> {
        self.stop();
        let command = open_command(path)?;
        send(&command)?;
        self.open = true;
        if let Err(code) = self
            .apply_volume(volume)
            .and_then(|()| send(&format!("play {ALIAS} from 0")))
        {
            self.stop();
            return Err(code);
        }
        Ok(())
    }

    pub fn set_volume(&mut self, volume: u8) -> Result<(), u32> {
        if self.open && self.volume != volume.min(MAX_VOLUME) {
            self.apply_volume(volume)?;
        }
        Ok(())
    }

    fn apply_volume(&mut self, volume: u8) -> Result<(), u32> {
        send(&format!(
            "setaudio {ALIAS} volume to {} wait",
            mci_volume(volume)
        ))?;
        self.volume = volume.min(MAX_VOLUME);
        Ok(())
    }

    pub fn keep_looping(&mut self) -> Result<(), u32> {
        if self.open && send(&format!("status {ALIAS} mode"))?.eq_ignore_ascii_case("stopped") {
            send(&format!("play {ALIAS} from 0"))?;
        }
        Ok(())
    }

    pub fn stop(&mut self) {
        if self.open {
            let _ = send(&format!("close {ALIAS} wait"));
            self.open = false;
        }
    }
}

impl Drop for MenuMusic {
    fn drop(&mut self) {
        self.stop();
    }
}

fn mci_volume(volume: u8) -> u32 {
    u32::from(volume.min(MAX_VOLUME)) * 10
}

fn open_command(path: &Path) -> Result<String, u32> {
    const INVALID_FILE: u32 = 296; // MCIERR_INVALID_FILE
    let name = path.to_str().ok_or(INVALID_FILE)?;
    if name.contains(['"', '\0', '\r', '\n']) {
        return Err(INVALID_FILE);
    }
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if !["mp3", "wav"]
        .iter()
        .any(|ext| extension.eq_ignore_ascii_case(ext))
    {
        return Err(INVALID_FILE);
    }
    Ok(format!(r#"open "{name}" type mpegvideo alias {ALIAS}"#))
}

#[cfg(windows)]
fn send(command: &str) -> Result<String, u32> {
    #[link(name = "winmm")]
    extern "system" {
        fn mciSendStringW(
            command: *const u16,
            output: *mut u16,
            output_len: u32,
            callback: *mut std::ffi::c_void,
        ) -> u32;
    }

    let command: Vec<u16> = command.encode_utf16().chain([0]).collect();
    let mut output = [0u16; 64];
    // Both buffers remain valid for this call. Passing no callback uses polling.
    let code = unsafe {
        mciSendStringW(
            command.as_ptr(),
            output.as_mut_ptr(),
            output.len() as u32,
            std::ptr::null_mut(),
        )
    };
    if code != 0 {
        return Err(code & 0xffff);
    }
    let end = output
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(output.len());
    Ok(String::from_utf16_lossy(&output[..end]))
}

#[cfg(not(windows))]
fn send(_command: &str) -> Result<String, u32> {
    Err(274) // MCIERR_UNSUPPORTED_FUNCTION
}
#[cfg(test)]
mod tests {
    use super::open_command;
    use std::path::Path;

    #[test]
    fn volume_defaults_low_and_cannot_exceed_the_menu_limit() {
        assert_eq!(super::DEFAULT_VOLUME, 10);
        assert_eq!(super::mci_volume(0), 0);
        assert_eq!(super::mci_volume(10), 100);
        assert_eq!(super::mci_volume(255), 250);
    }

    #[test]
    fn quotes_music_paths_and_selects_supported_audio_types() {
        assert_eq!(
            open_command(Path::new(r"O:\GTA VI\الصوت.MP3")).unwrap(),
            r#"open "O:\GTA VI\الصوت.MP3" type mpegvideo alias sunline_menu_music"#
        );
        assert!(open_command(Path::new("menu.wav"))
            .unwrap()
            .contains("type mpegvideo"));
        assert!(open_command(Path::new("audio.rpf")).is_err());
        assert!(open_command(Path::new("bad\"name.mp3")).is_err());
    }
}

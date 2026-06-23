use anyhow::Result;
use sync_image_core::Hotkey;

pub fn run<F>(hotkey: &Hotkey, on_trigger: F) -> Result<()>
where
    F: FnMut(),
{
    platform::run(hotkey, on_trigger)
}

#[cfg(windows)]
mod platform {
    use std::ptr;

    use anyhow::{Result, bail};
    use sync_image_core::Hotkey;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_ALT, MOD_CONTROL, MOD_SHIFT, RegisterHotKey, UnregisterHotKey,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

    const HOTKEY_ID: i32 = 0x4349;

    pub fn run<F>(hotkey: &Hotkey, mut on_trigger: F) -> Result<()>
    where
        F: FnMut(),
    {
        let modifiers = modifiers(hotkey);
        let key_code = hotkey.key as u32;

        unsafe {
            if RegisterHotKey(ptr::null_mut(), HOTKEY_ID, modifiers, key_code) == 0 {
                bail!("failed to register global hotkey {hotkey}");
            }
        }

        let _guard = HotkeyGuard;
        let mut message = MSG::default();
        loop {
            let result = unsafe { GetMessageW(&mut message, ptr::null_mut(), 0, 0) };
            if result == -1 {
                bail!("failed while waiting for Windows messages");
            }
            if result == 0 {
                break;
            }
            if message.message == WM_HOTKEY && message.wParam == HOTKEY_ID as usize {
                on_trigger();
            }
        }

        Ok(())
    }

    fn modifiers(hotkey: &Hotkey) -> u32 {
        let mut modifiers = 0;
        if hotkey.ctrl {
            modifiers |= MOD_CONTROL;
        }
        if hotkey.alt {
            modifiers |= MOD_ALT;
        }
        if hotkey.shift {
            modifiers |= MOD_SHIFT;
        }
        modifiers
    }

    struct HotkeyGuard;

    impl Drop for HotkeyGuard {
        fn drop(&mut self) {
            unsafe {
                UnregisterHotKey(ptr::null_mut(), HOTKEY_ID);
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    use anyhow::{Result, bail};
    use sync_image_core::Hotkey;

    pub fn run<F>(_hotkey: &Hotkey, _on_trigger: F) -> Result<()>
    where
        F: FnMut(),
    {
        bail!("global hotkey runtime is only supported on Windows in this MVP")
    }
}

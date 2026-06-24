use anyhow::Result;
use sync_image_core::Hotkey;

pub fn run<F>(hotkey: &Hotkey, on_trigger: F) -> Result<()>
where
    F: FnMut() + Send + 'static,
{
    start(hotkey, on_trigger)?.wait()
}

pub fn start<F>(hotkey: &Hotkey, on_trigger: F) -> Result<HotkeyRuntimeHandle>
where
    F: FnMut() + Send + 'static,
{
    platform::start(hotkey, on_trigger)
}

pub struct HotkeyRuntimeHandle {
    inner: platform::PlatformHandle,
}

impl HotkeyRuntimeHandle {
    pub fn stop(self) -> Result<()> {
        self.inner.stop()
    }

    pub fn wait(self) -> Result<()> {
        self.inner.wait()
    }
}

#[cfg(windows)]
mod platform {
    use std::{
        ptr,
        sync::mpsc,
        thread::{self, JoinHandle},
    };

    use anyhow::{Result, bail};
    use sync_image_core::Hotkey;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        MOD_ALT, MOD_CONTROL, MOD_SHIFT, RegisterHotKey, UnregisterHotKey,
    };
    use windows_sys::Win32::{
        System::Threading::GetCurrentThreadId,
        UI::WindowsAndMessaging::{GetMessageW, MSG, PostThreadMessageW, WM_HOTKEY, WM_QUIT},
    };

    use crate::hotkey_runtime::HotkeyRuntimeHandle;

    const HOTKEY_ID: i32 = 0x4349;

    pub struct PlatformHandle {
        join_handle: Option<JoinHandle<Result<()>>>,
        thread_id: u32,
    }

    impl PlatformHandle {
        pub fn stop(mut self) -> Result<()> {
            unsafe {
                if PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) == 0 {
                    bail!("failed to stop hotkey runtime thread");
                }
            }

            let join_handle = self.join_handle.take().expect("join handle should exist");
            join_result(join_handle)
        }

        pub fn wait(mut self) -> Result<()> {
            let join_handle = self.join_handle.take().expect("join handle should exist");
            join_result(join_handle)
        }
    }

    fn join_result(handle: JoinHandle<Result<()>>) -> Result<()> {
        match handle.join() {
            Ok(result) => result,
            Err(_) => Err(anyhow::anyhow!("hotkey runtime thread panicked")),
        }
    }

    pub fn start<F>(hotkey: &Hotkey, mut on_trigger: F) -> Result<HotkeyRuntimeHandle>
    where
        F: FnMut() + Send + 'static,
    {
        let hotkey = hotkey.clone();
        let (sender, receiver) = mpsc::channel();
        let join_handle = thread::spawn(move || {
            let modifiers = modifiers(&hotkey);
            let key_code = hotkey.key as u32;
            let thread_id = unsafe { GetCurrentThreadId() };

            unsafe {
                if RegisterHotKey(ptr::null_mut(), HOTKEY_ID, modifiers, key_code) == 0 {
                    let _ = sender.send(Err(anyhow::anyhow!(
                        "failed to register global hotkey {hotkey}"
                    )));
                    bail!("failed to register global hotkey {hotkey}");
                }
            }

            sender
                .send(Ok(thread_id))
                .map_err(|_| anyhow::anyhow!("failed to publish hotkey runtime thread id"))?;

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
        });

        let thread_id = receiver
            .recv()
            .map_err(|_| anyhow::anyhow!("failed to receive hotkey runtime thread id"))??;

        Ok(HotkeyRuntimeHandle {
            inner: PlatformHandle {
                join_handle: Some(join_handle),
                thread_id,
            },
        })
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

    use crate::hotkey_runtime::HotkeyRuntimeHandle;

    pub struct PlatformHandle;

    impl PlatformHandle {
        pub fn stop(self) -> Result<()> {
            Ok(())
        }

        pub fn wait(self) -> Result<()> {
            Ok(())
        }
    }

    pub fn start<F>(_hotkey: &Hotkey, _on_trigger: F) -> Result<HotkeyRuntimeHandle>
    where
        F: FnMut() + Send + 'static,
    {
        bail!("global hotkey runtime is only supported on Windows in this MVP")
    }
}

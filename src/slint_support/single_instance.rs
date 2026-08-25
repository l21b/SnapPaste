#[cfg(target_os = "windows")]
mod platform {
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, WAIT_OBJECT_0,
    };
    use windows_sys::Win32::System::Threading::{
        CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject,
    };

    #[cfg(not(debug_assertions))]
    const MUTEX_NAME: &str = "Local\\SnapPaste.SingleInstance.v2";
    #[cfg(not(debug_assertions))]
    const EVENT_NAME: &str = "Local\\SnapPaste.ShowMainWindow.v2";

    // Development builds run alongside the installed release so visual and
    // interaction QA never wakes or replaces the user's daily-use instance.
    #[cfg(debug_assertions)]
    const MUTEX_NAME: &str = "Local\\SnapPaste.Debug.SingleInstance.v2";
    #[cfg(debug_assertions)]
    const EVENT_NAME: &str = "Local\\SnapPaste.Debug.ShowMainWindow.v2";

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub struct SingleInstance {
        mutex: HANDLE,
        show_event: HANDLE,
    }

    impl SingleInstance {
        pub fn acquire() -> Result<Option<Self>, String> {
            let event_name = wide(EVENT_NAME);
            let mutex_name = wide(MUTEX_NAME);

            unsafe {
                let show_event = CreateEventW(null(), 0, 0, event_name.as_ptr());
                if show_event.is_null() {
                    return Err(format!(
                        "无法创建单实例事件：{}",
                        std::io::Error::last_os_error()
                    ));
                }

                let mutex = CreateMutexW(null(), 1, mutex_name.as_ptr());
                if mutex.is_null() {
                    CloseHandle(show_event);
                    return Err(format!(
                        "无法创建单实例锁：{}",
                        std::io::Error::last_os_error()
                    ));
                }

                if GetLastError() == ERROR_ALREADY_EXISTS {
                    let _ = SetEvent(show_event);
                    CloseHandle(mutex);
                    CloseHandle(show_event);
                    return Ok(None);
                }

                Ok(Some(Self { mutex, show_event }))
            }
        }

        pub fn take_show_request(&self) -> bool {
            unsafe { WaitForSingleObject(self.show_event, 0) == WAIT_OBJECT_0 }
        }
    }

    impl Drop for SingleInstance {
        fn drop(&mut self) {
            unsafe {
                if !self.mutex.is_null() {
                    CloseHandle(self.mutex);
                    self.mutex = null_mut();
                }
                if !self.show_event.is_null() {
                    CloseHandle(self.show_event);
                    self.show_event = null_mut();
                }
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    pub struct SingleInstance;

    impl SingleInstance {
        pub fn acquire() -> Result<Option<Self>, String> {
            Ok(Some(Self))
        }

        pub fn take_show_request(&self) -> bool {
            false
        }
    }
}

pub use platform::SingleInstance;

#![cfg(target_os = "windows")]

use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
use std::{
    num::{NonZeroIsize, NonZeroU32},
    ptr,
    rc::Rc,
};
use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::Gdi::{GdiFlush, GetDC, GetPixel, ReleaseDC},
    UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, HWND_TOPMOST, SW_HIDE, SWP_SHOWWINDOW, SetWindowPos,
        ShowWindow, WS_POPUP, WS_VISIBLE,
    },
};

struct NativeWindow(HWND);
impl Drop for NativeWindow {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}
impl HasWindowHandle for NativeWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe {
            WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(
                NonZeroIsize::new(self.0 as isize).unwrap(),
            )))
        })
    }
}
impl HasDisplayHandle for NativeWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
        })
    }
}

#[test]
#[ignore = "briefly shows a dedicated native window; run explicitly on an unlocked Windows desktop"]
fn full_present_survives_hide_and_move_from_clipped_position() {
    let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
    // A known backdrop verifies per-pixel transparency without reading other apps.
    let backdrop = unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            ptr::null(),
            WS_POPUP | WS_VISIBLE | 6,
            30,
            30,
            320,
            460,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
        )
    };
    assert!(!backdrop.is_null());
    let _backdrop = NativeWindow(backdrop);
    unsafe {
        SetWindowPos(backdrop, HWND_TOPMOST, 30, 30, 320, 460, SWP_SHOWWINDOW);
        windows_sys::Win32::Graphics::Gdi::UpdateWindow(backdrop);
    }
    let hwnd = unsafe {
        CreateWindowExW(
            windows_sys::Win32::UI::WindowsAndMessaging::WS_EX_LAYERED,
            class.as_ptr(),
            ptr::null(),
            WS_POPUP | WS_VISIBLE,
            -220,
            -390,
            264,
            421,
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            ptr::null(),
        )
    };
    assert!(!hwnd.is_null());
    let window = Rc::new(NativeWindow(hwnd));
    let context = softbuffer::Context::new(window.clone()).unwrap();
    let mut surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
    surface
        .resize(NonZeroU32::new(264).unwrap(), NonZeroU32::new(421).unwrap())
        .unwrap();
    for index in 0..20 {
        // Window libraries may reapply their cached styles during show/resize.
        if index % 3 == 0 {
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_LAYERED,
            };
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_LAYERED as isize));
            }
        }
        unsafe {
            ShowWindow(hwnd, SW_HIDE);
            assert_ne!(
                SetWindowPos(hwnd, HWND_TOPMOST, -220, -390, 264, 421, SWP_SHOWWINDOW),
                0
            );
        }
        let mut buffer = surface.buffer_mut().unwrap();
        buffer.fill(0xff2468ac);
        buffer[210 * 264 + 132] = 0;
        buffer.present().unwrap();
        unsafe {
            assert_ne!(
                SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    40 + index * 2,
                    40,
                    264,
                    421,
                    SWP_SHOWWINDOW
                ),
                0
            );
        }
        let mut buffer = surface.buffer_mut().unwrap();
        buffer.fill(0xff2468ac);
        buffer[210 * 264 + 132] = 0;
        buffer.present().unwrap();
        unsafe {
            GdiFlush();
            windows_sys::Win32::Graphics::Dwm::DwmFlush();
            let dc = GetDC(ptr::null_mut());
            assert!(!dc.is_null());
            let colors = [(4, 4), (259, 4), (4, 416), (259, 416), (132, 210)]
                .map(|(x, y)| GetPixel(dc, 40 + index * 2 + x, 40 + y));
            ReleaseDC(ptr::null_mut(), dc);
            assert!(
                colors[..4].iter().all(|color| *color == 0x00ac6824) && colors[4] == 0x00ffffff,
                "incomplete native frame on cycle {index}: {colors:?}"
            );
        }
    }
}

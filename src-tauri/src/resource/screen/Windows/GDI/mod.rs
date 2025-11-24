use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::{GetLastError, HWND};
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP, HDC,
    HGDIOBJ, RGBQUAD, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetDesktopWindow, GetSystemMetrics, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use super::{ScreenCaptureError, ScreenCapturer, ScreenFrame};

#[derive(Clone, Copy, Debug)]
struct CaptureRegion {
    origin_x: i32,
    origin_y: i32,
    width: i32,
    height: i32,
}

pub struct DesktopDuplicator {
    desktop_hwnd: HWND,
    screen_dc: HDC,
    memory_dc: HDC,
    bitmap: HBITMAP,
    old_bitmap: HGDIOBJ,
    region: CaptureRegion,
    stride: usize,
    buffer: Vec<u8>,
    bitmap_info: BITMAPINFO,
}

impl DesktopDuplicator {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        unsafe {
            let desktop_hwnd = GetDesktopWindow();
            let screen_dc = get_dc_checked(desktop_hwnd)?;
            let memory_dc = CreateCompatibleDC(screen_dc);
            if memory_dc.0.is_null() {
                release_dc_checked(desktop_hwnd, screen_dc);
                return Err(ScreenCaptureError::OsError {
                    context: "CreateCompatibleDC",
                    code: GetLastError().0,
                });
            }

            let region = detect_virtual_region();
            let bitmap = CreateCompatibleBitmap(screen_dc, region.width, region.height);
            if bitmap.0.is_null() {
                let _ = DeleteDC(memory_dc);
                release_dc_checked(desktop_hwnd, screen_dc);
                return Err(ScreenCaptureError::OsError {
                    context: "CreateCompatibleBitmap",
                    code: GetLastError().0,
                });
            }

            let bitmap_obj = HGDIOBJ(bitmap.0);
            let old_bitmap = SelectObject(memory_dc, bitmap_obj);
            if old_bitmap.0.is_null() {
                let _ = DeleteObject(bitmap);
                let _ = DeleteDC(memory_dc);
                release_dc_checked(desktop_hwnd, screen_dc);
                return Err(ScreenCaptureError::OsError {
                    context: "SelectObject",
                    code: GetLastError().0,
                });
            }

            let stride = (region.width as usize * 4).max(4);
            let buffer_len = stride * region.height as usize;
            let bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: region.width,
                    biHeight: -region.height, // top-down orientation
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: (stride * region.height as usize) as u32,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD::default(); 1],
            };

            Ok(Self {
                desktop_hwnd,
                screen_dc,
                memory_dc,
                bitmap,
                old_bitmap,
                region,
                stride,
                buffer: vec![0u8; buffer_len],
                bitmap_info,
            })
        }
    }

    fn capture_internal(&mut self) -> Result<(), ScreenCaptureError> {
        unsafe {
            BitBlt(
                self.memory_dc,
                0,
                0,
                self.region.width,
                self.region.height,
                self.screen_dc,
                self.region.origin_x,
                self.region.origin_y,
                SRCCOPY,
            )
            .map_err(|_| ScreenCaptureError::OsError {
                context: "BitBlt",
                code: GetLastError().0,
            })?;

            let scan_lines = GetDIBits(
                self.memory_dc,
                self.bitmap,
                0,
                self.region.height as u32,
                Some(self.buffer.as_mut_ptr() as *mut c_void),
                &mut self.bitmap_info,
                DIB_RGB_COLORS,
            );
            if scan_lines == 0 {
                return Err(ScreenCaptureError::OsError {
                    context: "GetDIBits",
                    code: GetLastError().0,
                });
            }
        }
        Ok(())
    }
}

impl ScreenCapturer for DesktopDuplicator {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        self.capture_internal()?;
        Ok(ScreenFrame {
            width: self.region.width as u32,
            height: self.region.height as u32,
            stride: self.stride,
            pixels: &self.buffer,
        })
    }

    fn size(&self) -> (u32, u32) {
        (
            self.region.width as u32,
            self.region.height as u32,
        )
    }
}

impl Drop for DesktopDuplicator {
    fn drop(&mut self) {
        unsafe {
            if !self.old_bitmap.0.is_null() {
                SelectObject(self.memory_dc, self.old_bitmap);
            }
            if !self.bitmap.0.is_null() {
                let _ = DeleteObject(self.bitmap);
            }
            if !self.memory_dc.0.is_null() {
                let _ = DeleteDC(self.memory_dc);
            }
            if !self.screen_dc.0.is_null() {
                release_dc_checked(self.desktop_hwnd, self.screen_dc);
            }
        }
    }
}

unsafe impl Send for DesktopDuplicator {}

fn detect_virtual_region() -> CaptureRegion {
    unsafe {
        let mut width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let mut height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
        let origin_x = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let origin_y = GetSystemMetrics(SM_YVIRTUALSCREEN);

        if width <= 0 {
            width = GetSystemMetrics(SM_CXSCREEN);
        }
        if height <= 0 {
            height = GetSystemMetrics(SM_CYSCREEN);
        }

        CaptureRegion {
            origin_x,
            origin_y,
            width: width.max(1),
            height: height.max(1),
        }
    }
}

unsafe fn get_dc_checked(hwnd: HWND) -> Result<HDC, ScreenCaptureError> {
    let dc = GetDC(hwnd);
    if dc.0.is_null() {
        Err(ScreenCaptureError::OsError {
            context: "GetDC",
            code: GetLastError().0,
        })
    } else {
        Ok(dc)
    }
}

unsafe fn release_dc_checked(hwnd: HWND, dc: HDC) {
    let _ = ReleaseDC(hwnd, dc);
}


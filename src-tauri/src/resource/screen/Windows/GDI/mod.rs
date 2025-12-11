//! Windows screen capture using GDI (Graphics Device Interface).
//!
//! This module provides a fallback screen capture implementation using GDI,
//! which has better compatibility with older systems and certain display configurations.

use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::Ordering;
use std::time::Instant;

use windows::Win32::Foundation::{GetLastError, HWND};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ERROR_NOT_FOUND};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    COLORONCOLOR, DIB_RGB_COLORS, HBITMAP, HDC, HGDIOBJ, RGBQUAD, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetDesktopWindow, GetSystemMetrics, SM_CXSCREEN, SM_CXVIRTUALSCREEN, SM_CYSCREEN,
    SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};

use crate::resource::screen::{ScreenCaptureError, ScreenCapturer, ScreenFrame};
use super::{CAPTURE_FPS, CAPTURE_SCALE_PERCENT};

const BYTES_PER_PIXEL: usize = 4;

#[derive(Clone, Copy, Debug)]
struct CaptureRegion {
    origin_x: i32,
    origin_y: i32,
    width: i32,
    height: i32,
}

struct ScreenDcGuard {
    hwnd: HWND,
    dc: HDC,
    active: bool,
}

impl ScreenDcGuard {
    unsafe fn new(hwnd: HWND) -> Result<Self, ScreenCaptureError> {
        Ok(Self {
            hwnd,
            dc: get_dc_checked(hwnd)?,
            active: true,
        })
    }

    fn handle(&self) -> HDC {
        self.dc
    }

    fn into_inner(mut self) -> HDC {
        self.active = false;
        self.dc
    }
}

impl Drop for ScreenDcGuard {
    fn drop(&mut self) {
        if self.active && !self.dc.0.is_null() {
            unsafe {
                release_dc_checked(self.hwnd, self.dc);
            }
        }
    }
}

struct MemoryDcGuard {
    dc: HDC,
    active: bool,
}

impl MemoryDcGuard {
    unsafe fn new(screen_dc: HDC) -> Result<Self, ScreenCaptureError> {
        let memory_dc = CreateCompatibleDC(Some(screen_dc));
        if memory_dc.0.is_null() {
            return Err(ScreenCaptureError::OsError {
                context: "CreateCompatibleDC",
                code: GetLastError().0,
            });
        }

        Ok(Self {
            dc: memory_dc,
            active: true,
        })
    }

    fn handle(&self) -> HDC {
        self.dc
    }

    fn into_inner(mut self) -> HDC {
        self.active = false;
        self.dc
    }
}

impl Drop for MemoryDcGuard {
    fn drop(&mut self) {
        if self.active && !self.dc.0.is_null() {
            unsafe {
                let _ = DeleteDC(self.dc);
            }
        }
    }
}

struct BitmapGuard {
    bitmap: HBITMAP,
    active: bool,
}

impl BitmapGuard {
    unsafe fn new(screen_dc: HDC, width: i32, height: i32) -> Result<Self, ScreenCaptureError> {
        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.0.is_null() {
            return Err(ScreenCaptureError::OsError {
                context: "CreateCompatibleBitmap",
                code: GetLastError().0,
            });
        }

        Ok(Self {
            bitmap,
            active: true,
        })
    }

    fn handle(&self) -> HBITMAP {
        self.bitmap
    }

    fn into_inner(mut self) -> HBITMAP {
        self.active = false;
        self.bitmap
    }
}

impl Drop for BitmapGuard {
    fn drop(&mut self) {
        if self.active && !self.bitmap.0.is_null() {
            unsafe {
                let _ = DeleteObject(self.bitmap.into());
            }
        }
    }
}

struct BitmapSelectionGuard {
    dc: HDC,
    old_bitmap: HGDIOBJ,
    active: bool,
}

impl BitmapSelectionGuard {
    unsafe fn new(dc: HDC, bitmap: HBITMAP) -> Result<Self, ScreenCaptureError> {
        let bitmap_obj = HGDIOBJ(bitmap.0);
        let old_bitmap = SelectObject(dc, bitmap_obj);
        if old_bitmap.0.is_null() {
            return Err(ScreenCaptureError::OsError {
                context: "SelectObject",
                code: GetLastError().0,
            });
        }

        Ok(Self {
            dc,
            old_bitmap,
            active: true,
        })
    }

    fn into_inner(mut self) -> HGDIOBJ {
        self.active = false;
        self.old_bitmap
    }
}

impl Drop for BitmapSelectionGuard {
    fn drop(&mut self) {
        if self.active && !self.old_bitmap.0.is_null() {
            unsafe {
                let _ = SelectObject(self.dc, self.old_bitmap);
            }
        }
    }
}

pub struct GdiCapturer {
    desktop_hwnd: HWND,
    screen_dc: HDC,
    memory_dc: HDC,
    bitmap: HBITMAP,
    old_bitmap: HGDIOBJ,
    region: CaptureRegion,
    output_index: usize,
    target_width: u32,
    target_height: u32,
    stride: usize,
    buffer: Vec<u8>,
    bitmap_info: BITMAPINFO,
    // Frame rate control
    last_capture_time: Option<Instant>,
    has_frame: bool,
}

impl GdiCapturer {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Self::with_output(0)
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        unsafe {
            let desktop_hwnd = GetDesktopWindow();
            let screen_dc_guard = ScreenDcGuard::new(desktop_hwnd)?;
            let memory_dc_guard = MemoryDcGuard::new(screen_dc_guard.handle())?;

            let region = detect_region(output_index);
            let scale_percent = CAPTURE_SCALE_PERCENT.load(Ordering::Relaxed).clamp(1, 100) as u32;
            let target_width = (region.width as u32 * scale_percent / 100).max(1);
            let target_height = (region.height as u32 * scale_percent / 100).max(1);

            let bitmap_guard = BitmapGuard::new(
                screen_dc_guard.handle(),
                target_width as i32,
                target_height as i32,
            )?;
            let selection_guard = BitmapSelectionGuard::new(memory_dc_guard.handle(), bitmap_guard.handle())?;

            // Prefer GPU-side scaling: memory bitmap uses the target (scaled) size.
            SetStretchBltMode(memory_dc_guard.handle(), COLORONCOLOR);

            let stride = (target_width as usize * BYTES_PER_PIXEL).max(4);
            let buffer_len = stride * target_height as usize;
            let bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: target_width as i32,
                    biHeight: -(target_height as i32), // top-down orientation
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    biSizeImage: (stride * target_height as usize) as u32,
                    biXPelsPerMeter: 0,
                    biYPelsPerMeter: 0,
                    biClrUsed: 0,
                    biClrImportant: 0,
                },
                bmiColors: [RGBQUAD::default(); 1],
            };
            let buffer = vec![0u8; buffer_len];

            // Transfer ownership of handles only after all fallible allocations succeed.
            let screen_dc = screen_dc_guard.into_inner();
            let memory_dc = memory_dc_guard.into_inner();
            let bitmap = bitmap_guard.into_inner();
            let old_bitmap = selection_guard.into_inner();

            Ok(Self {
                desktop_hwnd,
                screen_dc,
                memory_dc,
                bitmap,
                old_bitmap,
                region,
                output_index,
                target_width,
                target_height,
                stride,
                buffer,
                bitmap_info,
                last_capture_time: None,
                has_frame: false,
            })
        }
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }

    fn capture_internal(&mut self) -> Result<(), ScreenCaptureError> {
        unsafe {
            if !StretchBlt(
                self.memory_dc,
                0,
                0,
                self.target_width as i32,
                self.target_height as i32,
                Some(self.screen_dc),
                self.region.origin_x,
                self.region.origin_y,
                self.region.width,
                self.region.height,
                SRCCOPY,
            )
            .as_bool()
            {
                return Err(ScreenCaptureError::OsError {
                    context: "StretchBlt",
                    code: GetLastError().0,
                });
            }

            let scan_lines = GetDIBits(
                self.memory_dc,
                self.bitmap,
                0,
                self.target_height,
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

impl ScreenCapturer for GdiCapturer {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        let fps = CAPTURE_FPS.load(Ordering::Relaxed).clamp(1, 60) as u64;
        let interval = std::time::Duration::from_micros(1_000_000u64 / fps.max(1));
        let now = Instant::now();

        let should_capture = match self.last_capture_time {
            Some(last) => now.duration_since(last) >= interval,
            None => true,
        };

        if should_capture || !self.has_frame {
            self.capture_internal()?;
            self.last_capture_time = Some(now);
            self.has_frame = true;
        }

        Ok(ScreenFrame {
            width: self.target_width,
            height: self.target_height,
            stride: self.stride,
            pixels: &self.buffer,
            dirty_regions: &[],
        })
    }

    fn size(&self) -> (u32, u32) {
        (self.target_width, self.target_height)
    }
}

impl Drop for GdiCapturer {
    fn drop(&mut self) {
        unsafe {
            if !self.old_bitmap.0.is_null() {
                SelectObject(self.memory_dc, self.old_bitmap);
            }
            if !self.bitmap.0.is_null() {
                let _ = DeleteObject(self.bitmap.into());
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

unsafe impl Send for GdiCapturer {}

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

fn detect_region(output_index: usize) -> CaptureRegion {
    if let Ok(factory) = unsafe { CreateDXGIFactory1::<IDXGIFactory1>() } {
        let mut current_index = 0usize;
        for adapter_index in 0.. {
            let adapter = match unsafe { factory.EnumAdapters1(adapter_index) } {
                Ok(adapter) => adapter,
                Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(_) => break,
            };

            for output_idx in 0.. {
                let output = match unsafe { adapter.EnumOutputs(output_idx) } {
                    Ok(output) => output,
                    Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                    Err(_) => break,
                };

                let Ok(desc) = (unsafe { output.GetDesc() }) else { continue };
                if !desc.AttachedToDesktop.as_bool() {
                    continue;
                }

                if current_index == output_index {
                    let width = (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left).max(1);
                    let height = (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top).max(1);
                    return CaptureRegion {
                        origin_x: desc.DesktopCoordinates.left,
                        origin_y: desc.DesktopCoordinates.top,
                        width,
                        height,
                    };
                }

                current_index += 1;
            }
        }
    }

    detect_virtual_region()
}

unsafe fn get_dc_checked(hwnd: HWND) -> Result<HDC, ScreenCaptureError> {
    let dc = GetDC(Some(hwnd));
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
    let _ = ReleaseDC(Some(hwnd), dc);
}

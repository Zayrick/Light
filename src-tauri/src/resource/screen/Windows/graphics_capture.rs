//! Windows screen capture using WinRT Graphics Capture API.
//!
//! This module provides a high-performance screen capture backend using the
//! Windows Graphics Capture API, which is available on Windows 10 version 1903+.
//!
//! Key features:
//! - Event-driven frame updates (only captures when content changes)
//! - Support for dirty region tracking
//! - Cursor capture control
//! - Border visibility control

use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use rayon::prelude::*;
use windows::core::{Interface, Result as WinResult, HSTRING};
use windows::Foundation::Metadata::ApiInformation;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
    D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_9_1, D3D_FEATURE_LEVEL_9_2,
    D3D_FEATURE_LEVEL_9_3,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
    D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_SAMPLE_DESC;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ERROR_NOT_FOUND};
use windows::Win32::Graphics::Gdi::HMONITOR;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;

use crate::resource::screen::{DirtyRegion, ScreenCaptureError, ScreenCapturer, ScreenFrame};
use super::{BYTES_PER_PIXEL, CAPTURE_FPS, CAPTURE_SCALE_PERCENT};

/// WinRT Graphics Capture backend for fullscreen monitor capture.
///
/// Uses `Direct3D11CaptureFramePool::CreateFreeThreaded` for synchronous polling.
pub struct GraphicsCapturer {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    #[allow(dead_code)]
    direct3d_device: IDirect3DDevice,
    frame_pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    size: SizeInt32,
    buffer: Vec<u8>,
    stride: usize,
    dirty_regions: Vec<DirtyRegion>,
    output_index: usize,
    last_capture_time: Option<Instant>,
    has_frame: bool,
    // Reusable staging texture for CPU readback
    staging_texture: Option<ID3D11Texture2D>,
    staging_width: u32,
    staging_height: u32,
}

// SAFETY: The Windows COM objects are thread-safe when used correctly.
// We only access them from a single thread at a time.
unsafe impl Send for GraphicsCapturer {}

impl GraphicsCapturer {
    /// Creates a new Graphics Capture session for the specified monitor output.
    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        // Check if Graphics Capture API is supported
        if !Self::is_supported() {
            return Err(ScreenCaptureError::Unsupported(
                "Windows Graphics Capture API is not supported on this system",
            ));
        }

        let hmonitor = enumerate_monitor(output_index)?;

        let (device, context) = create_d3d11_device()?;
        let direct3d_device = create_direct3d_device(&device)?;

        let item = create_capture_item_for_monitor(hmonitor)?;
        let size = item.Size().map_err(|err| wrap_os_error("GraphicsCaptureItem::Size", err))?;

        // Use CreateFreeThreaded for synchronous polling from any thread
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &direct3d_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2, // Buffer count
            size,
        )
        .map_err(|err| wrap_os_error("CreateFreeThreaded", err))?;

        let session = frame_pool
            .CreateCaptureSession(&item)
            .map_err(|err| wrap_os_error("CreateCaptureSession", err))?;

        // Configure session options (if supported)
        Self::configure_session(&session);

        // Start capturing
        session
            .StartCapture()
            .map_err(|err| wrap_os_error("StartCapture", err))?;

        let stride = size.Width.max(1) as usize * BYTES_PER_PIXEL;

        Ok(Self {
            device,
            context,
            direct3d_device,
            frame_pool,
            session,
            size,
            buffer: Vec::new(),
            stride,
            dirty_regions: Vec::new(),
            output_index,
            last_capture_time: None,
            has_frame: false,
            staging_texture: None,
            staging_width: 0,
            staging_height: 0,
        })
    }

    /// Returns the output index this capturer is attached to.
    pub fn output_index(&self) -> usize {
        self.output_index
    }

    /// Checks if the Windows Graphics Capture API is supported.
    pub fn is_supported() -> bool {
        let result: WinResult<bool> = (|| {
            let contract_present = ApiInformation::IsApiContractPresentByMajor(
                &HSTRING::from("Windows.Foundation.UniversalApiContract"),
                8,
            )?;
            if !contract_present {
                return Ok(false);
            }
            GraphicsCaptureSession::IsSupported()
        })();
        result.unwrap_or(false)
    }

    /// Configure session options like cursor capture and border.
    fn configure_session(session: &GraphicsCaptureSession) {
        // Try to disable the capture border (available on Windows 10 2004+)
        let _ = session.SetIsBorderRequired(false);
        // Cursor capture is enabled by default
    }

    /// Ensure staging texture is properly sized.
    fn ensure_staging_texture(&mut self, width: u32, height: u32) -> Result<(), ScreenCaptureError> {
        if self.staging_texture.is_some() && self.staging_width == width && self.staging_height == height {
            return Ok(());
        }

        let staging_desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_STAGING,
            BindFlags: 0,
            CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
            MiscFlags: 0,
        };

        let mut staging = None;
        unsafe {
            self.device
                .CreateTexture2D(&staging_desc, None, Some(&mut staging))
                .map_err(|err| wrap_os_error("CreateTexture2D (staging)", err))?;
        }

        self.staging_texture = staging;
        self.staging_width = width;
        self.staging_height = height;

        Ok(())
    }

    /// Try to grab the next available frame.
    fn grab_frame(&mut self) -> Result<bool, ScreenCaptureError> {
        // Try to get next frame (non-blocking)
        let frame = match self.frame_pool.TryGetNextFrame() {
            Ok(f) => f,
            Err(_) => return Ok(false), // No frame available
        };

        let surface = frame
            .Surface()
            .map_err(|err| wrap_os_error("Frame::Surface", err))?;

        // Get the texture from the surface
        let access: windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess =
            surface
                .cast()
                .map_err(|err| wrap_os_error("cast<IDirect3DDxgiInterfaceAccess>", err))?;

        let texture: ID3D11Texture2D = unsafe {
            access
                .GetInterface()
                .map_err(|err| wrap_os_error("GetInterface<ID3D11Texture2D>", err))?
        };

        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut desc) };

        let width = desc.Width;
        let height = desc.Height;

        // Check if frame pool needs recreation due to size change
        let content_size = frame
            .ContentSize()
            .map_err(|err| wrap_os_error("ContentSize", err))?;
        if content_size.Width != self.size.Width || content_size.Height != self.size.Height {
            self.size = content_size;
            // Recreate frame pool with new size
            self.frame_pool
                .Recreate(
                    &self.direct3d_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    content_size,
                )
                .map_err(|err| wrap_os_error("FramePool::Recreate", err))?;
        }

        // Ensure staging texture is ready
        self.ensure_staging_texture(width, height)?;
        let staging = self.staging_texture.as_ref().unwrap();

        // Copy texture to staging
        unsafe {
            self.context.CopyResource(staging, &texture);
        }

        // Map staging texture for CPU read
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context
                .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .map_err(|err| wrap_os_error("Map (staging)", err))?;
        }

        let src_pitch = mapped.RowPitch as usize;
        let dst_stride = width as usize * BYTES_PER_PIXEL;
        let height_usize = height as usize;

        // Apply scaling if configured
        let scale_percent = CAPTURE_SCALE_PERCENT.load(Ordering::Relaxed).clamp(1, 100);
        let (target_width, target_height) = if scale_percent < 100 {
            compute_scaled_dimensions(width, height, scale_percent)
        } else {
            (width, height)
        };

        let target_stride = target_width as usize * BYTES_PER_PIXEL;
        self.buffer.resize(target_stride * target_height as usize, 0);

        let src = unsafe {
            std::slice::from_raw_parts(mapped.pData as *const u8, src_pitch * height_usize)
        };

        if scale_percent < 100 {
            // Parallel downsampling
            let src_width = width as usize;
            let src_height = height_usize;
            let dst_width = target_width as usize;
            let dst_height = target_height as usize;

            self.buffer
                .par_chunks_mut(target_stride)
                .enumerate()
                .for_each(|(y, row)| {
                    let src_y = y * src_height / dst_height;
                    for x in 0..dst_width {
                        let src_x = x * src_width / dst_width;
                        let src_idx = src_y * src_pitch + src_x * BYTES_PER_PIXEL;
                        let dst_idx = x * BYTES_PER_PIXEL;
                        if src_idx + 4 <= src.len() && dst_idx + 4 <= row.len() {
                            row[dst_idx..dst_idx + 4].copy_from_slice(&src[src_idx..src_idx + 4]);
                        }
                    }
                });
        } else {
            // Direct copy with parallel rows
            self.buffer
                .par_chunks_mut(dst_stride)
                .enumerate()
                .for_each(|(y, row)| {
                    let src_row = &src[y * src_pitch..y * src_pitch + dst_stride];
                    row.copy_from_slice(src_row);
                });
        }

        unsafe {
            self.context.Unmap(staging, 0);
        }

        self.stride = target_stride;
        self.size = SizeInt32 {
            Width: target_width as i32,
            Height: target_height as i32,
        };

        // Extract dirty regions if available
        self.dirty_regions.clear();
        // Note: UpdateRectangles is only available on newer Windows versions
        // and requires DirtyRegionMode to be set. We skip this for now as
        // it requires additional API checks.

        self.has_frame = true;
        Ok(true)
    }
}

impl ScreenCapturer for GraphicsCapturer {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        // Honor global FPS limiter
        let fps = CAPTURE_FPS.load(Ordering::Relaxed).clamp(1, 60) as u64;
        let interval = Duration::from_micros(1_000_000u64 / fps.max(1));
        let now = Instant::now();

        let should_capture = match self.last_capture_time {
            Some(last) => now.duration_since(last) >= interval,
            None => true,
        };

        if should_capture || !self.has_frame {
            // Non-blocking: try to grab available frames without waiting
            // Graphics Capture is event-driven, frames arrive when content changes
            let mut got_frame = false;
            
            // Drain all available frames, keep the latest
            loop {
                match self.grab_frame() {
                    Ok(true) => {
                        got_frame = true;
                        // Continue to drain any buffered frames
                    }
                    Ok(false) => break, // No more frames available
                    Err(e) => return Err(e),
                }
            }

            if got_frame {
                self.last_capture_time = Some(now);
            } else if !self.has_frame {
                // First capture: need to wait briefly for initial frame
                let deadline = Instant::now() + Duration::from_millis(50);
                while Instant::now() < deadline && !self.has_frame {
                    if self.grab_frame()? {
                        self.last_capture_time = Some(now);
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            }

            if !self.has_frame {
                return Err(ScreenCaptureError::InvalidState("No frame available yet"));
            }
        }

        let width = self.size.Width.max(1) as u32;
        let height = self.size.Height.max(1) as u32;

        Ok(ScreenFrame {
            width,
            height,
            stride: self.stride,
            pixels: &self.buffer,
            dirty_regions: &self.dirty_regions,
        })
    }

    fn size(&self) -> (u32, u32) {
        (self.size.Width.max(1) as u32, self.size.Height.max(1) as u32)
    }
}

impl Drop for GraphicsCapturer {
    fn drop(&mut self) {
        // Close session and frame pool
        let _ = self.session.Close();
        let _ = self.frame_pool.Close();
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create D3D11 device and context.
fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext), ScreenCaptureError> {
    let feature_levels = [
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0,
        D3D_FEATURE_LEVEL_9_3,
        D3D_FEATURE_LEVEL_9_2,
        D3D_FEATURE_LEVEL_9_1,
    ];

    let mut device = None;
    let mut feature_level = D3D_FEATURE_LEVEL::default();
    let mut context = None;

    unsafe {
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            Some(&mut feature_level),
            Some(&mut context),
        )
        .map_err(|err| wrap_os_error("D3D11CreateDevice", err))?;
    }

    if feature_level.0 < D3D_FEATURE_LEVEL_11_0.0 {
        return Err(ScreenCaptureError::Unsupported(
            "DirectX 11 feature level not supported",
        ));
    }

    Ok((device.unwrap(), context.unwrap()))
}

/// Create WinRT IDirect3DDevice from Win32 ID3D11Device.
fn create_direct3d_device(device: &ID3D11Device) -> Result<IDirect3DDevice, ScreenCaptureError> {
    let dxgi_device: windows::Win32::Graphics::Dxgi::IDXGIDevice = device
        .cast()
        .map_err(|err| wrap_os_error("cast<IDXGIDevice>", err))?;

    let inspectable = unsafe {
        CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)
            .map_err(|err| wrap_os_error("CreateDirect3D11DeviceFromDXGIDevice", err))?
    };

    inspectable
        .cast()
        .map_err(|err| wrap_os_error("cast<IDirect3DDevice>", err))
}

/// Enumerate monitors and return HMONITOR for the specified index.
fn enumerate_monitor(output_index: usize) -> Result<HMONITOR, ScreenCaptureError> {
    unsafe {
        let factory: IDXGIFactory1 =
            CreateDXGIFactory1().map_err(|err| wrap_os_error("CreateDXGIFactory1", err))?;
        let mut current = 0usize;

        for adapter_index in 0.. {
            let adapter = match factory.EnumAdapters1(adapter_index) {
                Ok(adapter) => adapter,
                Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(err) => return Err(wrap_os_error("EnumAdapters1", err)),
            };

            for output_idx in 0.. {
                let output = match adapter.EnumOutputs(output_idx) {
                    Ok(output) => output,
                    Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                    Err(err) => return Err(wrap_os_error("EnumOutputs", err)),
                };

                let desc = output
                    .GetDesc()
                    .map_err(|err| wrap_os_error("GetDesc", err))?;

                if !desc.AttachedToDesktop.as_bool() {
                    continue;
                }

                if current == output_index {
                    return Ok(desc.Monitor);
                }

                current += 1;
            }
        }
    }

    Err(ScreenCaptureError::InvalidState(
        "No monitor found for Graphics Capture",
    ))
}

/// Create a GraphicsCaptureItem for a monitor.
fn create_capture_item_for_monitor(monitor: HMONITOR) -> Result<GraphicsCaptureItem, ScreenCaptureError> {
    unsafe {
        let interop: IGraphicsCaptureItemInterop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
                .map_err(|err| wrap_os_error("factory<IGraphicsCaptureItemInterop>", err))?;

        interop
            .CreateForMonitor(monitor)
            .map_err(|err| wrap_os_error("CreateForMonitor", err))
    }
}

/// Compute scaled dimensions for downsampling.
fn compute_scaled_dimensions(width: u32, height: u32, scale_percent: u8) -> (u32, u32) {
    let target_width = (width.saturating_mul(scale_percent as u32) / 100).max(1);
    let target_height = (height.saturating_mul(scale_percent as u32) / 100).max(1);

    // Round to power-of-two steps for cleaner mipmap-like scaling
    let mut scaled_width = width.max(1);
    let mut scaled_height = height.max(1);

    while scaled_width / 2 >= target_width && scaled_height / 2 >= target_height {
        scaled_width = (scaled_width / 2).max(1);
        scaled_height = (scaled_height / 2).max(1);
    }

    (scaled_width, scaled_height)
}

/// Wrap Windows error into ScreenCaptureError.
fn wrap_os_error(context: &'static str, err: windows::core::Error) -> ScreenCaptureError {
    ScreenCaptureError::OsError {
        context,
        code: err.code().0 as u32,
    }
}

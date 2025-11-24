use std::{mem, slice};

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0},
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
                D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
                D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
            },
            Dxgi::{
                Common::{
                    DXGI_MODE_ROTATION, DXGI_MODE_ROTATION_IDENTITY, DXGI_MODE_ROTATION_ROTATE180,
                    DXGI_MODE_ROTATION_ROTATE270, DXGI_MODE_ROTATION_ROTATE90,
                    DXGI_MODE_ROTATION_UNSPECIFIED,
                },
                CreateDXGIFactory1, IDXGIAdapter, IDXGIAdapter1, IDXGIFactory1, IDXGIOutput1,
                IDXGIOutputDuplication, IDXGIResource, IDXGISurface1, DXGI_ERROR_ACCESS_DENIED,
                DXGI_ERROR_ACCESS_LOST, DXGI_ERROR_NOT_FOUND, DXGI_ERROR_WAIT_TIMEOUT,
                DXGI_MAPPED_RECT, DXGI_MAP_READ, DXGI_OUTDUPL_FRAME_INFO, DXGI_OUTPUT_DESC,
            },
        },
    },
};

use super::{ScreenCaptureError, ScreenCapturer, ScreenFrame};

const BYTES_PER_PIXEL: usize = 4;
const DEFAULT_TIMEOUT_MS: u32 = 16;

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
}

pub fn list_displays() -> Result<Vec<DisplayInfo>, ScreenCaptureError> {
    unsafe {
        let factory: IDXGIFactory1 =
            CreateDXGIFactory1().map_err(|err| os_error("CreateDXGIFactory1", err))?;
        let mut displays = Vec::new();
        let mut current_index = 0usize;

        for adapter_index in 0.. {
            let adapter = match factory.EnumAdapters1(adapter_index) {
                Ok(adapter) => adapter,
                Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(err) => return Err(os_error("EnumAdapters1", err)),
            };

            for output_index in 0.. {
                let output = match adapter.EnumOutputs(output_index) {
                    Ok(output) => output,
                    Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                    Err(err) => return Err(os_error("IDXGIAdapter::EnumOutputs", err)),
                };

                let desc = output
                    .GetDesc()
                    .map_err(|err| os_error("IDXGIOutput::GetDesc", err))?;
                if !desc.AttachedToDesktop.as_bool() {
                    continue;
                }

                let (width, height) = output_dimensions(&desc);
                let raw_name = wide_to_string(&desc.DeviceName);
                let fallback = format!("Display {}", current_index + 1);
                let name = raw_name
                    .trim()
                    .is_empty()
                    .then(|| fallback.clone())
                    .unwrap_or(raw_name);

                displays.push(DisplayInfo {
                    index: current_index,
                    name,
                    width,
                    height,
                });

                current_index += 1;
            }
        }

        Ok(displays)
    }
}

/// Global manager that owns one `DesktopDuplicator` per active display index and
/// reference-counts how many consumers are using each display.
///
/// This ensures that:
/// - A desktop duplication instance is created only on first use of a display.
/// - All effects/devices requesting the same display share the same duplicator.
/// - When the last consumer drops its handle, the duplicator is destroyed and
///   underlying DXGI/D3D resources are released.
struct ScreenCaptureManager {
    outputs: HashMap<usize, ManagedOutput>,
}

struct ManagedOutput {
    duplicator: DesktopDuplicator,
    ref_count: usize,
}

impl ScreenCaptureManager {
    fn new() -> Self {
        Self {
            outputs: HashMap::new(),
        }
    }

    /// Increase reference count for a display, creating the duplicator on first use.
    fn acquire(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        if let Some(entry) = self.outputs.get_mut(&output_index) {
            entry.ref_count += 1;
            return Ok(());
        }

        let duplicator = DesktopDuplicator::with_output(output_index)?;
        self.outputs.insert(
            output_index,
            ManagedOutput {
                duplicator,
                ref_count: 1,
            },
        );
        Ok(())
    }

    /// Decrease reference count and drop the duplicator when it is no longer used.
    fn release(&mut self, output_index: usize) {
        if let Some(entry) = self.outputs.get_mut(&output_index) {
            if entry.ref_count > 1 {
                entry.ref_count -= 1;
                return;
            }
        }
        self.outputs.remove(&output_index);
    }

    /// Perform a capture on the specified display and hand the frame to the caller.
    ///
    /// Returns:
    /// - Ok(true) if a frame was captured and the callback was invoked.
    /// - Ok(false) if there is currently no active duplicator for this display.
    /// - Err(_) if the capture failed; for certain invalid-state errors, the
    ///   duplicator is dropped so that a future acquire will recreate it.
    fn capture_with<F>(
        &mut self,
        output_index: usize,
        f: F,
    ) -> Result<bool, ScreenCaptureError>
    where
        F: FnOnce(&ScreenFrame<'_>),
    {
        let Some(entry) = self.outputs.get_mut(&output_index) else {
            return Ok(false);
        };

        match entry.duplicator.capture() {
            Ok(frame) => {
                f(&frame);
                Ok(true)
            }
            Err(err) => {
                // If duplication is no longer valid, drop this instance so it
                // can be recreated on next acquire.
                if matches!(err, ScreenCaptureError::InvalidState(_)) {
                    self.outputs.remove(&output_index);
                }
                Err(err)
            }
        }
    }
}

static SCREEN_CAPTURE_MANAGER: OnceLock<Mutex<ScreenCaptureManager>> = OnceLock::new();

fn global_manager() -> &'static Mutex<ScreenCaptureManager> {
    SCREEN_CAPTURE_MANAGER.get_or_init(|| Mutex::new(ScreenCaptureManager::new()))
}

/// RAII handle representing a subscription to a particular display.
///
/// Cloning is intentionally not supported; each effect/device should own its
/// own handle. Dropping the handle automatically decreases the reference count
/// in the global manager.
#[derive(Debug)]
pub struct ScreenSubscription {
    display_index: usize,
}

impl ScreenSubscription {
    /// Create a new subscription for the specified display, incrementing the
    /// global reference count and creating the duplicator if needed.
    pub fn new(display_index: usize) -> Result<Self, ScreenCaptureError> {
        let manager = global_manager();
        let mut guard = manager.lock().unwrap();
        guard.acquire(display_index)?;
        Ok(Self { display_index })
    }

    pub fn display_index(&self) -> usize {
        self.display_index
    }

    /// Capture a frame from the subscribed display and invoke the callback.
    pub fn capture_with<F>(&self, f: F) -> Result<bool, ScreenCaptureError>
    where
        F: FnOnce(&ScreenFrame<'_>),
    {
        let manager = global_manager();
        let mut guard = manager.lock().unwrap();
        guard.capture_with(self.display_index, f)
    }
}

impl Drop for ScreenSubscription {
    fn drop(&mut self) {
        let manager = global_manager();
        if let Ok(mut guard) = manager.lock() {
            guard.release(self.display_index);
        }
    }
}

pub struct DesktopDuplicator {
    device: ID3D11Device,
    device_context: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
    rotation: DXGI_MODE_ROTATION,
    output_index: usize,
    timeout_ms: u32,
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    stride: usize,
    has_frame: bool,
}

impl DesktopDuplicator {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Self::with_output(0)
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        let (device, device_context, duplication, desc) = create_duplication(output_index)?;
        let (width, height) = output_dimensions(&desc);

        Ok(Self {
            device,
            device_context,
            duplication,
            rotation: desc.Rotation,
            output_index,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            buffer: Vec::new(),
            width,
            height,
            stride: width as usize * BYTES_PER_PIXEL,
            has_frame: false,
        })
    }

    pub fn set_output_index(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        if self.output_index == output_index {
            return Ok(());
        }

        let (device, device_context, duplication, desc) = create_duplication(output_index)?;
        let (width, height) = output_dimensions(&desc);

        self.device = device;
        self.device_context = device_context;
        self.duplication = duplication;
        self.rotation = desc.Rotation;
        self.output_index = output_index;
        self.width = width;
        self.height = height;
        self.stride = width as usize * BYTES_PER_PIXEL;
        self.buffer.clear();
        self.has_frame = false;

        Ok(())
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }

    fn capture_internal(&mut self) -> Result<CaptureStatus, ScreenCaptureError> {
        unsafe {
            let mut _frame_info: DXGI_OUTDUPL_FRAME_INFO = mem::zeroed();
            let mut resource: Option<IDXGIResource> = None;

            if let Err(err) =
                self.duplication
                    .AcquireNextFrame(self.timeout_ms, &mut _frame_info, &mut resource)
            {
                let code = err.code();
                if code == DXGI_ERROR_WAIT_TIMEOUT {
                    return Ok(CaptureStatus::NoFrame);
                }
                if code == DXGI_ERROR_ACCESS_LOST || code == DXGI_ERROR_ACCESS_DENIED {
                    return Err(ScreenCaptureError::InvalidState(
                        "DXGI output duplication became unavailable",
                    ));
                }
                return Err(os_error("AcquireNextFrame", err));
            }

            let mut frame_guard = AcquireGuard::new(&self.duplication);

            let resource = resource.ok_or(ScreenCaptureError::InvalidState(
                "DXGI output duplication returned no resource",
            ))?;
            let texture: ID3D11Texture2D = resource
                .cast()
                .map_err(|err| os_error("IDXGIResource::cast<ID3D11Texture2D>", err))?;

            let mut desc = D3D11_TEXTURE2D_DESC::default();
            texture.GetDesc(&mut desc);

            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = 0;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            desc.MiscFlags = 0;

            let mut staging: Option<ID3D11Texture2D> = None;
            self.device
                .CreateTexture2D(&desc, None, Some(&mut staging))
                .map_err(|err| os_error("CreateTexture2D", err))?;
            let staging = staging.unwrap();

            self.device_context.CopyResource(&staging, &texture);

            frame_guard.release();
            drop(frame_guard);

            let surface: IDXGISurface1 = staging
                .cast()
                .map_err(|err| os_error("ID3D11Texture2D::cast<IDXGISurface1>", err))?;
            let mut mapped = DXGI_MAPPED_RECT::default();
            surface
                .Map(&mut mapped, DXGI_MAP_READ)
                .map_err(|err| os_error("IDXGISurface1::Map", err))?;

            self.copy_surface(&mapped, desc.Width as usize, desc.Height as usize);

            surface
                .Unmap()
                .map_err(|err| os_error("IDXGISurface1::Unmap", err))?;

            self.has_frame = true;
            Ok(CaptureStatus::Updated)
        }
    }

    fn copy_surface(&mut self, mapped: &DXGI_MAPPED_RECT, width: usize, height: usize) {
        unsafe {
            let pitch = mapped.Pitch as usize;
            let data = slice::from_raw_parts(mapped.pBits as *const u8, pitch * height);
            let (rotated_width, rotated_height) = rotated_dimensions(width, height, self.rotation);

            self.buffer.clear();
            self.buffer
                .reserve(rotated_width * rotated_height * BYTES_PER_PIXEL);

            match self.rotation {
                DXGI_MODE_ROTATION_IDENTITY | DXGI_MODE_ROTATION_UNSPECIFIED => {
                    for row in 0..height {
                        let start = row * pitch;
                        let end = start + width * BYTES_PER_PIXEL;
                        self.buffer.extend_from_slice(&data[start..end]);
                    }
                }
                DXGI_MODE_ROTATION_ROTATE90 => {
                    for x in 0..width {
                        for y in (0..height).rev() {
                            let idx = y * pitch + x * BYTES_PER_PIXEL;
                            self.buffer
                                .extend_from_slice(&data[idx..idx + BYTES_PER_PIXEL]);
                        }
                    }
                }
                DXGI_MODE_ROTATION_ROTATE180 => {
                    for y in (0..height).rev() {
                        for x in (0..width).rev() {
                            let idx = y * pitch + x * BYTES_PER_PIXEL;
                            self.buffer
                                .extend_from_slice(&data[idx..idx + BYTES_PER_PIXEL]);
                        }
                    }
                }
                DXGI_MODE_ROTATION_ROTATE270 => {
                    for x in (0..width).rev() {
                        for y in 0..height {
                            let idx = y * pitch + x * BYTES_PER_PIXEL;
                            self.buffer
                                .extend_from_slice(&data[idx..idx + BYTES_PER_PIXEL]);
                        }
                    }
                }
                _ => {}
            }

            self.width = rotated_width as u32;
            self.height = rotated_height as u32;
            self.stride = rotated_width * BYTES_PER_PIXEL;
        }
    }
}

impl ScreenCapturer for DesktopDuplicator {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        match self.capture_internal()? {
            CaptureStatus::Updated => {}
            CaptureStatus::NoFrame => {
                if !self.has_frame {
                    return Err(ScreenCaptureError::InvalidState("No frame available yet"));
                }
            }
        }

        Ok(ScreenFrame {
            width: self.width,
            height: self.height,
            stride: self.stride,
            pixels: &self.buffer,
        })
    }

    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

unsafe impl Send for DesktopDuplicator {}

fn create_duplication(
    target_output_index: usize,
) -> Result<
    (
        ID3D11Device,
        ID3D11DeviceContext,
        IDXGIOutputDuplication,
        DXGI_OUTPUT_DESC,
    ),
    ScreenCaptureError,
> {
    unsafe {
        let factory: IDXGIFactory1 =
            CreateDXGIFactory1().map_err(|err| os_error("CreateDXGIFactory1", err))?;
        let mut current_index = 0usize;

        for adapter_index in 0.. {
            let adapter = match factory.EnumAdapters1(adapter_index) {
                Ok(adapter) => adapter,
                Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(err) => return Err(os_error("EnumAdapters1", err)),
            };

            if let Some(result) =
                try_initialize_on_adapter(&adapter, target_output_index, &mut current_index)?
            {
                return Ok(result);
            }
        }
    }

    Err(ScreenCaptureError::InvalidState(
        "No DXGI outputs available for duplication",
    ))
}

fn try_initialize_on_adapter(
    adapter: &IDXGIAdapter1,
    target_output_index: usize,
    current_index: &mut usize,
) -> Result<
    Option<(
        ID3D11Device,
        ID3D11DeviceContext,
        IDXGIOutputDuplication,
        DXGI_OUTPUT_DESC,
    )>,
    ScreenCaptureError,
> {
    unsafe {
        let base_adapter: IDXGIAdapter = adapter
            .cast()
            .map_err(|err| os_error("IDXGIAdapter1::cast<IDXGIAdapter>", err))?;
        let (device, device_context) =
            create_device(Some(&base_adapter)).map_err(|err| os_error("D3D11CreateDevice", err))?;

        for output_index in 0.. {
            let output = match base_adapter.EnumOutputs(output_index) {
                Ok(output) => output,
                Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(err) => return Err(os_error("IDXGIAdapter::EnumOutputs", err)),
            };

            let desc = output
                .GetDesc()
                .map_err(|err| os_error("IDXGIOutput::GetDesc", err))?;
            if !desc.AttachedToDesktop.as_bool() {
                continue;
            }

            if *current_index == target_output_index {
                let output1: IDXGIOutput1 = output
                    .cast()
                    .map_err(|err| os_error("IDXGIOutput::cast<IDXGIOutput1>", err))?;

                match output1.DuplicateOutput(&device) {
                    Ok(duplication) => {
                        return Ok(Some((device, device_context, duplication, desc)));
                    }
                    Err(err) => return Err(os_error("IDXGIOutput1::DuplicateOutput", err)),
                }
            } else {
                *current_index += 1;
            }
        }
    }

    Ok(None)
}

fn create_device(
    adapter: Option<&IDXGIAdapter>,
) -> windows::core::Result<(ID3D11Device, ID3D11DeviceContext)> {
    unsafe {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;
        let feature_levels = [D3D_FEATURE_LEVEL_11_0];

        D3D11CreateDevice(
            adapter,
            D3D_DRIVER_TYPE_UNKNOWN,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            Some(&feature_levels),
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;

        Ok((device.unwrap(), context.unwrap()))
    }
}

fn output_dimensions(desc: &DXGI_OUTPUT_DESC) -> (u32, u32) {
    let width = (desc.DesktopCoordinates.right - desc.DesktopCoordinates.left).max(1) as u32;
    let height = (desc.DesktopCoordinates.bottom - desc.DesktopCoordinates.top).max(1) as u32;
    (width, height)
}

fn rotated_dimensions(width: usize, height: usize, rotation: DXGI_MODE_ROTATION) -> (usize, usize) {
    match rotation {
        DXGI_MODE_ROTATION_ROTATE90 | DXGI_MODE_ROTATION_ROTATE270 => (height, width),
        _ => (width, height),
    }
}

fn wide_to_string(buffer: &[u16]) -> String {
    let end = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    String::from_utf16_lossy(&buffer[..end])
}

fn os_error(context: &'static str, err: windows::core::Error) -> ScreenCaptureError {
    ScreenCaptureError::OsError {
        context,
        code: err.code().0 as u32,
    }
}

enum CaptureStatus {
    Updated,
    NoFrame,
}

struct AcquireGuard<'a> {
    duplication: &'a IDXGIOutputDuplication,
    released: bool,
}

impl<'a> AcquireGuard<'a> {
    fn new(duplication: &'a IDXGIOutputDuplication) -> Self {
        Self {
            duplication,
            released: false,
        }
    }

    fn release(&mut self) {
        if !self.released {
            unsafe {
                let _ = self.duplication.ReleaseFrame();
            }
            self.released = true;
        }
    }
}

impl Drop for AcquireGuard<'_> {
    fn drop(&mut self) {
        if !self.released {
            unsafe {
                let _ = self.duplication.ReleaseFrame();
            }
        }
    }
}

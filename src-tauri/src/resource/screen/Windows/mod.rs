use std::{mem, slice};

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

pub struct DesktopDuplicator {
    device: ID3D11Device,
    device_context: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
    rotation: DXGI_MODE_ROTATION,
    timeout_ms: u32,
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    stride: usize,
    has_frame: bool,
}

impl DesktopDuplicator {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        let (device, device_context, duplication, desc) = create_duplication()?;
        let (width, height) = output_dimensions(&desc);

        Ok(Self {
            device,
            device_context,
            duplication,
            rotation: desc.Rotation,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            buffer: Vec::new(),
            width,
            height,
            stride: width as usize * BYTES_PER_PIXEL,
            has_frame: false,
        })
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

fn create_duplication() -> Result<
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

        for adapter_index in 0.. {
            let adapter = match factory.EnumAdapters1(adapter_index) {
                Ok(adapter) => adapter,
                Err(err) if err.code() == DXGI_ERROR_NOT_FOUND => break,
                Err(err) => return Err(os_error("EnumAdapters1", err)),
            };

            if let Some(result) = try_initialize_on_adapter(&adapter)? {
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

            let output1: IDXGIOutput1 = output
                .cast()
                .map_err(|err| os_error("IDXGIOutput::cast<IDXGIOutput1>", err))?;

            match output1.DuplicateOutput(&device) {
                Ok(duplication) => {
                    return Ok(Some((device, device_context, duplication, desc)));
                }
                Err(err) if err.code() == DXGI_ERROR_ACCESS_DENIED => continue,
                Err(err) => return Err(os_error("IDXGIOutput1::DuplicateOutput", err)),
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

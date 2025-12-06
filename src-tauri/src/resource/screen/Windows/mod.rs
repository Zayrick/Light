//! Windows screen capture using Desktop Duplication API with GPU-accelerated HDR processing.
//!
//! This module implements a high-performance screen capture pipeline that:
//! 1. Captures desktop frames using DXGI Output Duplication
//! 2. Optionally requests HDR formats (R16G16B16A16_FLOAT, R10G10B10A2_UNORM)
//! 3. Uses GPU shaders for HDR to SDR tone mapping
//! 4. Performs hardware-accelerated downsampling via GenerateMips
//! 5. Only transfers the final small BGRA8 buffer to CPU

mod shaders;

use std::{mem, slice, time::Instant};

use serde::Serialize;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, AtomicU8, Ordering},
    Mutex, OnceLock,
};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::{
                D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP, D3D11_SRV_DIMENSION_TEXTURE2D,
                D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_DRIVER_TYPE_WARP,
                D3D_FEATURE_LEVEL_11_0,
            },
            Direct3D11::{
                D3D11CreateDevice, ID3D11Buffer, ID3D11Device, ID3D11DeviceContext,
                ID3D11InputLayout, ID3D11PixelShader, ID3D11RenderTargetView, ID3D11SamplerState,
                ID3D11ShaderResourceView, ID3D11Texture2D, ID3D11VertexShader,
                D3D11_BIND_RENDER_TARGET, D3D11_BIND_SHADER_RESOURCE, D3D11_BUFFER_DESC,
                D3D11_COMPARISON_NEVER, D3D11_CPU_ACCESS_READ, D3D11_CPU_ACCESS_WRITE,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_MAPPED_SUBRESOURCE,
                D3D11_MAP_READ, D3D11_RENDER_TARGET_VIEW_DESC,
                D3D11_RESOURCE_MISC_GENERATE_MIPS, D3D11_RTV_DIMENSION_TEXTURE2D,
                D3D11_SAMPLER_DESC, D3D11_SDK_VERSION, D3D11_SHADER_RESOURCE_VIEW_DESC,
                D3D11_SHADER_RESOURCE_VIEW_DESC_0,
                D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_RTV, D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC,
                D3D11_TEXTURE_ADDRESS_CLAMP, D3D11_USAGE_DEFAULT, D3D11_USAGE_DYNAMIC,
                D3D11_USAGE_STAGING, D3D11_VIEWPORT,
            },
            Dxgi::{
                Common::{
                    DXGI_COLOR_SPACE_TYPE, DXGI_FORMAT, DXGI_FORMAT_B8G8R8A8_UNORM,
                    DXGI_FORMAT_R10G10B10A2_UNORM, DXGI_FORMAT_R16G16B16A16_FLOAT,
                    DXGI_FORMAT_R32G32B32_FLOAT, DXGI_MODE_ROTATION, DXGI_MODE_ROTATION_IDENTITY,
                    DXGI_MODE_ROTATION_ROTATE180, DXGI_MODE_ROTATION_ROTATE270,
                    DXGI_MODE_ROTATION_ROTATE90, DXGI_MODE_ROTATION_UNSPECIFIED, DXGI_SAMPLE_DESC,
                },
                CreateDXGIFactory1, IDXGIAdapter, IDXGIAdapter1, IDXGIFactory1, IDXGIOutput1,
                IDXGIOutput6, IDXGIOutputDuplication, IDXGIResource, IDXGISurface1,
                DXGI_ERROR_ACCESS_DENIED, DXGI_ERROR_ACCESS_LOST, DXGI_ERROR_NOT_FOUND,
                DXGI_ERROR_WAIT_TIMEOUT, DXGI_MAPPED_RECT, DXGI_MAP_READ, DXGI_OUTDUPL_DESC,
                DXGI_OUTDUPL_FRAME_INFO, DXGI_OUTPUT_DESC, DXGI_OUTPUT_DESC1,
            },
        },
    },
};

use super::{ScreenCaptureError, ScreenCapturer, ScreenFrame};

/// HDR color space constant (DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020 = 12)
const HDR_COLOR_SPACE: DXGI_COLOR_SPACE_TYPE = DXGI_COLOR_SPACE_TYPE(12);

const BYTES_PER_PIXEL: usize = 4;
const DEFAULT_TIMEOUT_MS: u32 = 16;
const DEFAULT_CAPTURE_FPS: u8 = 30;
const DEFAULT_TARGET_NITS: u32 = 200;

/// Percentage scale factor (1-100) for the capture resolution.
static CAPTURE_SCALE_PERCENT: AtomicU8 = AtomicU8::new(5);
static CAPTURE_FPS: AtomicU8 = AtomicU8::new(DEFAULT_CAPTURE_FPS);
static HARDWARE_ACCELERATION: AtomicBool = AtomicBool::new(true);

pub fn set_capture_scale_percent(percent: u8) {
    CAPTURE_SCALE_PERCENT.store(percent.clamp(1, 100), Ordering::Relaxed);
}

pub fn get_capture_scale_percent() -> u8 {
    CAPTURE_SCALE_PERCENT.load(Ordering::Relaxed)
}

pub fn set_capture_fps(fps: u8) {
    CAPTURE_FPS.store(fps.clamp(1, 60), Ordering::Relaxed);
}

pub fn get_capture_fps() -> u8 {
    CAPTURE_FPS.load(Ordering::Relaxed)
}

pub fn set_hardware_acceleration(enabled: bool) {
    HARDWARE_ACCELERATION.store(enabled, Ordering::Relaxed);
}

pub fn get_hardware_acceleration() -> bool {
    HARDWARE_ACCELERATION.load(Ordering::Relaxed)
}

#[allow(dead_code)]
pub fn set_sample_ratio(_percent: u8) {}

#[allow(dead_code)]
pub fn get_sample_ratio() -> u8 {
    100
}

#[derive(Debug, Clone, Serialize)]
pub struct DisplayInfo {
    pub index: usize,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_hdr: bool,
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

                // Check HDR support via IDXGIOutput6
                let is_hdr = if let Ok(output6) = output.cast::<IDXGIOutput6>() {
                    if let Ok(desc1) = output6.GetDesc1() {
                        desc1.ColorSpace == HDR_COLOR_SPACE
                    } else {
                        false
                    }
                } else {
                    false
                };

                let (width, height) = output_dimensions(&desc);
                let raw_name = wide_to_string(&desc.DeviceName);
                let fallback = format!("Display {}", current_index + 1);
                let name = if raw_name.trim().is_empty() {
                    fallback
                } else {
                    raw_name
                };

                displays.push(DisplayInfo {
                    index: current_index,
                    name,
                    width,
                    height,
                    is_hdr,
                });

                current_index += 1;
            }
        }

        Ok(displays)
    }
}

/// Shares one `DesktopDuplicator` per display and frees it when unused.
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

    fn release(&mut self, output_index: usize) {
        if let Some(entry) = self.outputs.get_mut(&output_index) {
            if entry.ref_count > 1 {
                entry.ref_count -= 1;
                return;
            }
        }
        self.outputs.remove(&output_index);
    }

    fn capture_with<F>(&mut self, output_index: usize, f: F) -> Result<bool, ScreenCaptureError>
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

/// RAII handle for a display subscription.
#[derive(Debug)]
pub struct ScreenSubscription {
    display_index: usize,
}

impl ScreenSubscription {
    pub fn new(display_index: usize) -> Result<Self, ScreenCaptureError> {
        let manager = global_manager();
        let mut guard = manager.lock().unwrap();
        guard.acquire(display_index)?;
        Ok(Self { display_index })
    }

    pub fn display_index(&self) -> usize {
        self.display_index
    }

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

/// GPU resources for HDR processing pipeline.
struct GpuPipeline {
    // Shader resources
    vertex_shader: ID3D11VertexShader,
    pixel_shader: ID3D11PixelShader,
    input_layout: ID3D11InputLayout,
    sampler: ID3D11SamplerState,
    constant_buffer: ID3D11Buffer,

    // Render target for HDR conversion
    convert_texture: ID3D11Texture2D,
    render_target_view: ID3D11RenderTargetView,

    // For mip-map based downsampling (SDR path)
    mip_texture: Option<ID3D11Texture2D>,
    mip_srv: Option<ID3D11ShaderResourceView>,
    mip_levels: u32,
}

#[allow(dead_code)]
pub struct DesktopDuplicator {
    device: ID3D11Device,
    device_context: ID3D11DeviceContext,
    duplication: IDXGIOutputDuplication,
    dupl_desc: DXGI_OUTDUPL_DESC,
    rotation: DXGI_MODE_ROTATION,
    output_index: usize,
    timeout_ms: u32,
    buffer: Vec<u8>,
    width: u32,
    height: u32,
    stride: usize,
    has_frame: bool,
    last_capture_time: Option<Instant>,

    // HDR state
    is_hdr: bool,
    target_nits: u32,

    // Staging texture for CPU readback
    staging_texture: ID3D11Texture2D,
    actual_width: u32,
    actual_height: u32,

    // GPU pipeline (only for HDR or hardware acceleration)
    gpu_pipeline: Option<GpuPipeline>,
}

impl DesktopDuplicator {
    pub fn new() -> Result<Self, ScreenCaptureError> {
        Self::with_output(0)
    }

    pub fn with_output(output_index: usize) -> Result<Self, ScreenCaptureError> {
        let hardware = HARDWARE_ACCELERATION.load(Ordering::Relaxed);
        let (device, device_context, duplication, dupl_desc, desc, _desc1, is_hdr) =
            create_duplication(output_index, hardware)?;

        let (width, height) = output_dimensions(&desc);
        let rotation = match dupl_desc.Rotation {
            r @ (DXGI_MODE_ROTATION_ROTATE90 | DXGI_MODE_ROTATION_ROTATE270) => r,
            _ => desc.Rotation,
        };

        // Calculate actual capture dimensions after rotation
        let (actual_width, actual_height) = rotated_dimensions(width, height, rotation);

        // Calculate scaled dimensions
        let scale_percent = CAPTURE_SCALE_PERCENT.load(Ordering::Relaxed).clamp(1, 100) as u32;
        let scaled_width = (actual_width * scale_percent / 100).max(1);
        let scaled_height = (actual_height * scale_percent / 100).max(1);

        // Create staging texture for final CPU readback
        let staging_texture = unsafe {
            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: scaled_width,
                Height: scaled_height,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut staging: Option<ID3D11Texture2D> = None;
            device
                .CreateTexture2D(&staging_desc, None, Some(&mut staging))
                .map_err(|err| os_error("CreateTexture2D (staging)", err))?;
            staging.unwrap()
        };

        // Create GPU pipeline if hardware acceleration is enabled or HDR is active
        let gpu_pipeline = if hardware {
            Some(create_gpu_pipeline(
                &device,
                &device_context,
                width,
                height,
                scaled_width,
                scaled_height,
                is_hdr,
                DEFAULT_TARGET_NITS,
            )?)
        } else {
            None
        };

        Ok(Self {
            device,
            device_context,
            duplication,
            dupl_desc,
            rotation,
            output_index,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            buffer: Vec::new(),
            width: scaled_width,
            height: scaled_height,
            stride: scaled_width as usize * BYTES_PER_PIXEL,
            has_frame: false,
            last_capture_time: None,
            is_hdr,
            target_nits: DEFAULT_TARGET_NITS,
            staging_texture,
            actual_width,
            actual_height,
            gpu_pipeline,
        })
    }

    pub fn set_output_index(&mut self, output_index: usize) -> Result<(), ScreenCaptureError> {
        if self.output_index == output_index {
            return Ok(());
        }

        // Recreate with new output
        let new = Self::with_output(output_index)?;
        *self = new;
        Ok(())
    }

    pub fn output_index(&self) -> usize {
        self.output_index
    }

    fn capture_internal(&mut self) -> Result<CaptureStatus, ScreenCaptureError> {
        unsafe {
            let mut frame_info: DXGI_OUTDUPL_FRAME_INFO = mem::zeroed();
            let mut resource: Option<IDXGIResource> = None;

            if let Err(err) =
                self.duplication
                    .AcquireNextFrame(self.timeout_ms, &mut frame_info, &mut resource)
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

            let resource = resource.ok_or(ScreenCaptureError::InvalidState(
                "DXGI output duplication returned no resource",
            ))?;
            let desktop_texture: ID3D11Texture2D = resource
                .cast()
                .map_err(|err| os_error("IDXGIResource::cast<ID3D11Texture2D>", err))?;

            // Process frame based on pipeline type
            let has_gpu_pipeline = self.gpu_pipeline.is_some();
            if has_gpu_pipeline {
                self.process_gpu_pipeline(&desktop_texture)?;
            } else {
                self.process_cpu_fallback(&desktop_texture)?;
            }

            // Release frame after processing
            let _ = self.duplication.ReleaseFrame();

            self.has_frame = true;
            Ok(CaptureStatus::Updated)
        }
    }

    /// GPU-accelerated processing path.
    fn process_gpu_pipeline(
        &mut self,
        desktop_texture: &ID3D11Texture2D,
    ) -> Result<(), ScreenCaptureError> {
        let pipeline = self.gpu_pipeline.as_ref().unwrap();
        unsafe {
            let ctx = &self.device_context;

            if self.is_hdr {
                // HDR path: use shaders for tone mapping; if the captured frame is actually SDR
                // or shader creation fails, automatically fall back to the SDR pipeline and
                // disable HDR for the rest of the session (auto-detect to avoid "negative optimization").
                match self.process_hdr_with_shaders(desktop_texture, pipeline) {
                    Ok(()) => {}
                    Err(_) => {
                        // Disable HDR for subsequent frames and fall back to SDR path now.
                        self.is_hdr = false;
                        self.process_sdr_with_mips(desktop_texture, pipeline)?;
                    }
                }
            } else {
                // SDR path: use GenerateMips for hardware downsampling
                self.process_sdr_with_mips(desktop_texture, pipeline)?;
            }

            // Copy result to staging texture
            ctx.CopyResource(&self.staging_texture, &pipeline.convert_texture);

            // Map and read back to CPU buffer
            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            ctx.Map(&self.staging_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                .map_err(|err| os_error("Map staging texture", err))?;

            let src_pitch = mapped.RowPitch as usize;
            let dst_stride = self.width as usize * BYTES_PER_PIXEL;
            let height = self.height as usize;

            self.buffer.resize(dst_stride * height, 0);

            let src = slice::from_raw_parts(mapped.pData as *const u8, src_pitch * height);
            for y in 0..height {
                let src_row = &src[y * src_pitch..y * src_pitch + dst_stride];
                let dst_row = &mut self.buffer[y * dst_stride..(y + 1) * dst_stride];
                dst_row.copy_from_slice(src_row);
            }

            ctx.Unmap(&self.staging_texture, 0);

            Ok(())
        }
    }

    /// Process HDR content using pixel shaders for tone mapping.
    fn process_hdr_with_shaders(
        &self,
        desktop_texture: &ID3D11Texture2D,
        pipeline: &GpuPipeline,
    ) -> Result<(), ScreenCaptureError> {
        unsafe {
            let ctx = &self.device_context;

            // Get texture description for format info
            let mut tex_desc = D3D11_TEXTURE2D_DESC::default();
            desktop_texture.GetDesc(&mut tex_desc);

            // Verify it's an HDR format
            if tex_desc.Format != DXGI_FORMAT_R16G16B16A16_FLOAT
                && tex_desc.Format != DXGI_FORMAT_R10G10B10A2_UNORM
            {
                return Err(ScreenCaptureError::InvalidState(
                    "Expected HDR format but got SDR",
                ));
            }

            // Create shader resource view for the desktop texture
            let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: tex_desc.Format,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: 1,
                    },
                },
            };
            let mut srv: Option<ID3D11ShaderResourceView> = None;
            self.device
                .CreateShaderResourceView(desktop_texture, Some(&srv_desc), Some(&mut srv))
                .map_err(|err| os_error("CreateShaderResourceView (HDR)", err))?;
            let srv = srv.unwrap();

            // Set up render pipeline
            ctx.OMSetRenderTargets(Some(&[Some(pipeline.render_target_view.clone())]), None);
            ctx.VSSetShader(&pipeline.vertex_shader, None);
            ctx.PSSetShader(&pipeline.pixel_shader, None);
            ctx.PSSetShaderResources(0, Some(&[Some(srv.clone())]));
            ctx.PSSetSamplers(0, Some(&[Some(pipeline.sampler.clone())]));
            ctx.VSSetConstantBuffers(0, Some(&[Some(pipeline.constant_buffer.clone())]));
            ctx.IASetInputLayout(&pipeline.input_layout);
            ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);

            // Set viewport
            let viewport = D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.width as f32,
                Height: self.height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            };
            ctx.RSSetViewports(Some(&[viewport]));

            // Draw fullscreen quad
            ctx.Draw(4, 0);

            // Cleanup
            ctx.PSSetShaderResources(0, Some(&[None]));

            Ok(())
        }
    }

    /// Process SDR content using GenerateMips for hardware downsampling.
    fn process_sdr_with_mips(
        &self,
        desktop_texture: &ID3D11Texture2D,
        pipeline: &GpuPipeline,
    ) -> Result<(), ScreenCaptureError> {
        unsafe {
            let ctx = &self.device_context;

            if let (Some(ref mip_texture), Some(ref mip_srv)) =
                (&pipeline.mip_texture, &pipeline.mip_srv)
            {
                // Copy desktop to mip level 0
                ctx.CopySubresourceRegion(mip_texture, 0, 0, 0, 0, desktop_texture, 0, None);

                // Generate mipmaps on GPU
                ctx.GenerateMips(mip_srv);

                // Copy the appropriate mip level to the convert texture
                let target_mip = pipeline.mip_levels.saturating_sub(1);
                ctx.CopySubresourceRegion(
                    &pipeline.convert_texture,
                    0,
                    0,
                    0,
                    0,
                    mip_texture,
                    target_mip,
                    None,
                );
            } else {
                // Fallback: direct copy (no downsampling)
                ctx.CopyResource(&pipeline.convert_texture, desktop_texture);
            }

            Ok(())
        }
    }

    /// CPU fallback path for when hardware acceleration is disabled.
    fn process_cpu_fallback(
        &mut self,
        desktop_texture: &ID3D11Texture2D,
    ) -> Result<(), ScreenCaptureError> {
        unsafe {
            let mut desc = D3D11_TEXTURE2D_DESC::default();
            desktop_texture.GetDesc(&mut desc);

            // Create staging texture matching source format
            desc.Usage = D3D11_USAGE_STAGING;
            desc.BindFlags = 0;
            desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
            desc.MiscFlags = 0;

            let mut staging: Option<ID3D11Texture2D> = None;
            self.device
                .CreateTexture2D(&desc, None, Some(&mut staging))
                .map_err(|err| os_error("CreateTexture2D (CPU staging)", err))?;
            let staging = staging.unwrap();

            self.device_context.CopyResource(&staging, desktop_texture);

            let surface: IDXGISurface1 = staging
                .cast()
                .map_err(|err| os_error("cast<IDXGISurface1>", err))?;
            let mut mapped = DXGI_MAPPED_RECT::default();
            surface
                .Map(&mut mapped, DXGI_MAP_READ)
                .map_err(|err| os_error("IDXGISurface1::Map", err))?;

            self.copy_surface_cpu(
                &mapped,
                desc.Width as usize,
                desc.Height as usize,
                DXGI_FORMAT(desc.Format.0),
            );

            surface
                .Unmap()
                .map_err(|err| os_error("IDXGISurface1::Unmap", err))?;

            Ok(())
        }
    }

    /// CPU-based surface copy with format conversion and downsampling.
    fn copy_surface_cpu(
        &mut self,
        mapped: &DXGI_MAPPED_RECT,
        width: usize,
        height: usize,
        format: DXGI_FORMAT,
    ) {
        unsafe {
            let pitch = mapped.Pitch as usize;
            let data = slice::from_raw_parts(mapped.pBits as *const u8, pitch * height);
            let (rotated_width, rotated_height) = rotated_dimensions(
                width as u32,
                height as u32,
                self.rotation,
            );
            let rotated_width = rotated_width as usize;
            let rotated_height = rotated_height as usize;

            let scale_percent =
                CAPTURE_SCALE_PERCENT.load(Ordering::Relaxed).clamp(1, 100) as usize;

            let scaled_width = (rotated_width * scale_percent / 100).max(1);
            let scaled_height = (rotated_height * scale_percent / 100).max(1);

            let mut scaled = vec![0u8; scaled_width * scaled_height * BYTES_PER_PIXEL];

            let src_bpp = bytes_per_pixel_for_format(format);

            for y in 0..scaled_height {
                let rotated_y = y * rotated_height / scaled_height;
                let dst_row_start = y * scaled_width * BYTES_PER_PIXEL;

                for x in 0..scaled_width {
                    let rotated_x = x * rotated_width / scaled_width;

                    let (src_x, src_y) = match self.rotation {
                        DXGI_MODE_ROTATION_IDENTITY | DXGI_MODE_ROTATION_UNSPECIFIED => {
                            (rotated_x, rotated_y)
                        }
                        DXGI_MODE_ROTATION_ROTATE90 => {
                            let h = height;
                            (rotated_y, h - 1 - rotated_x)
                        }
                        DXGI_MODE_ROTATION_ROTATE180 => {
                            let w = width;
                            let h = height;
                            (w - 1 - rotated_x, h - 1 - rotated_y)
                        }
                        DXGI_MODE_ROTATION_ROTATE270 => {
                            let w = width;
                            (w - 1 - rotated_y, rotated_x)
                        }
                        _ => (rotated_x, rotated_y),
                    };

                    let src_idx = src_y * pitch + src_x * src_bpp;
                    let dst_idx = dst_row_start + x * BYTES_PER_PIXEL;

                    let bgra = decode_pixel_to_bgra8(&data[src_idx..], format);
                    scaled[dst_idx..dst_idx + BYTES_PER_PIXEL].copy_from_slice(&bgra);
                }
            }

            self.buffer = scaled;
            self.width = scaled_width as u32;
            self.height = scaled_height as u32;
            self.stride = scaled_width * BYTES_PER_PIXEL;
        }
    }
}

impl ScreenCapturer for DesktopDuplicator {
    fn capture(&mut self) -> Result<ScreenFrame<'_>, ScreenCaptureError> {
        let fps = CAPTURE_FPS.load(Ordering::Relaxed).clamp(1, 60) as u64;
        let interval = std::time::Duration::from_micros(1_000_000u64 / fps.max(1));
        let now = Instant::now();

        let should_capture = match self.last_capture_time {
            Some(last) => now.duration_since(last) >= interval,
            None => true,
        };

        if should_capture || !self.has_frame {
            match self.capture_internal()? {
                CaptureStatus::Updated => {
                    self.last_capture_time = Some(now);
                }
                CaptureStatus::NoFrame => {
                    if !self.has_frame {
                        return Err(ScreenCaptureError::InvalidState("No frame available yet"));
                    }
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

/// Create GPU pipeline for HDR/SDR processing.
fn create_gpu_pipeline(
    device: &ID3D11Device,
    _ctx: &ID3D11DeviceContext,
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
    is_hdr: bool,
    target_nits: u32,
) -> Result<GpuPipeline, ScreenCaptureError> {
    unsafe {
        // Create vertex shader
        let mut vertex_shader: Option<ID3D11VertexShader> = None;
        device
            .CreateVertexShader(shaders::VERTEX_SHADER_BYTECODE, None, Some(&mut vertex_shader))
            .map_err(|err| os_error("CreateVertexShader", err))?;
        let vertex_shader = vertex_shader.unwrap();

        // Create pixel shader
        let mut pixel_shader: Option<ID3D11PixelShader> = None;
        device
            .CreatePixelShader(shaders::PIXEL_SHADER_BYTECODE, None, Some(&mut pixel_shader))
            .map_err(|err| os_error("CreatePixelShader", err))?;
        let pixel_shader = pixel_shader.unwrap();

        // Create input layout
        let layout_desc = [D3D11_INPUT_ELEMENT_DESC {
            SemanticName: windows::core::s!("SV_Position"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        }];
        let mut input_layout: Option<ID3D11InputLayout> = None;
        device
            .CreateInputLayout(
                &layout_desc,
                shaders::VERTEX_SHADER_BYTECODE,
                Some(&mut input_layout),
            )
            .map_err(|err| os_error("CreateInputLayout", err))?;
        let input_layout = input_layout.unwrap();

        // Create sampler
        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            MipLODBias: 0.0,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D11_COMPARISON_NEVER,
            BorderColor: [0.0; 4],
            MinLOD: 0.0,
            MaxLOD: f32::MAX,
        };
        let mut sampler: Option<ID3D11SamplerState> = None;
        device
            .CreateSamplerState(&sampler_desc, Some(&mut sampler))
            .map_err(|err| os_error("CreateSamplerState", err))?;
        let sampler = sampler.unwrap();

        // Create constant buffer for shader parameters
        let params: [f32; 4] = [
            target_nits as f32,
            18.8515625 - 18.6875 * target_nits as f32,
            0.0,
            0.0,
        ];
        let buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: 16,
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: windows::Win32::Graphics::Direct3D11::D3D11_BIND_CONSTANT_BUFFER.0 as u32,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE.0 as u32,
            MiscFlags: 0,
            StructureByteStride: 0,
        };
        let init_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: params.as_ptr() as *const _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };
        let mut constant_buffer: Option<ID3D11Buffer> = None;
        device
            .CreateBuffer(&buffer_desc, Some(&init_data), Some(&mut constant_buffer))
            .map_err(|err| os_error("CreateBuffer (constant)", err))?;
        let constant_buffer = constant_buffer.unwrap();

        // Create render target texture
        let convert_desc = D3D11_TEXTURE2D_DESC {
            Width: dst_width,
            Height: dst_height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };
        let mut convert_texture: Option<ID3D11Texture2D> = None;
        device
            .CreateTexture2D(&convert_desc, None, Some(&mut convert_texture))
            .map_err(|err| os_error("CreateTexture2D (convert)", err))?;
        let convert_texture = convert_texture.unwrap();

        // Create render target view
        let rtv_desc = D3D11_RENDER_TARGET_VIEW_DESC {
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            ViewDimension: D3D11_RTV_DIMENSION_TEXTURE2D,
            Anonymous: windows::Win32::Graphics::Direct3D11::D3D11_RENDER_TARGET_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_RTV { MipSlice: 0 },
            },
        };
        let mut render_target_view: Option<ID3D11RenderTargetView> = None;
        device
            .CreateRenderTargetView(&convert_texture, Some(&rtv_desc), Some(&mut render_target_view))
            .map_err(|err| os_error("CreateRenderTargetView", err))?;
        let render_target_view = render_target_view.unwrap();

        // Create mipmap texture for SDR downsampling
        let (mip_texture, mip_srv, mip_levels) = if !is_hdr {
            // Calculate mip levels needed
            let max_dim = src_width.max(src_height);
            let mut levels = 1u32;
            let mut size = max_dim;
            while size > dst_width.max(dst_height) && size > 1 {
                size /= 2;
                levels += 1;
            }

            let mip_desc = D3D11_TEXTURE2D_DESC {
                Width: src_width,
                Height: src_height,
                MipLevels: levels,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: (D3D11_BIND_RENDER_TARGET.0 | D3D11_BIND_SHADER_RESOURCE.0) as u32,
                CPUAccessFlags: 0,
                MiscFlags: D3D11_RESOURCE_MISC_GENERATE_MIPS.0 as u32,
            };
            let mut mip_tex: Option<ID3D11Texture2D> = None;
            device
                .CreateTexture2D(&mip_desc, None, Some(&mut mip_tex))
                .map_err(|err| os_error("CreateTexture2D (mip)", err))?;
            let mip_tex = mip_tex.unwrap();

            let srv_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: levels,
                    },
                },
            };
            let mut mip_srv_opt: Option<ID3D11ShaderResourceView> = None;
            device
                .CreateShaderResourceView(&mip_tex, Some(&srv_desc), Some(&mut mip_srv_opt))
                .map_err(|err| os_error("CreateShaderResourceView (mip)", err))?;

            (Some(mip_tex), mip_srv_opt, levels)
        } else {
            (None, None, 1)
        };

        Ok(GpuPipeline {
            vertex_shader,
            pixel_shader,
            input_layout,
            sampler,
            constant_buffer,
            convert_texture,
            render_target_view,
            mip_texture,
            mip_srv,
            mip_levels,
        })
    }
}

fn create_duplication(
    target_output_index: usize,
    try_hdr: bool,
) -> Result<
    (
        ID3D11Device,
        ID3D11DeviceContext,
        IDXGIOutputDuplication,
        DXGI_OUTDUPL_DESC,
        DXGI_OUTPUT_DESC,
        Option<DXGI_OUTPUT_DESC1>,
        bool,
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

            if let Some(result) = try_initialize_on_adapter(
                &adapter,
                target_output_index,
                &mut current_index,
                try_hdr,
            )? {
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
    try_hdr: bool,
) -> Result<
    Option<(
        ID3D11Device,
        ID3D11DeviceContext,
        IDXGIOutputDuplication,
        DXGI_OUTDUPL_DESC,
        DXGI_OUTPUT_DESC,
        Option<DXGI_OUTPUT_DESC1>,
        bool,
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
                // Try to get IDXGIOutput6 for HDR support
                if let Ok(output6) = output.cast::<IDXGIOutput6>() {
                    let desc1 = output6.GetDesc1().ok();
                    let is_hdr = desc1
                        .as_ref()
                        .map(|d| d.ColorSpace == HDR_COLOR_SPACE)
                        .unwrap_or(false);

                    // Try HDR formats first if HDR is available and requested
                    if try_hdr && is_hdr {
                        let hdr_formats = [
                            DXGI_FORMAT_R16G16B16A16_FLOAT,
                            DXGI_FORMAT_R10G10B10A2_UNORM,
                        ];
                        if let Ok(duplication) =
                            output6.DuplicateOutput1(&device, 0, &hdr_formats)
                        {
                            let dupl_desc = duplication.GetDesc();
                            return Ok(Some((
                                device,
                                device_context,
                                duplication,
                                dupl_desc,
                                desc,
                                desc1,
                                true,
                            )));
                        }
                    }

                    // Fallback to SDR format
                    let sdr_format = DXGI_FORMAT_B8G8R8A8_UNORM;
                    if let Ok(duplication) = output6.DuplicateOutput1(&device, 0, &[sdr_format]) {
                        let dupl_desc = duplication.GetDesc();
                        return Ok(Some((
                            device,
                            device_context,
                            duplication,
                            dupl_desc,
                            desc,
                            desc1,
                            false,
                        )));
                    }
                }

                // Ultimate fallback: use IDXGIOutput1
                let output1: IDXGIOutput1 = output
                    .cast()
                    .map_err(|err| os_error("IDXGIOutput::cast<IDXGIOutput1>", err))?;

                match output1.DuplicateOutput(&device) {
                    Ok(duplication) => {
                        let dupl_desc = duplication.GetDesc();
                        return Ok(Some((
                            device,
                            device_context,
                            duplication,
                            dupl_desc,
                            desc,
                            None,
                            false,
                        )));
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

        // Try hardware first, then WARP, then unknown
        let driver_types = [
            D3D_DRIVER_TYPE_UNKNOWN,
            D3D_DRIVER_TYPE_HARDWARE,
            D3D_DRIVER_TYPE_WARP,
        ];

        for driver_type in driver_types {
            let result = D3D11CreateDevice(
                adapter,
                driver_type,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            );

            if result.is_ok() {
                return Ok((device.unwrap(), context.unwrap()));
            }
        }

        // If all failed, try one more time without adapter hint
        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
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

fn rotated_dimensions(width: u32, height: u32, rotation: DXGI_MODE_ROTATION) -> (u32, u32) {
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

#[allow(dead_code)]
struct AcquireGuard<'a> {
    duplication: &'a IDXGIOutputDuplication,
    released: bool,
}

#[allow(dead_code)]
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

// CPU fallback helper functions

fn bytes_per_pixel_for_format(format: DXGI_FORMAT) -> usize {
    match format {
        DXGI_FORMAT_B8G8R8A8_UNORM => 4,
        DXGI_FORMAT_R10G10B10A2_UNORM => 4,
        DXGI_FORMAT_R16G16B16A16_FLOAT => 8,
        _ => 4,
    }
}

#[inline]
fn decode_pixel_to_bgra8(src: &[u8], format: DXGI_FORMAT) -> [u8; 4] {
    match format {
        DXGI_FORMAT_R10G10B10A2_UNORM => {
            if src.len() < 4 {
                return [0, 0, 0, 255];
            }
            let packed = u32::from_le_bytes([src[0], src[1], src[2], src[3]]);
            let r10 = (packed & 0x3FF) as u16;
            let g10 = ((packed >> 10) & 0x3FF) as u16;
            let b10 = ((packed >> 20) & 0x3FF) as u16;
            let a2 = ((packed >> 30) & 0x3) as u8;

            let r8 = (r10 >> 2) as u8;
            let g8 = (g10 >> 2) as u8;
            let b8 = (b10 >> 2) as u8;
            let a8 = a2 * 85;

            [b8, g8, r8, a8]
        }
        DXGI_FORMAT_R16G16B16A16_FLOAT => {
            if src.len() < 8 {
                return [0, 0, 0, 255];
            }
            let r_half = u16::from_le_bytes([src[0], src[1]]);
            let g_half = u16::from_le_bytes([src[2], src[3]]);
            let b_half = u16::from_le_bytes([src[4], src[5]]);
            let a_half = u16::from_le_bytes([src[6], src[7]]);

            let r8 = half_to_u8_tonemapped(r_half);
            let g8 = half_to_u8_tonemapped(g_half);
            let b8 = half_to_u8_tonemapped(b_half);
            let a8 = half_to_u8_tonemapped(a_half);

            [b8, g8, r8, a8]
        }
        _ => {
            if src.len() < 4 {
                return [0, 0, 0, 255];
            }
            [src[0], src[1], src[2], src[3]]
        }
    }
}

/// Convert half-float to u8 with simple Reinhard tone mapping for HDR.
#[inline]
fn half_to_u8_tonemapped(half: u16) -> u8 {
    let f = half_to_f32(half);
    // Simple Reinhard tone mapping: L / (1 + L)
    let tonemapped = f / (1.0 + f);
    // Apply gamma and convert to 8-bit
    let gamma_corrected = tonemapped.powf(1.0 / 2.2);
    (gamma_corrected.clamp(0.0, 1.0) * 255.0) as u8
}

#[inline]
fn half_to_f32(half: u16) -> f32 {
    let sign = ((half >> 15) & 1) as u32;
    let exponent = ((half >> 10) & 0x1F) as u32;
    let mantissa = (half & 0x3FF) as u32;

    if exponent == 0 {
        if mantissa == 0 {
            f32::from_bits(sign << 31)
        } else {
            let mut m = mantissa;
            let mut e: i32 = -14;
            while (m & 0x400) == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x3FF;
            let new_exp = (e + 127) as u32;
            f32::from_bits((sign << 31) | (new_exp << 23) | (m << 13))
        }
    } else if exponent == 31 {
        if mantissa == 0 {
            f32::from_bits((sign << 31) | (0xFF << 23))
        } else {
            f32::from_bits((sign << 31) | (0xFF << 23) | (mantissa << 13))
        }
    } else {
        let new_exp = (exponent as i32 - 15 + 127) as u32;
        f32::from_bits((sign << 31) | (new_exp << 23) | (mantissa << 13))
    }
}

use crate::errors::*;
use crate::render::*;
use chrono::prelude::*;
use floating_duration::TimeAsFloat;
use std::time::Instant;
use std::{
    any::Any,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct WgpuRenderBackend {
    instance: wgpu::Instance,
    inner: Option<WgpuRenderBackendInner>,
    pipelines: Mutex<Vec<wgpu::RenderPipeline>>,
    dpi_factor: f32,
    frame_index: u64,
    time: Instant,
    time_last_frame: Instant,
}

impl WgpuRenderBackend {
    pub fn new() -> Self {
        Self {
            instance: wgpu::Instance::new(wgpu::BackendBit::PRIMARY),
            inner: None,
            pipelines: Default::default(),
            dpi_factor: 1.0,
            frame_index: 0,
            time: Instant::now(),
            time_last_frame: Instant::now(),
        }
    }
}

struct WgpuRenderBackendInner {
    surface: wgpu::Surface,
    swapchain_desc: wgpu::SwapChainDescriptor,
    device: wgpu::Device,
    queue: wgpu::Queue,
    swapchain: wgpu::SwapChain,
    vertex_shader: wgpu::ShaderModule,
    pipeline_layout: wgpu::PipelineLayout,
}

impl RenderBackend for WgpuRenderBackend {
    fn init_window(&mut self, window: &dyn Any) {
        let winit_window = window.downcast_ref::<winit::window::Window>().unwrap();

        let window_size = winit_window.inner_size();

        let first_init = self.inner.is_none();

        let instance = &self.instance;

        self.dpi_factor = winit_window.scale_factor() as f32;

        let inner = self.inner.get_or_insert_with(|| {
            let surface = unsafe { instance.create_surface(winit_window) };

            let adapter =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..Default::default()
                }))
                .ok_or_else(|| "Failed to request an adapter")
                .unwrap();

            let display_format = adapter.get_swap_chain_preferred_format(&surface).unwrap();

            let (device, queue) = pollster::block_on(adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("wgpu device"),
                    features: wgpu::Features::PUSH_CONSTANTS,
                    limits: wgpu::Limits {
                        max_push_constant_size: std::mem::size_of::<ShadertoyConstants>() as u32,
                        ..Default::default()
                    },
                },
                None,
            ))
            .unwrap();

            device.on_uncaptured_error(|error| log::error!("Captured error: {}", error));

            let swapchain_desc = wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
                format: display_format,
                width: window_size.width,
                height: window_size.height,
                present_mode: wgpu::PresentMode::Fifo,
            };

            let swapchain = device.create_swap_chain(&surface, &swapchain_desc);

            let mut vertex_module_desc = wgpu::include_spirv!("shadertoy_vs.vert.spv");
            vertex_module_desc.flags = wgpu::ShaderFlags::empty();

            let vertex_shader = device.create_shader_module(&vertex_module_desc);

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("pipeline layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[wgpu::PushConstantRange {
                    stages: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    range: 0..std::mem::size_of::<ShadertoyConstants>() as u32,
                }],
            });

            WgpuRenderBackendInner {
                surface,
                swapchain_desc,
                device,
                queue,
                swapchain,
                vertex_shader,
                pipeline_layout,
            }
        });

        if !first_init {
            inner.swapchain_desc.width = window_size.width;
            inner.swapchain_desc.height = window_size.height;
            inner.swapchain = inner
                .device
                .create_swap_chain(&inner.surface, &inner.swapchain_desc);
        }
    }

    fn render_frame(&mut self, params: RenderParams<'_>) {
        let inner = self
            .inner
            .as_ref()
            .expect("A window should have been initialised by this point");

        if let Ok(frame) = inner.swapchain.get_current_frame() {
            let mut command_encoder =
                inner
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("command encoder"),
                    });

            let (clear_r, clear_g, clear_b, clear_a) = params.clear_color;

            let pipelines = self.pipelines.lock().unwrap();

            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.output.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_r as f64,
                            g: clear_g as f64,
                            b: clear_b as f64,
                            a: clear_a as f64,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            let w = inner.swapchain_desc.width as f32;
            let h = inner.swapchain_desc.height as f32;

            for quad in params.quads {
                let constants = {
                    let time = self.time.elapsed().as_fractional_secs() as f32;
                    let delta_time = self.time_last_frame.elapsed().as_fractional_secs() as f32;

                    let dt: DateTime<Local> = Local::now();

                    let mut mouse = [
                        (params.mouse_pos.0 as f32) / self.dpi_factor,
                        (params.mouse_pos.1 as f32) / self.dpi_factor,
                        (params.mouse_click_pos.0 as f32) / self.dpi_factor,
                        (params.mouse_click_pos.1 as f32) / self.dpi_factor,
                    ];

                    // flip y
                    if mouse[1] > 0.0 {
                        mouse[1] = h - mouse[1];
                    }
                    if mouse[3] > 0.0 {
                        mouse[3] = h - mouse[3];
                    }

                    ShadertoyConstants {
                        iResolution: [(quad.size.0 * w), (quad.size.1 * h), w / h],
                        pad1: 0.0,
                        iMouse: mouse,
                        iTime: time,
                        iTimeDelta: delta_time,
                        iFrameRate: 1.0 / delta_time,
                        iSampleRate: 44100.0,
                        iFrame: self.frame_index as i32,
                        pad2: [0; 3],
                        iDate: [
                            dt.year() as f32,
                            dt.month() as f32,
                            dt.day() as f32,
                            dt.second() as f32, // TODO unclear what seconds should be here?
                        ],
                        iChannelTime: [time; 4], // TODO not correct
                        iChannelResolution: [[0.0; 4], [0.0; 4], [0.0; 4], [0.0; 4]],
                        iBlockOffset: 0.0,
                        pad3: [0.0; 3],
                    }
                };

                let pipeline = &pipelines[quad.pipeline_handle];

                render_pass.set_pipeline(&pipeline);
                render_pass.set_push_constants(
                    wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                    0,
                    bytemuck::bytes_of(&constants),
                );
                render_pass.set_viewport(
                    quad.pos.0 * w,
                    quad.pos.1 * h,
                    quad.size.0 * w,
                    quad.size.1 * h,
                    0.0,
                    1.0,
                );
                render_pass.draw(0..3, 0..1);
            }

            drop(render_pass);

            inner
                .queue
                .submit(std::iter::once(command_encoder.finish()));
        }

        self.frame_index += 1;
        self.time_last_frame = Instant::now();
    }

    fn new_pipeline(&self, shader_path: &str, shader_source: &str) -> Result<RenderPipelineHandle> {
        let inner = self
            .inner
            .as_ref()
            .expect("A window should have been initialised by this point");

        let spv_path = PathBuf::from(format!("{}.spv", shader_path));
        let mut buf = Vec::new();

        let spv_source = if let Ok(mut file) = File::open(&spv_path) {
            file.read_to_end(&mut buf)
                .chain_err(|| "failed reading spir-v shader file")?;

            wgpu::util::make_spirv(&buf)
        } else {
            let mut compiler = shaderc::Compiler::new().unwrap();
            let options = shaderc::CompileOptions::new().unwrap();

            let binary_result = compiler
                .compile_into_spirv(
                    shader_source,
                    shaderc::ShaderKind::Fragment,
                    "unknown name",
                    "main",
                    Some(&options),
                )
                .chain_err(|| "shaderc compilation to SPIRV failed")?;

            write_file(&spv_path, binary_result.as_binary_u8())?;

            wgpu::ShaderSource::SpirV(std::borrow::Cow::Owned(binary_result.as_binary().to_vec()))
        };

        let fragment_shader = inner
            .device
            .create_shader_module(&wgpu::ShaderModuleDescriptor {
                label: Some(shader_path),
                source: spv_source,
                flags: wgpu::ShaderFlags::empty(),
            });

        let pipeline = inner
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(&format!("render pipeline for {}", shader_path)),
                layout: Some(&inner.pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &inner.vertex_shader,
                    entry_point: "main",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader,
                    entry_point: "main",
                    targets: &[inner.swapchain_desc.format.into()],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
            });

        let mut pipelines = self.pipelines.lock().unwrap();

        let index = pipelines.len();

        pipelines.push(pipeline);

        Ok(index)
    }
}

fn write_file<P: AsRef<Path>>(path: P, buf: &[u8]) -> Result<()> {
    if let Some(parent_path) = path.as_ref().parent() {
        std::fs::create_dir_all(parent_path)?;
    }

    let mut file = File::create(&path)?;
    file.write_all(buf)?;
    Ok(())
}

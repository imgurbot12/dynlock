//! Complete Wgpu State Definition

use std::{ptr::NonNull, time::SystemTime};

use raw_window_handle::{
    RawDisplayHandle, RawWindowHandle, WaylandDisplayHandle, WaylandWindowHandle,
};
use smithay_client_toolkit::session_lock::SessionLockSurface;
use wayland_client::{Connection, Proxy};

use super::{screenshot::Screenshot, ui::IcedState, FRAG_SHADER, VERT_SHADER};

pub const PUSH_CONSTANTS_SIZE: u32 = std::mem::size_of::<FrameUniforms>() as u32;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniforms {
    elapsed: f32,
    fade_amount: f32,
    resolution: [f32; 2],
}

impl FrameUniforms {
    fn new(ctx: &RenderContext) -> Self {
        let duration = SystemTime::now().duration_since(ctx.start).unwrap();
        Self {
            elapsed: duration.as_secs_f32(),
            fade_amount: 0.0,
            resolution: [ctx.width as f32, ctx.height as f32],
        }
    }
}

pub struct RenderContext {
    width: usize,
    height: usize,
    start: SystemTime,
    fade_amount: f32,
}

impl RenderContext {
    fn new() -> Self {
        Self {
            width: 256,
            height: 256,
            start: SystemTime::now(),
            fade_amount: 0.0,
        }
    }
}

/// Graphics State Tracker
pub struct State<'a> {
    format: wgpu::TextureFormat,
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    surface: wgpu::Surface<'a>,
    context: RenderContext,
    iced: IcedState,
}

impl<'a> State<'a> {
    /// Build Wgpu State Instance
    pub async fn new(
        conn: &Connection,
        rgba: Screenshot,
        lock_surface: &SessionLockSurface,
    ) -> Self {
        // spawn wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        // build surface from raw wayland handles
        let raw_display_handle = RawDisplayHandle::Wayland(WaylandDisplayHandle::new(
            NonNull::new(conn.backend().display_ptr() as *mut _).unwrap(),
        ));
        let raw_window_handle = RawWindowHandle::Wayland(WaylandWindowHandle::new(
            NonNull::new(lock_surface.wl_surface().id().as_ptr() as *mut _).unwrap(),
        ));
        let surface = unsafe {
            instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle,
                    raw_window_handle,
                })
                .unwrap()
        };
        // build device/queue from adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Wgpu Init: Adapter Failed");
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::PUSH_CONSTANTS,
                    required_limits: wgpu::Limits {
                        max_push_constant_size: PUSH_CONSTANTS_SIZE,
                        ..Default::default()
                    },
                },
                Some(std::path::PathBuf::from("/home/andrew/Code/rust/dynlock/trace").as_path()),
            )
            .await
            .expect("Wgpu Init: Device/Queue Failed");

        // compile shader components
        let compiler = shaderc::Compiler::new().expect("Shader Init: Compiler Failed");
        let vs_spirv = compiler
            .compile_into_spirv(
                VERT_SHADER,
                shaderc::ShaderKind::Vertex,
                "shader.vert",
                "main",
                None,
            )
            .expect("Shader Init: Vertex Shader Failed");
        let fs_spirv = compiler
            .compile_into_spirv(
                FRAG_SHADER,
                shaderc::ShaderKind::Fragment,
                "shader.frag",
                "main",
                None,
            )
            .expect("Shader Init: Fragment Shader Failed");
        let vs_data = wgpu::util::make_spirv(vs_spirv.as_binary_u8());
        let fs_data = wgpu::util::make_spirv(fs_spirv.as_binary_u8());
        let vs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Vertex Shader"),
            source: vs_data,
        });
        let fs_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fragment Shader"),
            source: fs_data,
        });
        // build bind group
        let screenshot = super::screenshot::screenshot(rgba, &device, &queue);
        let screenshot_view = screenshot.create_view(&wgpu::TextureViewDescriptor::default());
        let screenshot_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // This should match the filterable field of the
                    // corresponding Texture entry above.
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&screenshot_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&screenshot_sampler),
                },
            ],
            label: Some("diffuse_bind_group"),
        });
        // build rendering pipeline
        let capabilities = surface.get_capabilities(&adapter);
        let texture_format = capabilities.formats[0];
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[wgpu::PushConstantRange {
                    stages: wgpu::ShaderStages::FRAGMENT,
                    range: 0..PUSH_CONSTANTS_SIZE,
                }],
            });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: "main",
                buffers: &[],
            },
            multiview: None,
            fragment: Some(wgpu::FragmentState {
                module: &fs_module,
                entry_point: "main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: texture_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
        });
        // spawn iced components
        let iced = IcedState::new(&adapter, &device, &queue, texture_format);
        // return compiled state object
        Self {
            format: texture_format,
            device,
            queue,
            render_pipeline,
            bind_group,
            surface,
            context: RenderContext::new(),
            iced,
        }
    }

    pub fn configure(&mut self, width: u32, height: u32) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.format,
            view_formats: vec![self.format],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width,
            height,
            desired_maximum_frame_latency: 2,
            // Wayland is inherently a mailbox system.
            present_mode: wgpu::PresentMode::Mailbox,
        };
        self.context.width = width as usize;
        self.context.height = height as usize;
        self.surface.configure(&self.device, &surface_config);
        self.iced.configure(width, height);
    }

    pub fn key_event(&mut self, event: iced_runtime::core::keyboard::Event) {
        self.iced.key_event(event);
    }

    pub fn mouse_event(&mut self, event: iced_runtime::core::mouse::Event) {
        self.iced.mouse_event(event);
    }

    #[inline]
    pub fn is_authenticated(&self) -> bool {
        return self.iced.is_authenticated();
    }

    pub fn render(&mut self) {
        // prepare texture from surface
        let surface_texture = self
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        // build renderpass for texture
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let render_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            };
            let mut render_pass = encoder.begin_render_pass(&render_pass_desc);

            // render shaders with uniforms and constants
            let constants = FrameUniforms::new(&self.context);
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_push_constants(
                wgpu::ShaderStages::FRAGMENT,
                0,
                bytemuck::bytes_of(&constants),
            );
            render_pass.draw(0..6, 0..1);
        }
        // submit rendering for final generation
        self.iced
            .render(&self.device, &self.queue, encoder, &texture_view);
        // self.queue.submit(Some(encoder.finish()));
        surface_texture.present();
    }
}

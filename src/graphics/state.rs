//! Complete Wgpu State Definition

use std::time::SystemTime;

use super::{inner::InnerState, BYTES_PER_PIXEL, FRAG_SHADER, TEXTURE_FORMAT, VERT_SHADER};

pub const PUSH_CONSTANTS_SIZE: u32 = std::mem::size_of::<FrameUniforms>() as u32;

/// Single Frame Rendering Data
#[derive(Debug)]
pub struct Frame {
    pub width: i32,
    pub height: i32,
    pub content: Vec<u8>,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniforms {
    elapsed: f32,
    fade_amount: f32,
    resolution: [f32; 2],
}

impl FrameUniforms {
    fn new(ctx: &RenderContext, width: u32, height: u32) -> Self {
        let duration = SystemTime::now().duration_since(ctx.start).unwrap();
        Self {
            elapsed: duration.as_secs_f32(),
            fade_amount: 0.0,
            resolution: [width as f32, height as f32],
        }
    }
}

pub struct RenderContext {
    start: SystemTime,
    fade_amount: f32,
}

impl RenderContext {
    fn new() -> Self {
        Self {
            start: SystemTime::now(),
            fade_amount: 0.0,
        }
    }
}

/// Graphics State Tracker
pub struct State<'a> {
    pub device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    inner: Option<InnerState<'a>>,
    context: RenderContext,
}

impl<'a> State<'a> {
    /// Build Wgpu State Instance
    pub async fn new(conn: wayland_client::Connection) -> Self {
        // spawn wgpu instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
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
        let screenshot = super::screenshot::screenshot(conn, &device, &queue);
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
                    format: TEXTURE_FORMAT,
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

        // return compiled state object
        Self {
            device,
            queue,
            render_pipeline,
            bind_group,
            inner: None,
            context: RenderContext::new(),
        }
    }

    //TODO: move reusable elements into separate option<state> object
    pub async fn render(&mut self, width: u32, height: u32) -> Frame {
        if self.inner.is_none() {
            self.inner = Some(InnerState::new(width, height, &self.device));
        }
        let inner = self.inner.as_ref().unwrap();
        let output_buffer = self.device.create_buffer(&inner.output_buffer_desc);
        // build renderpass for texture
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let render_pass_desc = wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &inner.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            };
            let mut render_pass = encoder.begin_render_pass(&render_pass_desc);

            let constants = FrameUniforms::new(&self.context, width, height);
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_push_constants(
                wgpu::ShaderStages::FRAGMENT,
                0,
                bytemuck::bytes_of(&constants),
            );

            render_pass.draw(0..6, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &inner.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(inner.padded_bytes_per_row as u32),
                    rows_per_image: Some(height),
                },
            },
            inner.texture_size,
        );
        // Submit the command in the queue to execute
        self.queue.submit(Some(encoder.finish()));
        //
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.receive().await.unwrap().unwrap();
        // map buffered data back to standard size
        // and convert rgba8888 to argb8888
        let mut data = buffer_slice.get_mapped_range_mut();
        let mut content = vec![];
        for chunk in data.chunks_mut(inner.padded_bytes_per_row) {
            let chunk = &mut chunk[..inner.unpadded_bytes_per_row];
            let chunk: Vec<u8> = chunk
                .chunks_exact(BYTES_PER_PIXEL as usize)
                .map(|s| {
                    let pixel = u32::from_le_bytes(s.try_into().unwrap());
                    let r = (pixel & 0xff000000) >> 24;
                    let g = (pixel & 0x00ff0000) >> 16;
                    let b = (pixel & 0x0000ff00) >> 8;
                    let a = pixel & 0x000000ff;
                    let new_pixel = a << 24 | r << 16 | g << 8 | b;
                    new_pixel.to_ne_bytes()
                })
                .flatten()
                .collect();
            content.extend_from_slice(&chunk);
        }
        Frame {
            width: width as i32,
            height: height as i32,
            content,
        }
    }
}

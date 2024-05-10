//! WGPU Integration with LockScreen

use rayon::prelude::*;

///TODO: replace with configuration supplied options later
const FRAG_SHADER: &'static str = include_str!("../shaders/shader.frag");
const VERT_SHADER: &'static str = include_str!("../shaders/shader.vert");

const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = std::mem::size_of::<u32>() as u32;

//TODO: replace expects with thiserror/anyhow later

#[derive(Debug)]
pub struct RenderResult {
    pub width: u32,
    pub height: u32,
    pub content: Vec<u8>,
}

struct InnerState<'a> {
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,
    pub texture: wgpu::Texture,
    pub texture_size: wgpu::Extent3d,
    pub texture_view: wgpu::TextureView,
    pub output_buffer_desc: wgpu::BufferDescriptor<'a>,
}

impl<'a> InnerState<'a> {
    fn new(width: u32, height: u32, device: &wgpu::Device) -> Self {
        // fix width to match required size (https://github.com/ggez/ggez/pull/1210)
        let unpadded_bytes_per_row = width as usize * BYTES_PER_PIXEL as usize;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = unpadded_bytes_per_row + padded_bytes_per_row_padding;
        // generate texture
        let texture_desc = wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[TEXTURE_FORMAT],
        };
        let texture = device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&Default::default());
        // build output buffer for texture-size
        let output_buffer_size = (padded_bytes_per_row as u32 * height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        Self {
            unpadded_bytes_per_row,
            padded_bytes_per_row,
            texture,
            texture_size: texture_desc.size,
            texture_view,
            output_buffer_desc,
        }
    }
}

/// Wgpu State Tracker
pub struct WgpuState<'a> {
    // wgpu elements
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_pipeline: wgpu::RenderPipeline,
    inner: Option<InnerState<'a>>,
}

impl<'a> WgpuState<'a> {
    /// Build Wgpu State Instance
    pub async fn new() -> Self {
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
            .request_device(&Default::default(), None)
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
        // build rendering pipeline
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[], // entrypoint for uniform variables like screenshot data
                push_constant_ranges: &[],
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
            inner: None,
        }
    }

    //TODO: move reusable elements into separate option<state> object
    pub async fn render(&mut self, width: u32, height: u32) -> RenderResult {
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
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            };
            let mut render_pass = encoder.begin_render_pass(&render_pass_desc);

            render_pass.set_pipeline(&self.render_pipeline);
            /*
             rp.set_bind_group(0, &self.bind_group, &[]);
             rp.set_push_constants(
                wgpu::ShaderStage::FRAGMENT,
                0,
                bytemuck::cast_slice(&[FrameUniforms::from(ctx)]),
            );
            rp.draw(0..4, 0..1);
            */
            render_pass.draw(0..3, 0..1);
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
            chunk.rotate_right(1);
            content.extend_from_slice(chunk);
        }
        RenderResult {
            width,
            height,
            content,
        }
    }
}

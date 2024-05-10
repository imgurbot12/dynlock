//! WGPU Integration with LockScreen

use rayon::prelude::*;
use wgpu::{Adapter, Device, Queue, ShaderModule, TextureFormat};

///TODO: replace with configuration supplied options later
const FRAG_SHADER: &'static str = include_str!("../shaders/shader.frag");
const VERT_SHADER: &'static str = include_str!("../shaders/shader.vert");

//TODO: replace expects with thiserror/anyhow later

#[derive(Debug)]
pub struct RenderResult {
    pub width: u32,
    pub height: u32,
    pub content: Vec<u8>,
}

/// Wgpu State Tracker
pub struct WgpuState {
    // wgpu elements
    adapter: Adapter,
    device: Device,
    queue: Queue,
    // shader elements
    fs_module: ShaderModule,
    vs_module: ShaderModule,
}

impl WgpuState {
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
        // return compiled state object
        Self {
            adapter,
            device,
            queue,
            fs_module,
            vs_module,
        }
    }

    //TODO: move reusable elements into separate option<state> object
    pub async fn render(&self, width: u32, height: u32) -> RenderResult {
        // fix width to match required size (https://github.com/ggez/ggez/pull/1210)
        let bytes_per_pixel = std::mem::size_of::<u32>() as u32;
        let unpadded_bytes_per_row = width as usize * bytes_per_pixel as usize;
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
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[TextureFormat::Rgba8UnormSrgb],
        };
        let texture = self.device.create_texture(&texture_desc);
        let texture_view = texture.create_view(&Default::default());
        // build output buffer for texture-size
        let output_buffer_size = (padded_bytes_per_row as u32 * height) as wgpu::BufferAddress;
        let output_buffer_desc = wgpu::BufferDescriptor {
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST
        // this tells wpgu that we want to read this buffer from the cpu
        | wgpu::BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = self.device.create_buffer(&output_buffer_desc);
        // build renderpass for texture
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let _renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row as u32),
                    rows_per_image: Some(height),
                },
            },
            texture_desc.size,
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
        // and convert rgba8888 to argb8888 :/
        let data = buffer_slice.get_mapped_range();
        let content = data
            .par_chunks(padded_bytes_per_row)
            .map(|chunk| {
                chunk[..unpadded_bytes_per_row]
                    .chunks_exact(4)
                    .map(|rgba| [rgba[3], rgba[0], rgba[1], rgba[2]])
                    .flatten()
                    .collect::<Vec<u8>>()
            })
            .flatten()
            .collect();
        RenderResult {
            width,
            height,
            content,
        }
    }
}

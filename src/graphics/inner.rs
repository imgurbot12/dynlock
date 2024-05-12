//! Inner Recalculated Wgpu State Values (based on resize)

use super::{BYTES_PER_PIXEL, TEXTURE_FORMAT};

/// Inner Potentially Recalculated Wgpu State (on Resize)
pub struct InnerState<'a> {
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: usize,
    pub texture: wgpu::Texture,
    pub texture_size: wgpu::Extent3d,
    pub texture_view: wgpu::TextureView,
    pub output_buffer_desc: wgpu::BufferDescriptor<'a>,
}

impl<'a> InnerState<'a> {
    pub fn new(width: u32, height: u32, device: &wgpu::Device) -> Self {
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

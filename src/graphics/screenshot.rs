//! Wgpu Rendering BindGroup

use wgpu::util::DeviceExt;

pub type Background = image::ImageBuffer<image::Rgba<u8>, Vec<u8>>;

/// Take Screenshot and make Wgpu Texture
pub fn screenshot(rgba: Background, device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    // build wgpu texture from image
    let dimensions = rgba.dimensions();
    let texture_size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };
    println!("dimensions {dimensions:?}");

    let texture = device.create_texture_with_data(
        queue,
        &wgpu::TextureDescriptor {
            // All textures are stored as 3D, we represent our 2D texture
            // by setting depth to 1.
            size: texture_size,
            mip_level_count: 1, // We'll talk about this a little later
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Most images are stored using sRGB, so we need to reflect that here.
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
            // COPY_DST means that we want to copy data to this texture
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("screenshot_texture"),
            // This is the same as with the SurfaceConfig. It
            // specifies what texture formats can be used to
            // create TextureViews for this texture. The base
            // texture format (Rgba8UnormSrgb in this case) is
            // always supported. Note that using a different
            // texture format is not supported on the WebGL2
            // backend.
            view_formats: &[],
        },
        wgpu::util::TextureDataOrder::MipMajor,
        &rgba,
    );
    texture
}

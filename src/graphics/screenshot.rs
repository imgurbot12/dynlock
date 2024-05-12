//! Wgpu Rendering BindGroup

use super::BYTES_PER_PIXEL;

/// Take Screenshot and make Wgpu Texture
pub fn screenshot(
    conn: wayland_client::Connection,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> wgpu::Texture {
    // get raw screenshot image buffer
    // let wayshot = libwayshot::WayshotConnection::from_connection(conn.clone())
    //     .expect("screenshot connection failed");
    // let rgba = wayshot.screenshot_all(false).expect("screenshot failed");
    let img = include_bytes!("../../happy-tree.png");
    let rgba = image::load_from_memory(img)
        .expect("failed to load kitty")
        .to_rgba8();
    // build wgpu texture from image
    let dimensions = rgba.dimensions();
    let texture_size = wgpu::Extent3d {
        width: dimensions.0,
        height: dimensions.1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
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
    });
    queue.write_texture(
        // Tells wgpu where to copy the pixel data
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        // The actual pixel data
        &rgba,
        // The layout of the texture
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(BYTES_PER_PIXEL * texture_size.width),
            rows_per_image: Some(texture_size.height),
        },
        texture_size,
    );
    texture
}

//! WGPU Rendering Implementation

mod inner;
mod screenshot;
mod state;

const FRAG_SHADER: &'static str = include_str!("../../shaders/shader.frag");
const VERT_SHADER: &'static str = include_str!("../../shaders/shader.vert");

const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
pub const BYTES_PER_PIXEL: u32 = std::mem::size_of::<u32>() as u32;

pub use state::{Frame, State, PUSH_CONSTANTS_SIZE};

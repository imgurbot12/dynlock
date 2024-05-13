//! WGPU Rendering Implementation

mod screenshot;
mod state;
mod ui;

const FRAG_SHADER: &'static str = include_str!("../../shaders/shader.frag");
const VERT_SHADER: &'static str = include_str!("../../shaders/shader.vert");

pub use screenshot::Screenshot;
pub use state::State;

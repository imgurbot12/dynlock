mod graphics;
mod lock;
mod wgpu_old;

fn main() {
    lock::runlock();
    // pollster::block_on(wgpu::setup());
}

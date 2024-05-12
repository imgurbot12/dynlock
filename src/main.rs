mod graphics;
mod lock;

fn main() {
    lock::runlock();
    // pollster::block_on(wgpu::setup());
}

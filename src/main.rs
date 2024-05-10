mod lock;
mod wgpu;
mod wgpu_old;

fn main() {
    lock::runlock();
    // pollster::block_on(wgpu::setup());
}

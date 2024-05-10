//! Smithay Wayland LockScreen Generation and Runtime
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use calloop::futures::Scheduler;
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;

use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::session_lock::{SessionLock, SessionLockHandler, SessionLockState};
use smithay_client_toolkit::session_lock::{SessionLockSurface, SessionLockSurfaceConfigure};
use smithay_client_toolkit::shm::{raw::RawPool, Shm, ShmHandler};

use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::{wl_buffer, wl_output, wl_shm, wl_surface};
use wayland_client::{Connection, Proxy, QueueHandle};

use raw_window_handle::{
    HasRawDisplayHandle, HasRawWindowHandle, RawDisplayHandle, RawWindowHandle,
    WaylandDisplayHandle, WaylandWindowHandle,
};

use crate::wgpu::{RenderResult, WgpuState};

struct AppData {
    loop_handle: LoopHandle<'static, Self>,
    conn: Connection,

    qh: QueueHandle<Self>,
    compositor_state: CompositorState,
    output_state: OutputState,
    registry_state: RegistryState,
    shm: Shm,

    session_lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    lock_surfaces: Vec<SessionLockSurface>,

    wgpu: Arc<WgpuState>,
    sched: Scheduler<RenderResult>,
    render: Option<RenderResult>,

    exit: bool,
}

// general flow
//
// 1. prepare wgpu elements and pass to AppData
// 2. prepare lock elements and pass to AppData
// 3. assign lock-surfaces and elements during lock generation in `locked`
// 4. begin drawing after `configure`. wgpu renders to its own interal surface
//    before copying bytes back into lock-surfaces each tick
//
// webviews don't support any means of rendering directly via wgpu or accessing/
// replacing the underlying wayland windows/surfaces.
//
// consider using iced_wgpu to integrate directly via wgpu-layer for basic ui
// integration on top of generated shaders
// https://github.com/iced-rs/iced/blob/master/examples/integration/src/main.rs
//
// the first test should simply be rendering something basic with wgpu
// onto the lockscreen. checking if a steady animation works well at high-fps
// and moving onto more advanced methods from there.

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

pub fn runlock() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();

    let (globals, event_queue) = registry_queue_init(&conn).unwrap();

    let qh: QueueHandle<AppData> = event_queue.handle();

    // prepare event-loop
    let mut event_loop: EventLoop<AppData> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");
    let handle = event_loop.handle();
    let (exec, sched) = calloop::futures::executor().unwrap();
    handle
        .insert_source(exec, |result: RenderResult, _metadata, shared| {
            shared.render = Some(result);
        })
        .map_err(|e| e.error)
        .unwrap();
    // prepare application state
    let wgpu = pollster::block_on(WgpuState::new());
    let mut app_data = AppData {
        loop_handle: event_loop.handle(),
        conn: conn.clone(),
        compositor_state: CompositorState::bind(&globals, &qh).unwrap(),
        output_state: OutputState::new(&globals, &qh),
        registry_state: RegistryState::new(&globals),
        shm: Shm::bind(&globals, &qh).unwrap(),
        session_lock_state: SessionLockState::new(&globals, &qh),
        session_lock: None,
        lock_surfaces: Vec::new(),
        exit: false,
        wgpu: Arc::new(wgpu),
        sched,
        render: None,
        qh: qh.to_owned(),
    };

    app_data.session_lock = Some(
        app_data
            .session_lock_state
            .lock(&qh)
            .expect("ext-session-lock not supported"),
    );

    WaylandSource::new(conn.clone(), event_queue)
        .insert(event_loop.handle())
        .unwrap();

    let signal = event_loop.get_signal();
    event_loop
        .run(
            std::time::Duration::from_millis(16),
            &mut app_data,
            |app_data| {
                if let Some(result) = app_data.render.as_ref() {
                    println!("frame!");
                    // update lockscreen with current result
                    let width = result.width;
                    let height = result.height;
                    app_data.render(&qh, result);
                    app_data.render = None;
                    // schedule rendering for next frame
                    let wgpu = Arc::clone(&app_data.wgpu);
                    let test = async move { wgpu.render(width, height).await };
                    app_data.sched.schedule(test).unwrap();
                }
                // println!("frame! {:?}", qh);
                if app_data.exit {
                    signal.stop();
                }
            },
        )
        .expect("Error during event loop!");
}

impl AppData {
    fn render(&self, qh: &QueueHandle<Self>, result: &RenderResult) {
        // make buffer object
        let mut pool = RawPool::new(
            result.width as usize * result.height as usize * 4,
            &self.shm,
        )
        .unwrap();
        let canvas = pool.mmap();
        canvas.copy_from_slice(&result.content);
        let buffer = pool.create_buffer(
            0,
            result.width as i32,
            result.height as i32,
            result.width as i32 * 4,
            wl_shm::Format::Argb8888,
            (),
            qh,
        );
        // update surfaces with render-result
        for surface in self.lock_surfaces.iter() {
            let wl = surface.wl_surface();
            wl.attach(Some(&buffer), 0, 0);
            wl.commit();
        }
        buffer.destroy();
    }
}

impl SessionLockHandler for AppData {
    fn locked(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, session_lock: SessionLock) {
        for output in self.output_state.outputs() {
            let surface = self.compositor_state.create_surface(&qh);
            let lock_surface = session_lock.create_lock_surface(surface, &output, qh);
            self.lock_surfaces.push(lock_surface);
        }

        // After 5 seconds, destroy lock
        self.loop_handle
            .insert_source(
                Timer::from_duration(Duration::from_secs(3)),
                |_, _, app_data| {
                    // Unlock the lock
                    app_data.session_lock.take().unwrap().unlock();
                    // Sync connection to make sure compostor receives destroy
                    app_data.conn.roundtrip().unwrap();
                    // Then we can exit
                    app_data.exit = true;
                    TimeoutAction::Drop
                },
            )
            .unwrap();
    }

    fn finished(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _session_lock: SessionLock,
    ) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        session_lock_surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;

        let wgpu = Arc::clone(&self.wgpu);
        let test = async move { wgpu.render(width, height).await };
        self.sched.schedule(test).unwrap();

        // let s = session_lock_surface.wl_surface();
        //
        // let mut pool = RawPool::new(width as usize * height as usize * 4, &self.shm).unwrap();
        // let canvas = pool.mmap();
        // canvas
        //     .chunks_exact_mut(4)
        //     .enumerate()
        //     .for_each(|(index, chunk)| {
        //         // let x = (index % width as usize) as u32;
        //         // let y = (index / width as usize) as u32;
        //         //
        //         // let a = 0xFF;
        //         // let r = u32::min(((width - x) * 0xFF) / width, ((height - y) * 0xFF) / height);
        //         // let g = u32::min((x * 0xFF) / width, ((height - y) * 0xFF) / height);
        //         // let b = u32::min(((width - x) * 0xFF) / width, (y * 0xFF) / height);
        //         // let color = (a << 24) + (r << 16) + (g << 8) + b;
        //
        //         let color: u32 = 0xB6CFB6;
        //         // let color: u32 = 0xff0000db;
        //
        //         let array: &mut [u8; 4] = chunk.try_into().unwrap();
        //         *array = color.to_le_bytes();
        //     });
        // let buffer = pool.create_buffer(
        //     0,
        //     width as i32,
        //     height as i32,
        //     width as i32 * 4,
        //     wl_shm::Format::Argb8888,
        //     (),
        //     qh,
        // );
        //
        // session_lock_surface
        //     .wl_surface()
        //     .attach(Some(&buffer), 0, 0);
        // session_lock_surface.wl_surface().commit();
        //
        // buffer.destroy();
    }
}

impl CompositorHandler for AppData {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }
}

impl OutputHandler for AppData {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl ProvidesRegistryState for AppData {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState,];
}

impl ShmHandler for AppData {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

smithay_client_toolkit::delegate_compositor!(AppData);
smithay_client_toolkit::delegate_output!(AppData);
smithay_client_toolkit::delegate_session_lock!(AppData);
smithay_client_toolkit::delegate_shm!(AppData);
smithay_client_toolkit::delegate_registry!(AppData);
wayland_client::delegate_noop!(AppData: ignore wl_buffer::WlBuffer);

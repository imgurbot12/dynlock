//! Smithay Wayland LockScreen Generation and Runtime
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use calloop::{EventLoop, LoopHandle};
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;

use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::seat::keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers};
use smithay_client_toolkit::seat::pointer::{PointerEvent, PointerHandler};
use smithay_client_toolkit::seat::Capability;
use smithay_client_toolkit::seat::{SeatHandler, SeatState};
use smithay_client_toolkit::session_lock::{SessionLock, SessionLockHandler, SessionLockState};
use smithay_client_toolkit::session_lock::{SessionLockSurface, SessionLockSurfaceConfigure};
use smithay_client_toolkit::shm::{Shm, ShmHandler};

use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{
    wl_buffer, wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface,
};
use wayland_client::{Connection, Proxy, QueueHandle};

use crate::graphics::{Screenshot, State};

type RenderersMap = BTreeMap<u32, State<'static>>;

struct SeatObject {
    seat: wl_seat::WlSeat,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
}

struct AppData {
    exit: bool,
    loop_handle: LoopHandle<'static, Self>,
    // common compositer components
    conn: Connection,
    compositor_state: CompositorState,
    output_state: OutputState,
    registry_state: RegistryState,
    shm: Shm,
    // lockscreen components
    session_lock_state: SessionLockState,
    session_lock: Option<SessionLock>,
    lock_surfaces: Vec<SessionLockSurface>,
    // rendering components
    renderers: Arc<RwLock<RenderersMap>>,
    screenshot: Screenshot,
    // input components
    seat_state: SeatState,
    seat_objects: Vec<SeatObject>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
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

pub fn runlock() {
    env_logger::init();

    let conn = Connection::connect_to_env().unwrap();
    let (globals, event_queue) = registry_queue_init(&conn).unwrap();
    let qh: QueueHandle<AppData> = event_queue.handle();

    // take screenshots of outputs
    // take screenshot of current output (TODO: multimonitor support)
    let wayshot = libwayshot::WayshotConnection::from_connection(conn.clone())
        .expect("screenshot connection failed");
    let screenshot = wayshot.screenshot_all(false).expect("screenshot failed");

    // weird fix to get the screenshot to render properly
    // load in and out of image objects
    let mut b: Vec<u8> = vec![];
    let mut w = std::io::Cursor::new(&mut b);
    screenshot
        .write_to(&mut w, image::ImageFormat::Png)
        .expect("failed to decode screenshot");
    let screenshot = image::load_from_memory(&b)
        .expect("failed to load screenshot")
        .to_rgba8();

    // prepare event-loop
    let mut event_loop: EventLoop<AppData> =
        EventLoop::try_new().expect("Failed to initialize the event loop!");

    let mut app_data = AppData {
        exit: false,
        loop_handle: event_loop.handle(),
        // compositor components
        conn: conn.clone(),
        compositor_state: CompositorState::bind(&globals, &qh).unwrap(),
        output_state: OutputState::new(&globals, &qh),
        registry_state: RegistryState::new(&globals),
        shm: Shm::bind(&globals, &qh).unwrap(),
        // lockscreen components
        session_lock_state: SessionLockState::new(&globals, &qh),
        session_lock: None,
        lock_surfaces: Vec::new(),
        // rendering components
        renderers: Arc::new(RwLock::new(RenderersMap::new())),
        screenshot,
        // input management components
        seat_state: SeatState::new(&globals, &qh),
        seat_objects: vec![],
        keyboard: None,
        pointer: None,
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

    let fps = 30;
    let dist = 1000 / fps;
    let handle = event_loop.handle();
    handle
        .insert_source(
            Timer::from_duration(Duration::from_millis(dist)),
            move |_, _, app_data| {
                let arc = Arc::clone(&app_data.renderers);
                let mut renderers = arc.write().expect("renderer write lock failed");
                for renderer in renderers.values_mut() {
                    renderer.render();
                }
                TimeoutAction::ToDuration(Duration::from_millis(dist))
            },
        )
        .expect("failed to schedule rendering loop");

    let signal = event_loop.get_signal();
    event_loop
        .run(
            std::time::Duration::from_millis(dist * 2),
            &mut app_data,
            |app_data| {
                // handle exit when specified
                if app_data.exit {
                    app_data.session_lock.take().unwrap().unlock();
                    app_data.conn.roundtrip().unwrap();
                    signal.stop();
                }
            },
        )
        .expect("Error during event loop!");
}

impl SessionLockHandler for AppData {
    fn locked(&mut self, conn: &Connection, qh: &QueueHandle<Self>, session_lock: SessionLock) {
        // prepare sufaces and renderers for lockscreen
        let arc = Arc::clone(&self.renderers);
        let mut renderers = arc.write().expect("renderer write lock failed");
        for output in self.output_state.outputs() {
            // generate wayland surfaces
            let surface = self.compositor_state.create_surface(&qh);
            let lock_surface = session_lock.create_lock_surface(surface, &output, qh);
            // generate wgpu renderer for surface
            let key = lock_surface.wl_surface().id().protocol_id();
            let screenshot = self.screenshot.clone();
            let renderer = pollster::block_on(State::new(conn, screenshot, &lock_surface));
            renderers.insert(key, renderer);
            self.lock_surfaces.push(lock_surface);
        }
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
        _qh: &QueueHandle<Self>,
        session_lock_surface: SessionLockSurface,
        configure: SessionLockSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;

        // let wgpu = pollster::block_on(State::new(conn.clone()));

        let arc = Arc::clone(&self.renderers);
        let key = session_lock_surface.wl_surface().id().protocol_id();
        let mut renderers = arc.write().expect("renderer write lock failed");
        let renderer = renderers.get_mut(&key).unwrap();
        renderer.configure(width, height);
        renderer.render();
    }
}

impl SeatHandler for AppData {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        let seat_object = match self.seat_objects.iter_mut().find(|s| s.seat == seat) {
            Some(seat) => seat,
            None => {
                self.seat_objects.push(SeatObject {
                    seat: seat.clone(),
                    keyboard: None,
                    pointer: None,
                });
                self.seat_objects.last_mut().unwrap()
            }
        };
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("Failed to create keyboard");
            self.keyboard = Some(keyboard.clone());
            seat_object.keyboard.replace(keyboard);
        }
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self
                .seat_state
                .get_pointer(qh, &seat)
                .expect("Failed to create pointer");
            self.pointer = Some(pointer.clone());
            seat_object.pointer.replace(pointer);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }
        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for AppData {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _keysyms: &[Keysym],
    ) {
        println!("keyboard enter!");
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _: u32,
    ) {
        println!("keyboard exit!");
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        if event.keysym == Keysym::Escape {
            log::info!("escape key pressed. exiting!");
            self.exit = true;
        }
        println!("keyboard {event:?}");
    }

    fn release_key(
        &mut self,
        _: &Connection,
        _qh: &QueueHandle<Self>,
        _kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _layout: u32,
    ) {
        println!("modifiers! {modifiers:?}");
    }
}

impl PointerHandler for AppData {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        _events: &[PointerEvent],
    ) {
        // println!("pointer events: {events:?}")
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

smithay_client_toolkit::delegate_seat!(AppData);
smithay_client_toolkit::delegate_keyboard!(AppData);
smithay_client_toolkit::delegate_pointer!(AppData);
smithay_client_toolkit::delegate_compositor!(AppData);
smithay_client_toolkit::delegate_output!(AppData);
smithay_client_toolkit::delegate_session_lock!(AppData);
smithay_client_toolkit::delegate_shm!(AppData);
smithay_client_toolkit::delegate_registry!(AppData);
wayland_client::delegate_noop!(AppData: ignore wl_buffer::WlBuffer);

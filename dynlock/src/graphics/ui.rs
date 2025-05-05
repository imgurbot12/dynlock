//! Iced UI Implementation
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use iced_runtime::command::Action;
use iced_runtime::core::keyboard;
use iced_runtime::{program::State, Debug, Font};

use iced_wgpu::core::alignment::Vertical;
use iced_wgpu::core::Point;
use iced_wgpu::core::{mouse, renderer, Clipboard, Color, Event, Length, Pixels, Size};
use iced_wgpu::graphics::Viewport;
use iced_wgpu::Settings;
use iced_wgpu::{wgpu, Backend, Renderer};

use iced_widget::{container, Column, Row, Theme};

use super::style;

const CAPS_LOCK_ICON: &'static [u8] = include_bytes!("../../icons/caps-lock.png");
const HIDE_ICON: &'static [u8] = include_bytes!("../../icons/hide.png");
const SHOW_ICON: &'static [u8] = include_bytes!("../../icons/show.png");

// keyboard handling utilities
const TAB: keyboard::Key = keyboard::Key::Named(keyboard::key::Named::Tab);
const ESCAPE: keyboard::Key = keyboard::Key::Named(keyboard::key::Named::Escape);
const CAPS_LOCK: keyboard::Key = keyboard::Key::Named(keyboard::key::Named::CapsLock);
const HOLD_KEY_TIMEOUT: Duration = Duration::from_millis(200);

/// Lockscreen UI Implementation
pub struct UI {
    input_id: iced_widget::text_input::Id,
    caps_img: iced_widget::image::Handle,
    show_img: iced_widget::image::Handle,
    hide_img: iced_widget::image::Handle,
    username: String,
    password: String,
    caps_lock: bool,
    hide_input: bool,
    auth_thread: Option<std::thread::JoinHandle<()>>,
    authenticated: Arc<Mutex<bool>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Typing(String),
    Submit,
    Focus,
    Reset,
    ToggleShow,
    CapsLock(bool),
}

impl UI {
    pub fn new() -> Self {
        let input_id = iced_widget::text_input::Id::unique();
        let caps_img = iced_widget::image::Handle::from_memory(CAPS_LOCK_ICON);
        let hide_img = iced_widget::image::Handle::from_memory(HIDE_ICON);
        let show_img = iced_widget::image::Handle::from_memory(SHOW_ICON);
        Self {
            input_id,
            caps_img,
            show_img,
            hide_img,
            username: whoami::username(),
            password: "".to_owned(),
            hide_input: true,
            caps_lock: false,
            auth_thread: None,
            authenticated: Arc::new(Mutex::new(false)),
        }
    }
    /// Check if Authentication Thread is Running
    #[inline]
    fn auth_running(&self) -> bool {
        self.auth_thread
            .as_ref()
            .map(|t| !t.is_finished())
            .unwrap_or(false)
    }
    /// Spawn Authentication Thread (if not already running)
    fn start_authenticate(&mut self) {
        // skip authenticating if already in progress
        if self.auth_running() {
            log::error!("requested auth while already authenticating");
            return;
        }
        // spawn thread to complete login attempt in background
        let username = self.username.to_owned();
        let password = self.password.to_owned();
        let authenticated = Arc::clone(&self.authenticated);
        self.auth_thread = Some(std::thread::spawn(move || {
            // attempt login via pam
            let mut client =
                pam::Client::with_password("system-auth").expect("Failed to init PAM client.");
            client
                .conversation_mut()
                .set_credentials(username, password);
            let auth_result = client.authenticate().is_ok();
            // update authentication status
            let mut auth = authenticated.lock().expect("mutex lock failed");
            *auth = auth_result;
        }));
    }
    /// Check if Successfully Authenticated
    #[inline]
    fn is_authenticated(&self) -> bool {
        *self.authenticated.lock().expect("mutex lock failed")
    }
}

impl iced_runtime::Program for UI {
    type Theme = Theme;
    type Message = Message;
    type Renderer = iced_wgpu::Renderer;

    fn view(&self) -> iced_runtime::core::Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        // password form
        let mut password =
            iced_widget::text_input("Type password to unlock...", self.password.as_str())
                .id(self.input_id.clone())
                .secure(self.hide_input)
                .width(Length::Fixed(300.0))
                .padding(5)
                .size(12.0)
                .style(style::password());
        if !self.auth_running() {
            password = password
                .on_input(Message::Typing)
                .on_submit(Message::Submit);
        }
        // build input controls
        let size = 15.0;
        let img = if self.hide_input {
            self.hide_img.clone()
        } else {
            self.show_img.clone()
        };
        let show = iced_widget::Button::new(
            iced_widget::Image::new(img)
                .width(Length::Fixed(size))
                .height(Length::Fixed(size)),
        )
        .on_press(Message::ToggleShow)
        .style(style::show());

        let mut controls = Row::new().push(password);
        let caps = if self.caps_lock {
            let caps = iced_widget::Image::new(self.caps_img.clone())
                .width(Length::Fixed(size))
                .height(Length::Fixed(size));
            iced_widget::Button::new(caps).style(style::show())
        } else {
            let empty = iced_widget::text("")
                .width(Length::Fixed(size))
                .height(Length::Fixed(size));
            iced_widget::Button::new(empty).style(style::show())
        };
        controls = controls.push(caps).push(show);

        // construct menu
        let now = chrono::Local::now();
        let message = iced_widget::text(now.format("%H:%M:%S")).size(32.0);
        let menu = Column::new()
            .push(message)
            .push(controls)
            .align_items(iced_wgpu::core::Alignment::Start);
        let menu_box = container(menu).padding(10).style(style::menubox());
        container(menu_box)
            .padding(25)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Vertical::Bottom)
            .into()
    }
    fn update(&mut self, message: Self::Message) -> iced_runtime::Command<Self::Message> {
        match message {
            Message::Typing(e) => self.password = e,
            Message::Submit => self.start_authenticate(),
            Message::Focus => return iced_widget::text_input::focus(self.input_id.clone()),
            Message::Reset => self.password.clear(),
            Message::CapsLock(caps) => self.caps_lock = caps,
            Message::ToggleShow => self.hide_input = !self.hide_input,
        }
        iced_runtime::Command::none()
    }
}

struct DummyClipboard {}

impl Clipboard for DummyClipboard {
    fn read(&self, kind: iced_wgpu::core::clipboard::Kind) -> Option<String> {
        log::debug!("dummy clipboard-read: {kind:?}");
        None
    }
    fn write(&mut self, kind: iced_wgpu::core::clipboard::Kind, contents: String) {
        log::debug!("dummy clipboard-write: {kind:?} {contents:?}");
    }
}

struct LastKeyTracker {
    pub keypress: keyboard::Event,
    time: SystemTime,
}

impl LastKeyTracker {
    fn new(keypress: keyboard::Event) -> Self {
        let time = SystemTime::now();
        Self { keypress, time }
    }
    #[inline]
    fn should_repeat(&self) -> bool {
        let now = SystemTime::now();
        now.duration_since(self.time).unwrap() >= HOLD_KEY_TIMEOUT
    }
}

/// Iced User Interface State Management and Operation
pub struct IcedState {
    format: wgpu::TextureFormat,
    renderer: Renderer,
    debug: Debug,
    state: Option<State<UI>>,
    viewport: Option<Viewport>,
    cursor: mouse::Cursor,
    clipboard: DummyClipboard,
    last_key: Option<LastKeyTracker>,
}

impl IcedState {
    pub fn new(
        _adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        log::debug!("iced - building initial ui state");
        let debug = Debug::default();
        let engine = Backend::new(&device, &queue, Settings::default(), format);
        let renderer = Renderer::new(engine, Font::default(), Pixels::from(32));
        Self {
            format,
            debug,
            renderer,
            viewport: None,
            state: None,
            cursor: mouse::Cursor::Available(Point::new(0.0, 0.0)),
            clipboard: DummyClipboard {},
            last_key: None,
        }
    }

    /// Configure State for Given Viewport Size
    pub fn configure(&mut self, width: u32, height: u32) {
        log::debug!("iced - configuing viewports from surface ({width}/{height})");
        let ui = UI::new();
        let bounds = Size::new(width, height);
        let viewport = Viewport::with_physical_size(bounds, 1.0);
        let size = viewport.logical_size();
        self.viewport = Some(viewport);
        self.state = Some(State::new(ui, size, &mut self.renderer, &mut self.debug));
    }

    /// Supply Keyboard Events to UI
    pub fn key_event(&mut self, event: keyboard::Event) {
        let state = self.state.as_mut().expect("ui state not configured yet");
        match &event {
            keyboard::Event::KeyPressed { key, .. } => match key.clone() {
                TAB => state.queue_message(Message::Focus),
                ESCAPE => state.queue_message(Message::Reset),
                CAPS_LOCK => state.queue_message(Message::CapsLock(true)),
                _ => self.last_key = Some(LastKeyTracker::new(event.to_owned())),
            },
            keyboard::Event::KeyReleased { key, .. } => {
                if key == &CAPS_LOCK {
                    state.queue_message(Message::CapsLock(false));
                }
                self.last_key = None;
            }
            _ => {}
        }
        state.queue_event(Event::Keyboard(event));
    }

    /// Supply Mouse Events to UI
    pub fn mouse_event(&mut self, event: mouse::Event) {
        let state = self.state.as_mut().expect("ui state not configured yet");
        state.queue_event(Event::Mouse(event));
        match event {
            mouse::Event::CursorMoved { position } => {
                self.cursor = mouse::Cursor::Available(position);
            }
            _ => {}
        }
    }

    /// Check if UI State if Authenticated
    #[inline]
    pub fn is_authenticated(&self) -> bool {
        self.state
            .as_ref()
            .expect("ui state not configured yet")
            .program()
            .is_authenticated()
    }

    /// Render UI Frame using WGPU
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        // update rendering with contents
        let state = self.state.as_mut().unwrap();
        let viewport = self.viewport.as_ref().unwrap();
        let bounds = viewport.logical_size();
        // spam focus on password field
        state.queue_message(Message::Focus);
        // handle backspace repitition
        if let Some(last_key) = self.last_key.as_ref() {
            if last_key.should_repeat() {
                state.queue_event(Event::Keyboard(last_key.keypress.to_owned()));
            }
        }
        // update iced-runtime program state and render
        let (_, command) = state.update(
            bounds,
            self.cursor,
            &mut self.renderer,
            &Theme::Dark,
            &renderer::Style {
                text_color: Color::WHITE,
            },
            &mut self.clipboard,
            &mut self.debug,
        );
        // collect operable events from commands
        let mut operations = Vec::new();
        if let Some(actions) = command.map(|c| c.actions()) {
            for action in actions {
                match action {
                    Action::Widget(op) => operations.push(op),
                    action => {
                        log::debug!("ui ignoring action {action:?}");
                    }
                }
            }
        }
        // complete operations (if any)
        state.operate(
            &mut self.renderer,
            operations.into_iter(),
            bounds,
            &mut self.debug,
        );
        // complete rendering
        self.renderer.with_primitives(|backend, primitive| {
            backend.present(
                &device,
                &queue,
                encoder,
                None,
                self.format,
                &view,
                primitive,
                &viewport,
                &self.debug.overlay(),
            )
        });
        // self.engine.submit(&queue, encoder);
    }
}

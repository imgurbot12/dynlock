//! Iced UI Implementation

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use iced_runtime::command::Action;
use iced_runtime::core::keyboard;
use iced_runtime::{program::State, Debug, Font};
use iced_wgpu::core::Point;
use iced_wgpu::Settings;
use iced_wgpu::{
    core::{mouse, renderer, Clipboard, Color, Event, Length, Pixels, Size},
    graphics::Viewport,
    wgpu, Backend, Renderer,
};
use iced_widget::{container, Column, Theme};

// keyboard handling utilities
const TAB: keyboard::Key = keyboard::Key::Named(keyboard::key::Named::Tab);
const HOLD_KEY_TIMEOUT: Duration = Duration::from_millis(200);

/// Lockscreen UI Implementation
pub struct UI {
    input_id: iced_widget::text_input::Id,
    username: String,
    realname: String,
    password: String,
    hide_input: bool,
    auth_thread: Option<std::thread::JoinHandle<()>>,
    authenticated: Arc<Mutex<bool>>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Typing(String),
    Submit,
    ToggleHide,
    Focus,
}

impl UI {
    pub fn new() -> Self {
        let input_id = iced_widget::text_input::Id::unique();
        Self {
            input_id,
            username: whoami::username(),
            realname: whoami::realname(),
            password: "".to_owned(),
            hide_input: true,
            auth_thread: None,
            authenticated: Arc::new(Mutex::new(false)),
        }
    }
    #[inline]
    fn auth_running(&self) -> bool {
        self.auth_thread
            .as_ref()
            .map(|t| !t.is_finished())
            .unwrap_or(false)
    }
    fn start_authenticate(&mut self) {
        // skip authenticating if already in progress
        if self.auth_running() {
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
        let mut password: iced_widget::text_input::TextInput<
            '_,
            Self::Message,
            Self::Theme,
            Self::Renderer,
        > = iced_widget::text_input("password", self.password.as_str())
            .id(self.input_id.clone())
            .secure(self.hide_input)
            .width(Length::Fixed(300.0));
        if !self.auth_running() {
            password = password
                .on_input(Message::Typing)
                .on_submit(Message::Submit);
        }
        // construct menu
        let message = iced_widget::text(self.realname.as_str());
        let menu = Column::new()
            .push(message)
            .push(password)
            .align_items(iced_wgpu::core::Alignment::Center);
        container(menu)
            .padding(10)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
    fn update(&mut self, message: Self::Message) -> iced_runtime::Command<Self::Message> {
        match message {
            Message::Typing(e) => {
                self.password = e;
                iced_runtime::Command::none()
            }
            Message::Submit => {
                self.start_authenticate();
                iced_runtime::Command::none()
            }
            Message::ToggleHide => {
                self.hide_input = !self.hide_input;
                iced_runtime::Command::none()
            }
            Message::Focus => {
                println!("focusing!");
                iced_widget::text_input::focus(self.input_id.clone())
                // iced_widget::text_input::move_cursor_to_end(self.input_id.clone())
            }
        }
    }
}

struct DummyClipboard {}

impl Clipboard for DummyClipboard {
    fn read(&self, kind: iced_wgpu::core::clipboard::Kind) -> Option<String> {
        println!("attempting to read from clipboard!");
        None
    }
    fn write(&mut self, kind: iced_wgpu::core::clipboard::Kind, contents: String) {
        println!("write to clipboard: {contents:?}");
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

    pub fn configure(&mut self, width: u32, height: u32) {
        let ui = UI::new();
        let bounds = Size::new(width, height);
        let viewport = Viewport::with_physical_size(bounds, 1.0);
        let size = viewport.logical_size();
        self.viewport = Some(viewport);
        self.state = Some(State::new(ui, size, &mut self.renderer, &mut self.debug));
    }

    pub fn key_event(&mut self, event: keyboard::Event) {
        let state = self.state.as_mut().expect("ui state not configured yet");
        match &event {
            keyboard::Event::KeyPressed { key, .. } if key == &TAB => {
                state.queue_message(Message::Focus);
            }
            keyboard::Event::KeyPressed { .. } => {
                self.last_key = Some(LastKeyTracker::new(event.to_owned()));
            }
            keyboard::Event::KeyReleased { .. } => {
                self.last_key = None;
            }
            _ => {}
        }
        state.queue_event(Event::Keyboard(event));
    }

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

    #[inline]
    pub fn is_authenticated(&self) -> bool {
        self.state
            .as_ref()
            .expect("ui state not configured yet")
            .program()
            .is_authenticated()
    }

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

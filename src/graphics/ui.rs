//! Iced UI Implementation

use std::time::{Duration, SystemTime};

use iced_runtime::core::keyboard;
use iced_runtime::{program::State, Debug, Font};
use iced_wgpu::core::Point;
use iced_wgpu::{
    core::{mouse, renderer, Clipboard, Color, Event, Length, Pixels, Size},
    graphics::Viewport,
    wgpu, Engine, Renderer,
};
use iced_widget::{container, Column, Theme};

// reference to backspace key definition
const BACKSPACE: keyboard::Key = keyboard::Key::Named(keyboard::key::Named::Backspace);
const BACKSPACE_TIMEOUT: Duration = Duration::from_millis(200);
const BACKSPACE_EVENT: keyboard::Event = keyboard::Event::KeyPressed {
    key: BACKSPACE,
    location: keyboard::Location::Standard,
    modifiers: keyboard::Modifiers::empty(),
    text: None,
};

/// Lockscreen UI Implementation
pub struct UI {
    input_id: iced_widget::text_input::Id,
    realname: String,
    input_value: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Typing(String),
    Submit,
}

impl UI {
    pub fn new() -> Self {
        let input_id = iced_widget::text_input::Id::unique();
        Self {
            input_id,
            realname: whoami::realname(),
            input_value: "".to_owned(),
        }
    }
}

impl iced_runtime::Program for UI {
    type Theme = iced_wgpu::core::Theme;
    type Message = Message;
    type Renderer = iced_wgpu::Renderer;

    fn view(&self) -> iced_runtime::core::Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        let message = iced_widget::text(self.realname.as_str());
        let password = iced_widget::text_input("password", self.input_value.as_str())
            .id(self.input_id.clone())
            .on_input(Message::Typing)
            .on_submit(Message::Submit)
            .width(Length::Fixed(300.0));
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
                self.input_value = e;
            }
            Message::Submit => {
                println!("submit: {:?}", self.input_value);
            }
        };
        iced_runtime::Command::none()
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

pub struct IcedState {
    format: wgpu::TextureFormat,
    engine: Engine,
    renderer: Renderer,
    debug: Debug,
    state: Option<State<UI>>,
    viewport: Option<Viewport>,
    cursor: mouse::Cursor,
    clipboard: DummyClipboard,
    // hack for holding down backspace
    backspace: Option<SystemTime>,
}

impl IcedState {
    pub fn new(
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
    ) -> Self {
        let debug = Debug::default();
        let engine = Engine::new(&adapter, &device, &queue, format, None);
        let renderer = Renderer::new(&device, &engine, Font::default(), Pixels::from(32));
        Self {
            format,
            debug,
            engine,
            renderer,
            viewport: None,
            state: None,
            cursor: mouse::Cursor::Available(Point::new(0.0, 0.0)),
            clipboard: DummyClipboard {},
            backspace: None,
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
            keyboard::Event::KeyPressed { key, .. } if key == &BACKSPACE => {
                self.backspace = Some(SystemTime::now());
            }
            keyboard::Event::KeyReleased { key, .. } if key == &BACKSPACE => {
                self.backspace = None;
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

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        mut encoder: wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        // update rendering with contents
        let state = self.state.as_mut().unwrap();
        let viewport = self.viewport.as_ref().unwrap();
        let bounds = viewport.logical_size();
        // handle backspace repitition
        let now = SystemTime::now();
        if let Some(backspace) = self.backspace {
            if now.duration_since(backspace).unwrap() >= BACKSPACE_TIMEOUT {
                state.queue_event(Event::Keyboard(BACKSPACE_EVENT));
            }
        }
        // update iced-runtime program state and render
        let _ = state.update(
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
        // complete rendering
        self.renderer.present(
            &mut self.engine,
            &device,
            &queue,
            &mut encoder,
            None,
            self.format,
            &view,
            &viewport,
            &self.debug.overlay(),
        );
        self.engine.submit(&queue, encoder);
    }
}

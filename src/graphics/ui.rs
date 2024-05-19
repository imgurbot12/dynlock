//! Iced UI Implementation

use iced_runtime::core::keyboard;
use iced_runtime::{program::State, Debug, Font};
use iced_wgpu::core::Point;
use iced_wgpu::{
    core::{mouse, renderer, Clipboard, Color, Event, Length, Pixels, Size},
    graphics::Viewport,
    wgpu, Engine, Renderer,
};
use iced_widget::{container, Column, Theme};

/// Lockscreen UI Implementation
pub struct UI {
    input_id: iced_widget::text_input::Id,
    input_value: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    EventOccured(Event),
}

impl UI {
    pub fn new() -> Self {
        let input_id = iced_widget::text_input::Id::unique();
        Self {
            input_id,
            input_value: "".to_owned(),
        }
    }
}

impl iced_runtime::Program for UI {
    type Theme = iced_wgpu::core::Theme;
    type Message = Message;
    type Renderer = iced_wgpu::Renderer;

    fn view(&self) -> iced_runtime::core::Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        let message = iced_widget::text("Change Da World. My Final Message. Goodbye!");
        let password = iced_widget::text_input("password", self.input_value.as_str())
            .id(self.input_id.clone())
            .width(Length::Fixed(200.0));
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
            Message::EventOccured(e) => {
                println!("event! {e:?}");
                iced_runtime::Command::none()
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

pub struct IcedState {
    format: wgpu::TextureFormat,
    engine: Engine,
    renderer: Renderer,
    viewport: Option<Viewport>,
    debug: Debug,
    state: Option<State<UI>>,
    cursor: mouse::Cursor,
    clipboard: DummyClipboard,
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
        let state = self.state.as_mut().unwrap();
        // state.queue_message(Message::Key("".to_owned()));
        state.queue_event(Event::Keyboard(event));
    }

    pub fn mouse_event(&mut self, event: mouse::Event) {
        let state = self.state.as_mut().unwrap();
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
        // update iced-runtime program state and render
        let (events, _) = state.update(
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
        if !events.is_empty() {
            println!("ignored events {events:?}");
        }
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

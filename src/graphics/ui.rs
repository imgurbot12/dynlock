//! Iced UI Implementation

use iced_runtime::{program::State, Debug, Font};
use iced_wgpu::{
    core::{mouse, renderer, Clipboard, Color, Pixels, Size},
    graphics::Viewport,
    wgpu, Engine, Renderer,
};
use iced_widget::Theme;

/// Lockscreen UI Implementation
pub struct UI {}

#[derive(Debug, Clone, Copy)]
pub enum Message {}

impl iced_runtime::Program for UI {
    type Theme = iced_wgpu::core::Theme;
    type Message = Message;
    type Renderer = iced_wgpu::Renderer;

    fn view(&self) -> iced_runtime::core::Element<'_, Self::Message, Self::Theme, Self::Renderer> {
        iced_widget::text("Hello World!").into()
    }
    fn update(&mut self, message: Self::Message) -> iced_runtime::Command<Self::Message> {
        match message {};
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
        let renderer = Renderer::new(&device, &engine, Font::default(), Pixels::from(16));
        Self {
            format,
            debug,
            engine,
            renderer,
            viewport: None,
            state: None,
            clipboard: DummyClipboard {},
        }
    }

    pub fn configure(&mut self, width: u32, height: u32) {
        let ui = UI {};
        let bounds = Size::new(width, height);
        let bounds2 = Size::new(width as f32, height as f32);
        self.viewport = Some(Viewport::with_physical_size(bounds, 1.0));
        self.state = Some(State::new(ui, bounds2, &mut self.renderer, &mut self.debug));
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
        state.update(
            bounds,
            mouse::Cursor::Unavailable,
            &mut self.renderer,
            &Theme::Dark,
            &renderer::Style {
                text_color: Color::BLACK,
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

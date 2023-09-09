use glam::UVec2;
use quirky::widget::Widget;
use quirky::QuirkyApp;
use std::sync::Arc;
use wgpu::{Backends, Instance, InstanceDescriptor, Surface, SurfaceCapabilities, TextureFormat};
use winit::dpi::PhysicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};

pub struct QuirkyWinitApp {
    quirky_app: Arc<QuirkyApp>,
    event_loop: Option<EventLoop<()>>,
    surface: Surface,
    surface_format: TextureFormat,
    surface_capabilities: SurfaceCapabilities,
    window: Window,
}

impl QuirkyWinitApp {
    pub async fn new(widget: Arc<dyn Widget>) -> anyhow::Result<(QuirkyWinitApp, Arc<QuirkyApp>)> {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new().build(&event_loop).unwrap();

        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe { instance.create_surface(&window) }.unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);

        let quirky_app = Arc::new(QuirkyApp::new(device, queue, surface_format, widget));

        let quirky_winit_app = Self {
            quirky_app: quirky_app.clone(),
            event_loop: Some(event_loop),
            surface,
            surface_format,
            surface_capabilities,
            window,
        };

        Ok((quirky_winit_app, quirky_app))
    }

    pub fn get_trigger_draw_callback(&self) -> impl Fn() {
        let elproxy = self.event_loop.as_ref().unwrap().create_proxy();

        move || {
            elproxy
                .send_event(())
                .expect("failed to send eventloop message on new drawables")
        }
    }
    pub fn run(mut self) {
        let event_loop = self
            .event_loop
            .take()
            .expect("invalid QuirkiWinitApp: missing event loop");

        event_loop.run(move |event, _target, control_flow: &mut ControlFlow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::UserEvent(()) => {
                    self.window.request_redraw();
                }
                Event::RedrawRequested(_window_id) => {
                    self.quirky_app.draw(&self.surface);
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(new_size) => self.resize_window(new_size),
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    _ => {}
                },
                _ => {}
            }
        });
    }

    fn resize_window(&self, new_size: PhysicalSize<u32>) {
        if new_size.height > 0 && new_size.width > 0 {
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: self.surface_format,
                width: new_size.width,
                height: new_size.height,
                present_mode: self.surface_capabilities.present_modes[0],
                alpha_mode: self.surface_capabilities.alpha_modes[0],
                view_formats: vec![],
            };

            self.surface.configure(&self.quirky_app.device, &config);
            self.quirky_app
                .viewport_size
                .set(UVec2::new(new_size.width, new_size.height));
        }
    }
}

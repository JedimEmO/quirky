use glam::UVec2;
use glyphon::{FontSystem, SwashCache};
use quirky::widget::Widget;
use quirky::{MouseButton, MouseEvent, QuirkyApp, WidgetEvent};
use std::sync::Arc;
use uuid::Uuid;
use wgpu::{
    Backends, Instance, InstanceDescriptor, PresentMode, Surface, SurfaceCapabilities,
    TextureFormat,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Event, WindowEvent};
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
    pub async fn new(
        widget: Arc<dyn Widget>,
        font_system: FontSystem,
        font_cache: SwashCache,
    ) -> anyhow::Result<(QuirkyWinitApp, Arc<QuirkyApp>)> {
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

        let quirky_app = Arc::new(QuirkyApp::new(
            device,
            queue,
            surface_format,
            widget,
            font_system,
            font_cache,
        ));

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

        let mut mouse_pos = PhysicalPosition::default();
        let mut prev_hovered: Option<Uuid> = None;
        let mut target_widget: Option<Uuid> = None;
        let mut prev_drag_pos: Option<UVec2> = None;
        let mut drag_button: Option<MouseButton> = None;

        event_loop.run(move |event, _target, control_flow: &mut ControlFlow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::UserEvent(()) => {
                    self.window.request_redraw();
                }
                Event::RedrawRequested(_window_id) => {
                    let _ = self.quirky_app.draw(&self.surface);
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::Resized(new_size) => self.resize_window(new_size),
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::MouseInput { state, button, .. } => {
                        if state == ElementState::Pressed {
                            if let Some(p) = self
                                .quirky_app
                                .get_widgets_at(UVec2::new(mouse_pos.x as u32, mouse_pos.y as u32))
                            {
                                prev_drag_pos =
                                    Some(UVec2::new(mouse_pos.x as u32, mouse_pos.y as u32));

                                let new_target_widget = *p.first().unwrap();

                                target_widget = Some(new_target_widget);

                                if button == winit::event::MouseButton::Left {
                                    drag_button = Some(MouseButton::Left);
                                } else if button == winit::event::MouseButton::Right {
                                    drag_button = Some(MouseButton::Right);
                                } else if button == winit::event::MouseButton::Middle {
                                    drag_button = Some(MouseButton::Middle);
                                }

                                if let Some(b) = &drag_button {
                                    self.quirky_app.dispatch_event_to_widget(
                                        new_target_widget,
                                        WidgetEvent::MouseEvent {
                                            event: MouseEvent::ButtonDown { button: *b },
                                        },
                                    );
                                }
                            }
                        }

                        if state == ElementState::Released {
                            if let Some(prev_target_widget) = target_widget {
                                self.quirky_app.dispatch_event_to_widget(
                                    prev_target_widget,
                                    WidgetEvent::MouseEvent {
                                        event: MouseEvent::ButtonUp {
                                            button: drag_button.unwrap(),
                                        },
                                    },
                                );
                            }

                            drag_button = None;
                            target_widget = None;
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        mouse_pos = position;
                        let pos = UVec2::new(mouse_pos.x as u32, mouse_pos.y as u32);

                        if target_widget.is_some() && prev_drag_pos.is_some() {
                            self.quirky_app.dispatch_event_to_widget(
                                target_widget.unwrap(),
                                WidgetEvent::MouseEvent {
                                    event: MouseEvent::Drag {
                                        from: prev_drag_pos.unwrap(),
                                        to: pos,
                                        button: drag_button.unwrap(),
                                    },
                                },
                            );
                        }

                        if let Some(widgets) = self.quirky_app.get_widgets_at(pos) {
                            let new_target_widget = *widgets.first().unwrap();

                            if let Some(p) = prev_hovered {
                                if p != new_target_widget {
                                    self.quirky_app.dispatch_event_to_widget(
                                        p,
                                        WidgetEvent::MouseEvent {
                                            event: MouseEvent::Leave {},
                                        },
                                    );
                                }
                            }

                            prev_hovered = Some(new_target_widget);

                            self.quirky_app.dispatch_event_to_widget(
                                new_target_widget,
                                WidgetEvent::MouseEvent {
                                    event: MouseEvent::Move { pos },
                                },
                            );
                        }

                        prev_drag_pos = Some(pos);
                    }
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
                present_mode: PresentMode::AutoVsync,
                alpha_mode: self.surface_capabilities.alpha_modes[0],
                view_formats: vec![],
            };

            self.surface
                .configure(&self.quirky_app.context.device, &config);
            self.quirky_app
                .viewport_size
                .set(UVec2::new(new_size.width, new_size.height));
        }
    }
}

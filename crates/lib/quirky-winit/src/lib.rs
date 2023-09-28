use futures_signals::signal::{Mutable, SignalExt};
use glam::UVec2;
use glyphon::{FontSystem, SwashCache};
use quirky::widget::Widget;
use quirky::{
    clone, KeyCode, KeyboardEvent, KeyboardModifier, MouseButton, MouseEvent, QuirkyApp,
    WidgetEvent,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;
use wgpu::{
    Backends, Instance, InstanceDescriptor, PresentMode, Surface, SurfaceCapabilities,
    TextureFormat,
};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Event, VirtualKeyCode, WindowEvent};
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

    pub fn run_event_loop(mut self) {
        let event_loop = self
            .event_loop
            .take()
            .expect("invalid QuirkiWinitApp: missing event loop");

        let mut mouse_pos = PhysicalPosition::default();
        let prev_hovered: Mutable<Option<Uuid>> = Default::default();
        let target_widget: Mutable<Option<Uuid>> = Default::default();
        let prev_drag_pos: Mutable<Option<UVec2>> = Default::default();
        let drag_button: Mutable<Option<MouseButton>> = Default::default();
        let mut modifiers = KeyboardModifier::default();
        let current_mouse_pos: Mutable<UVec2> = Default::default();

        let quirky_app = self.quirky_app.clone();
        tokio::spawn(clone!(
            prev_hovered,
            clone!(
                drag_button,
                clone!(
                    target_widget,
                    clone!(
                        prev_drag_pos,
                        clone!(current_mouse_pos, async move {
                            current_mouse_pos
                                .signal()
                                .throttle(|| sleep(Duration::from_millis(5)))
                                .for_each(|pos| {
                                    if target_widget.get().is_some()
                                        && prev_drag_pos.get().is_some()
                                    {
                                        quirky_app.dispatch_event_to_widget(
                                            target_widget.get().unwrap(),
                                            WidgetEvent::MouseEvent {
                                                event: MouseEvent::Drag {
                                                    from: prev_drag_pos.get().unwrap(),
                                                    to: pos,
                                                    button: drag_button.get().unwrap(),
                                                },
                                            },
                                        );
                                    }

                                    if let Some(widgets) = quirky_app.get_widgets_at(pos) {
                                        let new_target_widget = *widgets.first().unwrap();

                                        if let Some(p) = prev_hovered.get() {
                                            if p != new_target_widget {
                                                quirky_app.dispatch_event_to_widget(
                                                    p,
                                                    WidgetEvent::MouseEvent {
                                                        event: MouseEvent::Leave {},
                                                    },
                                                );
                                            }
                                        }

                                        prev_hovered.set(Some(new_target_widget));

                                        quirky_app.dispatch_event_to_widget(
                                            new_target_widget,
                                            WidgetEvent::MouseEvent {
                                                event: MouseEvent::Move { pos },
                                            },
                                        );
                                    }

                                    prev_drag_pos.set(Some(pos));

                                    async move {}
                                })
                                .await;
                        })
                    )
                )
            )
        ));

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
                    WindowEvent::ModifiersChanged(state) => {
                        modifiers.alt = state.alt();
                        modifiers.shift = state.shift();
                        modifiers.ctrl = state.ctrl();
                    }
                    WindowEvent::KeyboardInput { input, .. } => {
                        if input.state == ElementState::Pressed {
                            let target = prev_hovered.get().or(Some(Uuid::nil())).unwrap();

                            let code = input
                                .virtual_keycode
                                .map(|code| winit_keycode_to_quirky(code))
                                .or(Some(KeyCode::Unknown))
                                .unwrap();

                            self.quirky_app.dispatch_event_to_widget(
                                target,
                                WidgetEvent::KeyboardEvent {
                                    event: KeyboardEvent::KeyPressed {
                                        key_code: code,
                                        modifier: modifiers.clone(),
                                    },
                                },
                            )
                        }
                    }
                    WindowEvent::Resized(new_size) => self.resize_window(new_size),
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::MouseInput { state, button, .. } => {
                        let mouse_pos = current_mouse_pos.get();

                        if state == ElementState::Pressed {
                            if let Some(p) = self
                                .quirky_app
                                .get_widgets_at(UVec2::new(mouse_pos.x, mouse_pos.y))
                            {
                                prev_drag_pos.set(Some(UVec2::new(mouse_pos.x, mouse_pos.y)));

                                let new_target_widget = *p.first().unwrap();

                                target_widget.set(Some(new_target_widget));

                                if button == winit::event::MouseButton::Left {
                                    drag_button.set(Some(MouseButton::Left));
                                } else if button == winit::event::MouseButton::Right {
                                    drag_button.set(Some(MouseButton::Right));
                                } else if button == winit::event::MouseButton::Middle {
                                    drag_button.set(Some(MouseButton::Middle));
                                }

                                if let Some(b) = &drag_button.get() {
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
                            if let Some(prev_target_widget) = target_widget.get() {
                                self.quirky_app.dispatch_event_to_widget(
                                    prev_target_widget,
                                    WidgetEvent::MouseEvent {
                                        event: MouseEvent::ButtonUp {
                                            button: drag_button.get().unwrap(),
                                        },
                                    },
                                );
                            }

                            drag_button.set(None);
                            target_widget.set(None);
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        mouse_pos = position;
                        let pos = UVec2::new(mouse_pos.x as u32, mouse_pos.y as u32);
                        current_mouse_pos.set(pos);
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
fn winit_keycode_to_quirky(keycode: VirtualKeyCode) -> KeyCode {
    match keycode {
        VirtualKeyCode::Key1 => KeyCode::Key1,
        VirtualKeyCode::Key2 => KeyCode::Key2,
        VirtualKeyCode::Key3 => KeyCode::Key3,
        VirtualKeyCode::Key4 => KeyCode::Key4,
        VirtualKeyCode::Key5 => KeyCode::Key5,
        VirtualKeyCode::Key6 => KeyCode::Key6,
        VirtualKeyCode::Key7 => KeyCode::Key7,
        VirtualKeyCode::Key8 => KeyCode::Key8,
        VirtualKeyCode::Key9 => KeyCode::Key9,
        VirtualKeyCode::Key0 => KeyCode::Key0,
        VirtualKeyCode::A => KeyCode::A,
        VirtualKeyCode::B => KeyCode::B,
        VirtualKeyCode::C => KeyCode::C,
        VirtualKeyCode::D => KeyCode::D,
        VirtualKeyCode::E => KeyCode::E,
        VirtualKeyCode::F => KeyCode::F,
        VirtualKeyCode::G => KeyCode::G,
        VirtualKeyCode::H => KeyCode::H,
        VirtualKeyCode::I => KeyCode::I,
        VirtualKeyCode::J => KeyCode::J,
        VirtualKeyCode::K => KeyCode::K,
        VirtualKeyCode::L => KeyCode::L,
        VirtualKeyCode::M => KeyCode::M,
        VirtualKeyCode::N => KeyCode::N,
        VirtualKeyCode::O => KeyCode::O,
        VirtualKeyCode::P => KeyCode::P,
        VirtualKeyCode::Q => KeyCode::Q,
        VirtualKeyCode::R => KeyCode::R,
        VirtualKeyCode::S => KeyCode::S,
        VirtualKeyCode::T => KeyCode::T,
        VirtualKeyCode::U => KeyCode::U,
        VirtualKeyCode::V => KeyCode::V,
        VirtualKeyCode::W => KeyCode::W,
        VirtualKeyCode::X => KeyCode::X,
        VirtualKeyCode::Y => KeyCode::Y,
        VirtualKeyCode::Z => KeyCode::Z,
        VirtualKeyCode::Escape => KeyCode::Escape,
        VirtualKeyCode::Back => KeyCode::Backspace,
        VirtualKeyCode::Return => KeyCode::Return,
        VirtualKeyCode::Space => KeyCode::Space,
        VirtualKeyCode::Comma => KeyCode::Comma,
        VirtualKeyCode::Grave => KeyCode::Grave,
        VirtualKeyCode::Period => KeyCode::Period,
        VirtualKeyCode::Caret => KeyCode::Caret,
        VirtualKeyCode::Asterisk => KeyCode::Asterisk,
        VirtualKeyCode::Backslash => KeyCode::Backslash,
        VirtualKeyCode::Semicolon => KeyCode::Semicolon,
        VirtualKeyCode::At => KeyCode::At,
        _ => KeyCode::Unknown,
    }
}

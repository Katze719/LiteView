use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};
use wgpu::{
    Backends, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PresentMode, Queue, Surface, SurfaceConfiguration, Texture, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, TextureView,
};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowAttributes, WindowId};

#[cfg(target_os = "linux")]
use winit::platform::wayland::EventLoopBuilderExtWayland;
#[cfg(target_os = "linux")]
use winit::platform::x11::EventLoopBuilderExtX11;

const FPS_UPDATE_INTERVAL: Duration = Duration::from_secs(1);

pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u32>,
}

pub struct PreviewState {
    pub frame: Mutex<Option<FrameData>>,
    pub running: std::sync::atomic::AtomicBool,
    pub frame_available: Condvar,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            frame: Mutex::new(None),
            running: std::sync::atomic::AtomicBool::new(true),
            frame_available: Condvar::new(),
        }
    }
}

struct WgpuContext {
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    config: SurfaceConfiguration,
    texture: Option<Texture>,
    texture_view: Option<TextureView>,
    texture_size: (u32, u32),
    render_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
}

impl WgpuContext {
    async fn new(window: Arc<Window>) -> Self {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &DeviceDescriptor {
                    label: Some("Preview Device"),
                    required_features: Features::empty(),
                    required_limits: Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let size = window.inner_size();
        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Texture Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            texture: None,
            texture_view: None,
            texture_size: (0, 0),
            render_pipeline,
            bind_group_layout,
            sampler,
            bind_group: None,
        }
    }

    fn resize(&mut self, new_size: (u32, u32)) {
        if new_size.0 > 0 && new_size.1 > 0 {
            self.config.width = new_size.0;
            self.config.height = new_size.1;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update_texture(&mut self, width: u32, height: u32, data: &[u32]) {
        if self.texture.is_none() || self.texture_size != (width, height) {
            self.texture = Some(self.device.create_texture(&TextureDescriptor {
                label: Some("Frame Texture"),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            }));
            let view = self
                .texture
                .as_ref()
                .unwrap()
                .create_view(&wgpu::TextureViewDescriptor::default());

            self.bind_group = Some(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Texture Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }));

            self.texture_view = Some(view);
            self.texture_size = (width, height);
        }

        let rgba_data: Vec<u8> = data
            .iter()
            .flat_map(|&pixel| {
                let r = ((pixel >> 16) & 0xFF) as u8;
                let g = ((pixel >> 8) & 0xFF) as u8;
                let b = (pixel & 0xFF) as u8;
                [r, g, b, 255]
            })
            .collect();

        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: self.texture.as_ref().unwrap(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        if self.bind_group.is_none() {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

struct PreviewApp {
    state: Arc<PreviewState>,
    window: Option<Arc<Window>>,
    wgpu_context: Option<WgpuContext>,
    frame_count: u32,
    fps_last: Instant,
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        // Window is only created when the first frame arrives
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.state.running.store(false, Ordering::Relaxed);
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(ref mut ctx) = self.wgpu_context {
                    ctx.resize((size.width, size.height));
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(ref window), Some(ref mut ctx)) =
                    (&self.window, &mut self.wgpu_context)
                {
                    let mut has_new_frame = false;

                    if let Ok(mut guard) = self.state.frame.try_lock() {
                        if let Some(frame_data) = guard.take() {
                            ctx.update_texture(
                                frame_data.width,
                                frame_data.height,
                                &frame_data.buffer,
                            );
                            has_new_frame = true;
                        }
                    }

                    if let Ok(()) = ctx.render() {
                        if has_new_frame {
                            self.frame_count += 1;
                            if self.frame_count == 1 {
                                self.fps_last = Instant::now();
                            }
                        }
                        let elapsed = self.fps_last.elapsed();
                        if elapsed >= FPS_UPDATE_INTERVAL && self.frame_count > 0 {
                            let fps = self.frame_count as f64 / elapsed.as_secs_f64();
                            window.set_title(&format!("LiteView Preview — {:.0} fps", fps));
                            self.frame_count = 0;
                            self.fps_last = Instant::now();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.state.running.load(Ordering::Relaxed) {
            if let Some(ref window) = self.window {
                window.set_visible(false);
            }
            self.window = None;
            self.wgpu_context = None;
            event_loop.exit();
            return;
        }

        let wait_duration = Duration::from_millis(16);
        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + wait_duration));

        // Create window when first frame is available
        if self.window.is_none() {
            if let Ok(mut guard) = self.state.frame.try_lock() {
                if let Some(frame_data) = guard.take() {
                    let attrs = WindowAttributes::default()
                        .with_title("LiteView Preview")
                        .with_inner_size(LogicalSize::new(frame_data.width, frame_data.height))
                        .with_resizable(true)
                        .with_decorations(false);

                    if let Ok(window) = event_loop.create_window(attrs) {
                        let window = Arc::new(window);
                        let mut ctx = pollster::block_on(WgpuContext::new(window.clone()));
                        ctx.update_texture(frame_data.width, frame_data.height, &frame_data.buffer);
                        let _ = ctx.render();
                        self.wgpu_context = Some(ctx);
                        self.window = Some(window.clone());
                        self.frame_count = 1;
                        self.fps_last = Instant::now();
                        window.request_redraw();
                    }
                }
            }
        } else {
            // Request redraw only when a new frame is available
            if let Ok(guard) = self.state.frame.try_lock() {
                if guard.is_some() {
                    if let Some(ref window) = self.window {
                        window.request_redraw();
                    }
                }
            }
        }
    }
}

pub fn run_preview_window(state: Arc<PreviewState>) {
    let mut event_loop_builder = winit::event_loop::EventLoop::builder();

    #[cfg(target_os = "linux")]
    {
        let session_type = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
        if session_type == "wayland" {
            EventLoopBuilderExtWayland::with_any_thread(&mut event_loop_builder, true);
        } else {
            EventLoopBuilderExtX11::with_any_thread(&mut event_loop_builder, true);
        }
    }

    let event_loop = event_loop_builder.build().unwrap();

    let mut app = PreviewApp {
        state,
        window: None,
        wgpu_context: None,
        frame_count: 0,
        fps_last: Instant::now(),
    };

    let _ = event_loop.run_app(&mut app);
}

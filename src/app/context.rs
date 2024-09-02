use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

use crate::cpu::{DISPLAY_DATA_LEN, DISPLAY_HEIGHT, DISPLAY_WIDTH};

/// Number of bytes in the render buffer
const RENDER_BUF_SIZE: usize = DISPLAY_DATA_LEN * 4;
const TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

const UPSCALE_SHADER: &str = include_str!("../../shaders/upscale.wgsl");

async fn request_adapter_and_device<'a>(
    instance: &wgpu::Instance,
    surface: &wgpu::Surface<'a>,
) -> (wgpu::Adapter, wgpu::Device, wgpu::Queue) {
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::LowPower,
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults()
                .using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::MemoryUsage
        }, None)
        .await
        .unwrap();

    (adapter, device, queue)
}

/// Rendering context
pub struct Context<'win> {
    surface: wgpu::Surface<'win>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    pub buffer_data: [u8; RENDER_BUF_SIZE],
    render_texture: wgpu::Texture,

    upscale_pipeline: wgpu::RenderPipeline,
    upscale_bind_group: wgpu::BindGroup
}
impl<'win> Context<'win> {
    pub fn new(win: Arc<Window>) -> Self {
        let win_size = win.inner_size();

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(win).unwrap();

        // Request an adapter and a device and block the thread utill we receive them
        let (adapter, device, queue) = pollster::block_on(request_adapter_and_device(&instance, &surface));

        // Create render target texture
        let render_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render target texture"),
            size: wgpu::Extent3d {
                width: DISPLAY_WIDTH,
                height: DISPLAY_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[TEXTURE_FORMAT],
        });
        let render_view = render_texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Render target view"),
            format: Some(TEXTURE_FORMAT),
            ..Default::default()
        });
        let render_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Render target sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Create upscale bind group
        let upscale_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Upscale bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None
                }
            ],
        });
        let upscale_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Upscale bind group"),
            layout: &upscale_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_view)
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_sampler)
                },
            ],
        });

        // Create upscale render pipeline
        let upscale_pipeline = {
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Upscale shader module"),
                source: wgpu::ShaderSource::Wgsl(UPSCALE_SHADER.into())
            });
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Upscale pipeline layout"),
                bind_group_layouts: &[&upscale_bind_group_layout],
                push_constant_ranges: &[],
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Upscale render pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    compilation_options: Default::default(),
                    targets: &[Some(TEXTURE_FORMAT.into())]
                }),
                primitive: wgpu::PrimitiveState::default(),
                multisample: wgpu::MultisampleState::default(),
                depth_stencil: None,
                multiview: None,
                cache: None,
            })
        };

        // Create surface config
        let mut config = surface
            .get_default_config(&adapter, win_size.width, win_size.height)
            .unwrap();
        config.format = TEXTURE_FORMAT;
        surface.configure(&device, &config);

        Self {
            surface,
            device,
            queue,
            config,

            buffer_data: [0; RENDER_BUF_SIZE],
            render_texture,

            upscale_pipeline,
            upscale_bind_group,
        }
    }

    pub fn render(&mut self) {
        let frame = self.surface.get_current_texture().unwrap();
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("Upscale frame view"),
            ..Default::default()
        });
        let mut encoder = self.device.create_command_encoder(&Default::default());

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Upscale render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.upscale_pipeline);
            rpass.set_bind_group(0, &self.upscale_bind_group, &[]);
            rpass.draw(0..4, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
    }
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    /// Write the buffer to the texture
    pub fn write_buf(&mut self) {
        self.queue.write_texture(
            self.render_texture.as_image_copy(),
            &self.buffer_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(DISPLAY_WIDTH * 4),
                rows_per_image: Some(DISPLAY_HEIGHT),
            },
            wgpu::Extent3d {
                width: DISPLAY_WIDTH,
                height: DISPLAY_HEIGHT,
                depth_or_array_layers: 1,
            },
        )
    }
}

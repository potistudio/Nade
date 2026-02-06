use iced::advanced::graphics::Viewport;
use iced::widget::shader::{Pipeline, Primitive, Program};
use iced::{Rectangle, mouse};
use nade_core::FrameData;
use std::fmt::Debug;

// Need wgpu types
use wgpu;

#[derive(Debug, Clone)]
pub struct VideoView {
	frame: Option<FrameData>,
}

impl VideoView {
	pub fn new(frame: Option<FrameData>) -> Self {
		Self { frame }
	}
}

impl<Message> Program<Message> for VideoView {
	type State = ();
	type Primitive = VideoPrimitive;

	fn draw(
		&self,
		_state: &Self::State,
		_cursor: mouse::Cursor,
		_bounds: Rectangle,
	) -> Self::Primitive {
		VideoPrimitive {
			frame: self.frame.clone(),
		}
	}
}

#[derive(Debug)]
pub struct VideoPrimitive {
	frame: Option<FrameData>,
}

#[derive(Debug)]
pub struct VideoPipeline {
	texture: Option<wgpu::Texture>,
	bind_group: Option<wgpu::BindGroup>,
	pipeline: wgpu::RenderPipeline,
	layout: wgpu::BindGroupLayout,
	last_ptr: usize,
}

impl Pipeline for VideoPipeline {
	fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
		// 1. Create BindGroupLayout
		let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Video Bind Group Layout"),
			entries: &[
				// Texture
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
				// Sampler
				wgpu::BindGroupLayoutEntry {
					binding: 1,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
					count: None,
				},
			],
		});

		// 2. Create Shader Module
		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("Video Shader"),
			source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!(
				"../shaders/video_view.wgsl"
			))),
		});

		// 3. Create Pipeline Layout
		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Video Pipeline Layout"),
			bind_group_layouts: &[&layout],
			push_constant_ranges: &[],
		});

		// 4. Create Render Pipeline
		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("Video Render Pipeline"),
			layout: Some(&pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: Some("vs_main"),
				buffers: &[],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: Some("fs_main"),
				targets: &[Some(wgpu::ColorTargetState {
					format,
					blend: Some(wgpu::BlendState::REPLACE),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				compilation_options: wgpu::PipelineCompilationOptions::default(),
			}),
			primitive: wgpu::PrimitiveState::default(),
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			multiview: None,
			cache: None, // New field
		});

		Self {
			texture: None,
			bind_group: None,
			pipeline,
			layout,
			last_ptr: 0,
		}
	}
}

impl Primitive for VideoPrimitive {
	type Pipeline = VideoPipeline;

	fn prepare(
		&self,
		pipeline: &mut Self::Pipeline,
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		_bounds: &Rectangle,
		_storage: &Viewport,
	) {
		if let Some(frame) = &self.frame {
			let width = frame.width;
			let height = frame.height;

			// Recreate texture if needed
			let recreate = if let Some(tex) = &pipeline.texture {
				tex.width() != width || tex.height() != height
			} else {
				true
			};

			if recreate {
				let texture = device.create_texture(&wgpu::TextureDescriptor {
					label: Some("Video Frame Texture"),
					size: wgpu::Extent3d {
						width,
						height,
						depth_or_array_layers: 1,
					},
					mip_level_count: 1,
					sample_count: 1,
					dimension: wgpu::TextureDimension::D2,
					format: wgpu::TextureFormat::Rgba8Unorm,
					usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
					view_formats: &[],
				});

				let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
				let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
					mag_filter: wgpu::FilterMode::Linear,
					min_filter: wgpu::FilterMode::Linear,
					// Clamp to edge to avoid border artifacts
					address_mode_u: wgpu::AddressMode::ClampToEdge,
					address_mode_v: wgpu::AddressMode::ClampToEdge,
					..Default::default()
				});

				let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
					label: Some("Video Bind Group"),
					layout: &pipeline.layout,
					entries: &[
						wgpu::BindGroupEntry {
							binding: 0,
							resource: wgpu::BindingResource::TextureView(&view),
						},
						wgpu::BindGroupEntry {
							binding: 1,
							resource: wgpu::BindingResource::Sampler(&sampler),
						},
					],
				});

				pipeline.texture = Some(texture);
				pipeline.bind_group = Some(bind_group);
			}

			// Upload data
			// Optimization: Only upload if pixels buffer changed or texture was recreated
			let current_ptr = frame.pixels.as_ptr() as usize;
			if (recreate || current_ptr != pipeline.last_ptr)
				&& let Some(texture) = &pipeline.texture
			{
				queue.write_texture(
					wgpu::TexelCopyTextureInfo {
						texture,
						mip_level: 0,
						origin: wgpu::Origin3d::ZERO,
						aspect: wgpu::TextureAspect::All,
					},
					&frame.pixels,
					wgpu::TexelCopyBufferLayout {
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
				pipeline.last_ptr = current_ptr;
			}
		}
	}

	fn render(
		&self,
		pipeline: &Self::Pipeline,
		encoder: &mut wgpu::CommandEncoder,
		view: &wgpu::TextureView,
		viewport: &Rectangle<u32>,
	) {
		if pipeline.bind_group.is_some() {
			let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
				label: Some("Video Render Pass"),
				color_attachments: &[Some(wgpu::RenderPassColorAttachment {
					view,
					resolve_target: None,
					ops: wgpu::Operations {
						load: wgpu::LoadOp::Load,
						store: wgpu::StoreOp::Store,
					},
					depth_slice: None,
				})],
				depth_stencil_attachment: None,
				timestamp_writes: None,
				occlusion_query_set: None,
			});

			pass.set_viewport(
				viewport.x as f32,
				viewport.y as f32,
				viewport.width as f32,
				viewport.height as f32,
				0.0,
				1.0,
			);

			pass.set_pipeline(&pipeline.pipeline);
			if let Some(bg) = &pipeline.bind_group {
				pass.set_bind_group(0, bg, &[]);
				// Draw 6 vertices (2 triangles)
				pass.draw(0..6, 0..1);
			}
		}
	}
}

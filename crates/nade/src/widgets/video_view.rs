use crate::message::PreviewMessage;
use core::FrameData;
use iced::advanced::graphics::Viewport;
use iced::widget::shader::{Pipeline, Primitive, Program};
use iced::{Color, Rectangle, Vector, mouse};
use std::fmt::Debug;

use wgpu;

use constants::style;

/// Backdrop around the comp frame (pasteboard).
const CANVAS_BG: Color = style::WELL_COLOR;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ViewUniforms {
	viewport_size: [f32; 2],
	frame_size: [f32; 2],
	offset: [f32; 2],
	zoom: f32,
	_pad: f32,
	background: [f32; 4],
}

// Manual impls: this crate depends on `core`, which shadows `::core` in derives.
unsafe impl bytemuck::Zeroable for ViewUniforms {}
unsafe impl bytemuck::Pod for ViewUniforms {}

#[derive(Debug, Clone)]
pub struct VideoView {
	frame: Option<FrameData>,
	zoom: f32,
	offset: Vector,
}

impl VideoView {
	pub fn new(frame: Option<FrameData>, zoom: f32, offset: [f32; 2]) -> Self {
		Self {
			frame,
			zoom: if zoom <= 0.0 { 1.0 } else { zoom },
			offset: Vector::new(offset[0], offset[1]),
		}
	}
}

impl Program<PreviewMessage> for VideoView {
	type State = ();
	type Primitive = VideoPrimitive;

	// 入力はすべて ViewerOverlay が扱う。ここでは描画のみ。
	fn mouse_interaction(
		&self,
		_state: &Self::State,
		_bounds: Rectangle,
		_cursor: mouse::Cursor,
	) -> mouse::Interaction {
		mouse::Interaction::None
	}

	fn draw(&self, _state: &Self::State, _cursor: mouse::Cursor, bounds: Rectangle) -> Self::Primitive {
		let (frame_w, frame_h) = self
			.frame
			.as_ref()
			.map(|f| (f.width as f32, f.height as f32))
			.unwrap_or((1.0, 1.0));

		VideoPrimitive {
			frame: self.frame.clone(),
			uniforms: ViewUniforms {
				viewport_size: [bounds.width, bounds.height],
				frame_size: [frame_w, frame_h],
				offset: [self.offset.x, self.offset.y],
				zoom: self.zoom,
				_pad: 0.0,
				background: [CANVAS_BG.r, CANVAS_BG.g, CANVAS_BG.b, 1.0],
			},
		}
	}
}

#[derive(Debug)]
pub struct VideoPrimitive {
	frame: Option<FrameData>,
	uniforms: ViewUniforms,
}

#[derive(Debug)]
pub struct VideoPipeline {
	texture: Option<wgpu::Texture>,
	uniform_buffer: wgpu::Buffer,
	bind_group: Option<wgpu::BindGroup>,
	pipeline: wgpu::RenderPipeline,
	layout: wgpu::BindGroupLayout,
	sampler: wgpu::Sampler,
	last_ptr: usize,
}

impl Pipeline for VideoPipeline {
	fn new(device: &wgpu::Device, _queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
		let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Video Bind Group Layout"),
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
				wgpu::BindGroupLayoutEntry {
					binding: 2,
					visibility: wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Uniform,
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				},
			],
		});

		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("Video Shader"),
			source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(include_str!("../shaders/video_view.wgsl"))),
		});

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Video Pipeline Layout"),
			bind_group_layouts: &[&layout],
			push_constant_ranges: &[],
		});

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
			cache: None,
		});

		let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("Video View Uniforms"),
			size: std::mem::size_of::<ViewUniforms>() as u64,
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

		let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
			mag_filter: wgpu::FilterMode::Linear,
			min_filter: wgpu::FilterMode::Linear,
			address_mode_u: wgpu::AddressMode::ClampToEdge,
			address_mode_v: wgpu::AddressMode::ClampToEdge,
			..Default::default()
		});

		Self {
			texture: None,
			uniform_buffer,
			bind_group: None,
			pipeline,
			layout,
			sampler,
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
		bounds: &Rectangle,
		viewport: &Viewport,
	) {
		// iced は render 時に bounds * scale_factor を viewport にする。
		// uniforms も物理ピクセルに揃えないと Retina でサンプリングがずれる。
		let scale = viewport.scale_factor();
		let uniforms = ViewUniforms {
			viewport_size: [bounds.width * scale, bounds.height * scale],
			frame_size: self.uniforms.frame_size,
			offset: [self.uniforms.offset[0] * scale, self.uniforms.offset[1] * scale],
			zoom: self.uniforms.zoom,
			_pad: 0.0,
			background: self.uniforms.background,
		};
		queue.write_buffer(&pipeline.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

		let Some(frame) = &self.frame else {
			pipeline.bind_group = None;
			return;
		};

		let width = frame.width;
		let height = frame.height;

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
						resource: wgpu::BindingResource::Sampler(&pipeline.sampler),
					},
					wgpu::BindGroupEntry {
						binding: 2,
						resource: pipeline.uniform_buffer.as_entire_binding(),
					},
				],
			});

			pipeline.texture = Some(texture);
			pipeline.bind_group = Some(bind_group);
		}

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

	fn draw(&self, pipeline: &Self::Pipeline, render_pass: &mut wgpu::RenderPass<'_>) -> bool {
		let Some(bind_group) = &pipeline.bind_group else {
			return true;
		};

		render_pass.set_pipeline(&pipeline.pipeline);
		render_pass.set_bind_group(0, bind_group, &[]);
		render_pass.draw(0..6, 0..1);
		true
	}
}

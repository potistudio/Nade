use crate::message::PreviewMessage;
use core::FrameData;
use iced::advanced::graphics::Viewport;
use iced::keyboard;
use iced::widget::shader::{Action, Pipeline, Primitive, Program};
use iced::{Color, Event, Point, Rectangle, Size, Vector, mouse};
use std::fmt::Debug;

use wgpu;

const ZOOM_SPEED: f32 = 0.01;
const MIN_ZOOM: f32 = 0.05;
const MAX_ZOOM: f32 = 32.0;
const SCROLL_LINE_PX: f32 = 20.0;
const CANVAS_BG: Color = Color::from_rgb8(18, 23, 29);

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

/// Ephemeral interaction state only (persists while the shader widget stays mounted).
#[derive(Debug, Default)]
pub struct VideoViewState {
	modifiers: keyboard::Modifiers,
	panning: Option<Point>,
}

fn fit_scale(viewport: Size, frame_w: f32, frame_h: f32) -> f32 {
	(viewport.width / frame_w.max(1.0)).min(viewport.height / frame_h.max(1.0))
}

fn frame_origin(zoom: f32, offset: Vector, viewport: Size, frame_w: f32, frame_h: f32) -> Point {
	let display_scale = fit_scale(viewport, frame_w, frame_h) * zoom;
	let display_w = frame_w * display_scale;
	let display_h = frame_h * display_scale;
	Point::new(
		(viewport.width - display_w) * 0.5 + offset.x,
		(viewport.height - display_h) * 0.5 + offset.y,
	)
}

fn screen_to_local(zoom: f32, offset: Vector, screen: Point, viewport: Size, frame_w: f32, frame_h: f32) -> Point {
	let origin = frame_origin(zoom, offset, viewport, frame_w, frame_h);
	let display_scale = fit_scale(viewport, frame_w, frame_h) * zoom;
	Point::new(
		(screen.x - origin.x) / display_scale,
		(screen.y - origin.y) / display_scale,
	)
}

fn zoom_at(
	zoom: f32,
	offset: Vector,
	delta: f32,
	cursor: Point,
	viewport: Size,
	frame_w: f32,
	frame_h: f32,
) -> (f32, Vector) {
	let local = screen_to_local(zoom, offset, cursor, viewport, frame_w, frame_h);
	let new_zoom = (zoom * (1.0 + delta * ZOOM_SPEED)).clamp(MIN_ZOOM, MAX_ZOOM);
	if (new_zoom - zoom).abs() < f32::EPSILON {
		return (zoom, offset);
	}

	let display_scale = fit_scale(viewport, frame_w, frame_h) * new_zoom;
	let display_w = frame_w * display_scale;
	let display_h = frame_h * display_scale;
	let center_origin = Point::new((viewport.width - display_w) * 0.5, (viewport.height - display_h) * 0.5);
	let new_offset = Vector::new(
		cursor.x - center_origin.x - local.x * display_scale,
		cursor.y - center_origin.y - local.y * display_scale,
	);
	(new_zoom, new_offset)
}

fn view_changed(zoom: f32, offset: Vector) -> PreviewMessage {
	PreviewMessage::ViewChanged {
		zoom,
		offset: [offset.x, offset.y],
	}
}

impl Program<PreviewMessage> for VideoView {
	type State = VideoViewState;
	type Primitive = VideoPrimitive;

	fn update(
		&self,
		state: &mut Self::State,
		event: &Event,
		bounds: Rectangle,
		cursor: mouse::Cursor,
	) -> Option<Action<PreviewMessage>> {
		let Some(frame) = &self.frame else {
			return None;
		};
		let frame_w = frame.width as f32;
		let frame_h = frame.height as f32;
		let viewport = bounds.size();

		match event {
			Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
				state.modifiers = *modifiers;
				None
			}
			Event::Mouse(mouse_event) => match mouse_event {
				mouse::Event::ButtonPressed(mouse::Button::Middle) => {
					let cursor_pos = cursor.position_in(bounds)?;
					state.panning = Some(cursor_pos);
					Some(Action::capture())
				}
				mouse::Event::ButtonReleased(mouse::Button::Middle) => {
					if state.panning.take().is_some() {
						Some(Action::capture())
					} else {
						None
					}
				}
				mouse::Event::CursorMoved { .. } => {
					let cursor_pos = cursor.position_in(bounds)?;
					if let Some(prev) = state.panning {
						let offset = Vector::new(
							self.offset.x + cursor_pos.x - prev.x,
							self.offset.y + cursor_pos.y - prev.y,
						);
						state.panning = Some(cursor_pos);
						Some(Action::publish(view_changed(self.zoom, offset)).and_capture())
					} else {
						None
					}
				}
				mouse::Event::WheelScrolled { delta } => {
					let cursor_pos = cursor.position_in(bounds)?;
					let (dx, dy) = match delta {
						mouse::ScrollDelta::Lines { x, y } => (*x * SCROLL_LINE_PX, *y * SCROLL_LINE_PX),
						mouse::ScrollDelta::Pixels { x, y } => (*x, *y),
					};

					let (zoom, offset) = if state.modifiers.control() {
						zoom_at(self.zoom, self.offset, dy, cursor_pos, viewport, frame_w, frame_h)
					} else if state.modifiers.shift() {
						(self.zoom, Vector::new(self.offset.x + dy, self.offset.y))
					} else {
						(self.zoom, Vector::new(self.offset.x + dx, self.offset.y + dy))
					};
					Some(Action::publish(view_changed(zoom, offset)).and_capture())
				}
				_ => None,
			},
			_ => None,
		}
	}

	fn mouse_interaction(&self, state: &Self::State, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
		if state.panning.is_some() {
			return mouse::Interaction::Grabbing;
		}
		if cursor.is_over(bounds) {
			mouse::Interaction::Grab
		} else {
			mouse::Interaction::default()
		}
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
		_bounds: &Rectangle,
		_storage: &Viewport,
	) {
		queue.write_buffer(&pipeline.uniform_buffer, 0, bytemuck::bytes_of(&self.uniforms));

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

	fn render(
		&self,
		pipeline: &Self::Pipeline,
		encoder: &mut wgpu::CommandEncoder,
		view: &wgpu::TextureView,
		viewport: &Rectangle<u32>,
	) {
		if pipeline.bind_group.is_none() {
			return;
		}

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
			pass.draw(0..6, 0..1);
		}
	}
}

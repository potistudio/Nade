//! Entry point of Nade
//!
//! A raw pixel viewer application built with Iced, demonstrating
//! real-time pixel buffer updates with animated sine wave patterns.

use iced::widget::{Image, column, container, image, row, slider, text};
use iced::{Color, Element, Length, Subscription, Theme, time};
use std::time::{Duration, Instant};

/// Application entry point
pub fn main() -> iced::Result {
	iced::application(PixelViewer::default, PixelViewer::update, PixelViewer::view)
		.subscription(PixelViewer::subscription)
		.theme(theme)
		.title("Raw Pixel Viewer")
		.window_size((800.0, 600.0))
		.run()
}

/// Return the application theme
fn theme(_state: &PixelViewer) -> Theme {
	Theme::Dark
}

/// Application state
#[derive(Debug)]
struct PixelViewer {
	/// Width of the pixel buffer
	width: u32,
	/// Height of the pixel buffer
	height: u32,
	/// Raw RGBA pixel data
	pixels: Vec<u8>,
	/// Animation time value
	time: f32,
	/// Last frame timestamp for FPS calculation
	last_frame: Option<Instant>,
	/// Current FPS value
	fps: f32,
	/// Status bar component
	status_bar: status_bar::StatusBar,
}

impl PixelViewer {
	/// Create a new default PixelViewer
	fn default() -> Self {
		let width = 320;
		let height = 240;
		let pixels = vec![0u8; (width * height * 4) as usize];

		Self {
			width,
			height,
			pixels,
			time: 0.0,
			last_frame: None,
			fps: 0.0,
			status_bar: status_bar::StatusBar::new(),
		}
	}
}

/// Messages that can be sent to the application
#[derive(Debug, Clone)]
enum Message {
	/// Animation tick with current timestamp
	Tick(Instant),
	/// Time slider value changed
	TimeChanged(f32),
}

impl PixelViewer {
	/// Update the application state based on received messages
	fn update(&mut self, message: Message) {
		match message {
			Message::Tick(now) => {
				// Calculate delta time
				let dt = if let Some(last) = self.last_frame {
					now.duration_since(last).as_secs_f32()
				} else {
					0.016 // Assume ~60fps for first frame
				};
				self.last_frame = Some(now);

				// Update FPS (smoothed)
				if dt > 0.0 {
					self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
				}

				// Update time and regenerate pixels
				self.time += dt;
				self.update_pixels();
			}
			Message::TimeChanged(value) => {
				self.time = value;
				self.update_pixels();
			}
		}
	}

	/// Update the pixel buffer with animated sine wave pattern
	fn update_pixels(&mut self) {
		let w = self.width as i32;
		let h = self.height as i32;
		let t = self.time;

		for y in 0..h {
			for x in 0..w {
				let fx = x as f32 / w as f32;
				let fy = y as f32 / h as f32;

				// Animated pattern using sine waves
				let r = ((fx * 10.0 + t).sin() * 0.5 + 0.5) * 255.0;
				let g = ((fy * 10.0 - t * 1.3).cos() * 0.5 + 0.5) * 255.0;
				let d = ((fx - 0.5).powi(2) + (fy - 0.5).powi(2)).sqrt();
				let b = (((d * 20.0) - t * 0.7).sin() * 0.5 + 0.5) * 255.0;

				let idx = ((y * w + x) * 4) as usize;
				self.pixels[idx] = r as u8; // R
				self.pixels[idx + 1] = g as u8; // G
				self.pixels[idx + 2] = b as u8; // B
				self.pixels[idx + 3] = 255; // A
			}
		}
	}

	/// Render the application UI
	fn view(&self) -> Element<'_, Message> {
		// Create image from pixel buffer
		let handle = image::Handle::from_rgba(self.width, self.height, self.pixels.clone());

		// Header
		let header = text("Raw Pixel Viewer (Iced)").size(20);

		// Time slider
		let time_control = row![
			text("Time:").size(14),
			slider(0.0..=100.0, self.time, Message::TimeChanged).width(Length::Fixed(200.0)),
			text(format!("{:.1}", self.time)).size(14),
		]
		.spacing(10)
		.align_y(iced::Alignment::Center);

		// FPS display
		let fps_text = text(format!("{:.1} FPS", self.fps))
			.size(14)
			.color(Color::from_rgb(0.4, 0.8, 1.0));

		// Main image display
		let img = Image::new(handle)
			.content_fit(iced::ContentFit::Contain)
			.width(Length::Fill)
			.height(Length::Fill);

		// Image container with FPS overlay
		let image_container = container(img).width(Length::Fill).height(Length::Fill);

		// Main content column
		let content = column![header, time_control, fps_text, image_container,]
			.spacing(10)
			.padding(10)
			.width(Length::Fill)
			.height(Length::Fill);

		// Add status bar at bottom
		let main_layout = column![
			container(content).width(Length::Fill).height(Length::Fill),
			container(self.status_bar.view())
				.width(Length::Fill)
				.style(|_theme| container::Style {
					background: Some(iced::Background::Color(Color::from_rgb(0.1, 0.1, 0.1))),
					..Default::default()
				}),
		]
		.width(Length::Fill)
		.height(Length::Fill);

		main_layout.into()
	}

	/// Subscribe to time-based events for animation
	fn subscription(&self) -> Subscription<Message> {
		// Request updates at ~60 FPS
		time::every(Duration::from_millis(16)).map(Message::Tick)
	}
}

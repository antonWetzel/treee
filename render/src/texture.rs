use image::GenericImageView;
use nalgebra as na;

use crate::State;

pub struct Texture {
	pub size: na::Point2<u32>,
	pub gpu: wgpu::Texture,
}

pub type TextureDimension = wgpu::TextureDimension;

impl Texture {
	pub fn new(state: &State, data: &[u8], format: wgpu::TextureFormat) -> Self {
		Self::new_xd(state, data, wgpu::TextureDimension::D2, format)
	}

	pub fn new_1d(state: &State, data: &[u8], format: wgpu::TextureFormat) -> Self {
		Self::new_xd(state, data, wgpu::TextureDimension::D1, format)
	}

	fn new_xd(state: &State, data: &[u8], dimension: TextureDimension, format: wgpu::TextureFormat) -> Self {
		let img = image::load_from_memory(data).unwrap();
		let dimensions = img.dimensions();

		let texture_size = wgpu::Extent3d {
			width: dimensions.0,
			height: dimensions.1,
			depth_or_array_layers: 1,
		};
		let texture = state.device.create_texture(&wgpu::TextureDescriptor {
			size: texture_size,
			mip_level_count: 1,
			sample_count: 1,
			dimension,
			format,
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
			label: Some("lookup texture"),
			view_formats: &[],
		});

		state.queue.write_texture(
			wgpu::ImageCopyTexture {
				texture: &texture,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
				aspect: wgpu::TextureAspect::All,
			},
			&img.to_rgba8(),
			wgpu::ImageDataLayout {
				offset: 0,
				bytes_per_row: Some(4 * dimensions.0),
				rows_per_image: Some(dimensions.1),
			},
			texture_size,
		);

		Self {
			size: [dimensions.0, dimensions.1].into(),
			gpu: texture,
		}
	}
}

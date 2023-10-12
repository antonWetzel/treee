use image::GenericImageView;
use wgpu::util::DeviceExt;

use crate::{Has, State};

pub struct Lookup {
	bind_group: wgpu::BindGroup,
}

impl Lookup {
	pub fn new_png(state: &impl Has<State>, data: &[u8]) -> Self {
		let state: &State = state.get();
		let img = image::load_from_memory(data).unwrap();
		let dimensions = img.dimensions();

		assert!(dimensions.0.is_power_of_two());
		assert_eq!(dimensions.1, 1);

		let texture_size = wgpu::Extent3d {
			width: dimensions.0,
			height: 1,
			depth_or_array_layers: 1,
		};
		let texture = state.device.create_texture(&wgpu::TextureDescriptor {
			// All textures are stored as 3D, we represent our 2D texture
			// by setting depth to 1.
			size: texture_size,
			mip_level_count: 1, // We'll talk about this a little later
			sample_count: 1,
			dimension: wgpu::TextureDimension::D1,
			// Most images are stored using sRGB so we need to reflect that here.
			format: wgpu::TextureFormat::Rgba8UnormSrgb,
			// TEXTURE_BINDING tells wgpu that we want to use this texture in shaders
			// COPY_DST means that we want to copy data to this texture
			usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
			label: Some("lookup texture"),
			// This is the same as with the SurfaceConfig. It
			// specifies what texture formats can be used to
			// create TextureViews for this texture. The base
			// texture format (Rgba8UnormSrgb in this case) is
			// always supported. Note that using a different
			// texture format is not supported on the WebGL2
			// backend.
			view_formats: &[],
		});

		state.queue.write_texture(
			// Tells wgpu where to copy the pixel data
			wgpu::ImageCopyTexture {
				texture: &texture,
				mip_level: 0,
				origin: wgpu::Origin3d::ZERO,
				aspect: wgpu::TextureAspect::All,
			},
			// The actual pixel data
			&img.to_rgba8(),
			// The layout of the texture
			wgpu::ImageDataLayout {
				offset: 0,
				bytes_per_row: Some(4 * dimensions.0),
				rows_per_image: Some(dimensions.1),
			},
			texture_size,
		);
		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

		let bind_group_layout = Self::get_layout(state);

		let scale = dimensions.0.leading_zeros() + 1;

		let buffer = state
			.device
			.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Camera Buffer"),
				contents: bytemuck::cast_slice(&[scale]),
				usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
			});

		let bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(&view),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: buffer.as_entire_binding(),
				},
			],
			label: Some("diffuse_bind_group"),
		});

		Self { bind_group }
	}

	pub fn get_bind_group(&self) -> &wgpu::BindGroup {
		&self.bind_group
	}

	pub fn get_layout(state: &impl Has<State>) -> wgpu::BindGroupLayout {
		state
			.get()
			.device
			.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				entries: &[
					wgpu::BindGroupLayoutEntry {
						binding: 0,
						visibility: wgpu::ShaderStages::FRAGMENT,
						ty: wgpu::BindingType::Texture {
							multisampled: false,
							view_dimension: wgpu::TextureViewDimension::D1,
							sample_type: wgpu::TextureSampleType::Float { filterable: true },
						},
						count: None,
					},
					wgpu::BindGroupLayoutEntry {
						binding: 1,
						visibility: wgpu::ShaderStages::FRAGMENT,
						ty: wgpu::BindingType::Buffer {
							ty: wgpu::BufferBindingType::Uniform,
							has_dynamic_offset: false,
							min_binding_size: None,
						},
						count: None,
					},
				],
				label: Some("lookup layout"),
			})
	}
}

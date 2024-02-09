use math::{X, Y};
use wgpu::util::DeviceExt;

use crate::{Has, State, Texture};

pub struct Lookup {
	bind_group: wgpu::BindGroup,
}

impl Lookup {
	pub fn new_png(state: &impl Has<State>, data: &[u8], range: u32) -> Self {
		let state = state.get();
		let texture = Texture::new_1d(state, data);
		assert!(texture.size[X].is_power_of_two());
		assert_eq!(texture.size[Y], 1);

		let view = texture.gpu.create_view(&Default::default());

		let bind_group_layout = Self::get_layout(state);

		let scale = range / texture.size[X] + 1;

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

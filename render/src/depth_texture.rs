pub struct DepthTexture {
	pub view: wgpu::TextureView,
}

impl DepthTexture {
	pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

	pub fn new(device: &wgpu::Device, size: wgpu::Extent3d, label: &str) -> Self {
		let desc = wgpu::TextureDescriptor {
			label: Some(label),
			size,
			mip_level_count: 1,
			sample_count: 1,
			dimension: wgpu::TextureDimension::D2,
			format: Self::DEPTH_FORMAT,
			view_formats: &[],
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
		};
		let texture = device.create_texture(&desc);
		let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

		Self { view }
	}
}

pub struct RenderPass<'a>(wgpu::RenderPass<'a>);


impl<'a> RenderPass<'a> { }


impl<'a> std::ops::Deref for RenderPass<'a> {
	type Target = wgpu::RenderPass<'a>;


	fn deref(&self) -> &Self::Target {
		&self.0
	}
}


impl<'a> std::ops::DerefMut for RenderPass<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}


impl<'a> RenderPass<'a> {
	pub fn new(render_pass: wgpu::RenderPass<'a>) -> Self {
		Self(render_pass)
	}


	pub fn render<Data, R: Render<'a, Data>>(&mut self, value: &'a R, data: Data) {
		value.render(self, data);
	}
}


pub trait Render<'a, Data> {
	fn render(&'a self, render_pass: &mut RenderPass<'a>, data: Data);
}

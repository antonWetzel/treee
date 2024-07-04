use crate::{environment, program::Event};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct Empty {
	sender: crossbeam::channel::Sender<Event>,
}

impl Empty {
	pub fn new() -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		sender.send(Event::ClearPointClouds).unwrap();
		(Self { sender }, receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui) {
		if ui
			.add_sized([ui.available_width(), 0.0], egui::Button::new("Load File"))
			.clicked()
		{
			environment::get_source(&self.sender);
		}

		#[cfg(target_arch = "wasm32")]
		if ui
			.add_sized([ui.available_width(), 0.0], egui::Button::new("Example"))
			.clicked()
		{
			let sender = self.sender.clone();
			wasm_bindgen_futures::spawn_local(async move {
				fetch_example(sender).await.unwrap();
			});
		}
	}
}

#[cfg(target_arch = "wasm32")]
async fn fetch_example(sender: crossbeam::channel::Sender<Event>) -> Result<(), JsValue> {
	use js_sys::Uint8Array;
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{Request, RequestInit, RequestMode, Response};

	let mut opts = RequestInit::new();
	opts.method("GET");
	opts.mode(RequestMode::Cors);

	let url = "./example/ALS-on_BR03_2019-07-05_300m.laz";
	let request = Request::new_with_str_and_init(&url, &opts)?;

	request
		.headers()
		.set("Accept", "application/vnd.github.v3+json")?;

	let window = web_sys::window().unwrap();
	let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

	let resp: Response = resp_value.dyn_into().unwrap();

	let blob = JsFuture::from(resp.blob()?).await?;
	let blob: web_sys::Blob = blob.dyn_into().unwrap();
	let array = JsFuture::from(blob.array_buffer()).await?;
	let array = Uint8Array::new(&array);
	let data = array.to_vec();

	// log::warn!("{:?}", data);
	sender
		.send(Event::Load((data, "example.laz".into())))
		.unwrap();

	Ok(())
}

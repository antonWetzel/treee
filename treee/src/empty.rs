use crate::{environment, program::Event};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub struct Empty {
	sender: crossbeam::channel::Sender<Event>,
	progress: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

impl Empty {
	pub fn new() -> (Self, crossbeam::channel::Receiver<Event>) {
		let (sender, receiver) = crossbeam::channel::unbounded();
		_ = sender.send(Event::ClearPointClouds);
		(Self {
			sender,
			progress: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(1000)),
		}, receiver)
	}

	pub fn ui(&self, ui: &mut egui::Ui) {
		if ui
			.add_sized([ui.available_width(), 0.0], egui::Button::new("Load"))
			.clicked()
		{
			environment::Source::new(&self.sender);
		}

		#[cfg(target_arch = "wasm32")]
		{
			ui.separator();
			let load  = ui.add_sized([ui.available_width(), 0.0], egui::Button::new("Example"));


			if load.clicked() {
				let sender = self.sender.clone();
				let progress = self.progress.clone();
				wasm_bindgen_futures::spawn_local(async move {
					fetch_example(sender, progress).await.unwrap();
				});
			}
			ui.hyperlink("https://doi.org/10.1594/PANGAEA.942856");
			ui.label("ALS-on_BR04_2019-07-05_140m.laz");

			let progress = self.progress.load(std::sync::atomic::Ordering::Relaxed);
			if progress != 1000 {
				let progress = progress as f32 / 1000.0;
				ui.add(egui::ProgressBar::new(progress).rounding(egui::Rounding::ZERO));
			}
		}
	}
}

#[cfg(target_arch = "wasm32")]
async fn fetch_example(
	sender: crossbeam::channel::Sender<Event>,
	progress: std::sync::Arc<std::sync::atomic::AtomicUsize>,
) -> Result<(), JsValue> {
	use js_sys::Uint8Array;
	use wasm_bindgen_futures::JsFuture;
	use web_sys::{ReadableStreamDefaultReader, Request, RequestInit, RequestMode, Response};

	let mut opts = RequestInit::new();
	opts.method("GET");
	opts.mode(RequestMode::Cors);

	let url = "./example/ALS-on_BR04_2019-07-05_140m.laz";
	let request = Request::new_with_str_and_init(&url, &opts)?;

	let window = web_sys::window().unwrap();
	let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

	let resp: Response = resp_value.dyn_into().unwrap();
	let headers = resp.headers();
	let length: usize = headers.get("Content-Length").unwrap().as_ref().map_or("0s", |v| v).parse().unwrap();
	let data = if length == 0 {
		let blob = JsFuture::from(resp.blob()?).await?;
		let blob: web_sys::Blob = blob.dyn_into().unwrap();
		let array = JsFuture::from(blob.array_buffer()).await?;
		let array = Uint8Array::new(&array);
		array.to_vec()
	} else {
		let reader: ReadableStreamDefaultReader = resp.body().unwrap().get_reader().dyn_into().unwrap();
		let mut data = Vec::with_capacity(length);
		loop {
			let chunk = JsFuture::from(reader.read()).await?;
			let done = js_sys::Reflect::get(&chunk, &JsValue::from_str("done")).unwrap().as_bool().unwrap();
			if done {
				break;
			}
			let value = js_sys::Reflect::get(&chunk, &JsValue::from_str("value")).unwrap();
			let value = js_sys::Uint8Array::new(&value).to_vec();
			data.extend_from_slice(&value);
			let p = (data.len() as f32 / length as f32 * 1000.0) as usize;
			progress.store(p, std::sync::atomic::Ordering::Relaxed);
		}
		progress.store(1000, std::sync::atomic::Ordering::Relaxed);
		data
	};
	
	_ = sender.send(Event::Load(environment::Source::from_data(
		data,
		"example.laz".into(),
	)));

	return Ok(());


}

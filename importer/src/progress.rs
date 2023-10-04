use std::io::Write;

pub struct Progress {
	name: String,
	current: usize,
	last_progess: usize,
	total: usize,
}

impl Progress {
	pub fn new(name: String, total: usize) -> Self {
		Self {
			name,
			current: 0,
			total,
			last_progess: 1001,
		}
	}

	pub fn increase(&mut self) {
		self.current += 1;
		let progress = (self.current * 1000) / self.total;
		if progress != self.last_progess {
			if progress == 1000 {
				print!("{}: 1.{:03}\r", self.name, 0);
			} else {
				print!("{}: 0.{:03}\r", self.name, progress);
			}
			std::io::stdout().flush().unwrap();
			self.last_progess = progress;
		}
	}
}

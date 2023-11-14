use std::{
	io::Write,
	time::{Duration, Instant},
};

pub struct Progress<'a> {
	start: Instant,
	time: Instant,
	current: usize,
	goal: usize,
	name: &'a str,
}

impl<'a> Progress<'a> {
	pub fn new(name: &'a str, goal: usize) -> Self {
		let start = Instant::now();
		Self::print(name, 0, goal, start);
		Self {
			start,
			time: start,
			current: 0,
			goal,
			name,
		}
	}

	pub fn step(&mut self) {
		self.current += 1;
		self.maybe_print();
	}

	fn maybe_print(&mut self) {
		let now = Instant::now();
		if now.duration_since(self.time) > Duration::from_millis(100) {
			self.time = now;
			Self::print(self.name, self.current, self.goal, self.start);
		}
	}

	pub fn step_by(&mut self, amount: usize) {
		self.current += amount;
		self.maybe_print();
	}

	const SUB_STEPS: &str = " ▏▎▍▌▋▊▉";

	fn print(name: &str, progress: usize, goal: usize, start: Instant) {
		let (hours, minutes, seconds) = time(start);

		let sub_length = Self::SUB_STEPS.chars().count();
		let mut size = termsize::get().map(|s| s.cols as usize).unwrap_or(80);
		if size > 30 {
			size -= 30;
		}

		let used = progress * size * sub_length / goal;
		let left = used / sub_length;
		let (left, sub, right) = if left < size {
			(
				left,
				Self::SUB_STEPS
					.chars()
					.skip(used % sub_length)
					.next()
					.unwrap(),
				size - left - 1,
			)
		} else {
			(left - 1, '█', 0)
		};

		print!(
			"{}:{: <width$} [{:0>2}:{:0>2}:{:0>2}] █{:█<left$}{}{:<right$}█\r",
			name,
			"",
			hours,
			minutes,
			seconds,
			"",
			sub,
			"",
			width = 15 - name.len(),
			left = left,
			right = right
		);
		std::io::stdout().flush().unwrap();
	}

	pub fn finish(self) {
		Self::print(self.name, self.goal, self.goal, self.start);
		print!("\n")
	}
}

fn time(start: Instant) -> (u64, u64, u64) {
	let seconds = start.elapsed().as_secs();

	let minutes = seconds / 60;
	let hours = minutes / 60;
	let minutes = minutes - hours * 60;
	let seconds = seconds - minutes * 60 - hours * 60 * 60;

	(hours, minutes, seconds)
}

pub struct Stage<'a> {
	start: Instant,
	name: &'a str,
}

impl<'a> Stage<'a> {
	pub fn new(name: &'a str) -> Self {
		print!(
			"{}:{: >width$} [00:00:00] ...\r",
			name,
			"",
			width = 15 - name.len()
		);
		std::io::stdout().flush().unwrap();

		Self { start: Instant::now(), name }
	}

	pub fn finish(self) {
		let (hours, minutes, seconds) = time(self.start);
		println!(
			"{}:{: >width$} [{:0>2}:{:0>2}:{:0>2}] ...",
			self.name,
			"",
			hours,
			minutes,
			seconds,
			width = 15 - self.name.len()
		);
	}
}

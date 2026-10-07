use std::collections::VecDeque;

/// Lets one run go at a time. Runs submitted meanwhile wait, and go in the order they came.
pub(super) struct RunQueue<S> {
	active: bool,
	waiting: VecDeque<S>,
}

impl<S> RunQueue<S> {
	pub(super) const fn new() -> Self {
		Self { active: false, waiting: VecDeque::new() }
	}

	/// Returns `start` if no run is active, and keeps it waiting otherwise.
	pub(super) fn submit(&mut self, start: S) -> Option<S> {
		if self.active {
			self.waiting.push_back(start);
			None
		} else {
			self.active = true;
			Some(start)
		}
	}

	/// Ends the active run and returns the next waiting one, which is then active.
	pub(super) fn finish(&mut self) -> Option<S> {
		let next = self.waiting.pop_front();
		self.active = next.is_some();
		next
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn first_run_starts_at_once() {
		let mut runs = RunQueue::new();
		assert_eq!(runs.submit(1), Some(1));
	}

	#[test]
	fn run_submitted_while_one_is_active_waits() {
		let mut runs = RunQueue::new();
		runs.submit(1);
		assert_eq!(runs.submit(2), None);
	}

	#[test]
	fn waiting_runs_start_in_submission_order() {
		let mut runs = RunQueue::new();
		runs.submit(1);
		runs.submit(2);
		runs.submit(3);
		assert_eq!(runs.finish(), Some(2));
		assert_eq!(runs.finish(), Some(3));
		assert_eq!(runs.finish(), None);
	}

	#[test]
	fn run_handed_out_by_finish_is_active() {
		let mut runs = RunQueue::new();
		runs.submit(1);
		runs.submit(2);
		runs.finish();
		assert_eq!(runs.submit(3), None);
	}

	#[test]
	fn queue_is_idle_after_the_last_run_finishes() {
		let mut runs = RunQueue::new();
		runs.submit(1);
		runs.finish();
		assert_eq!(runs.submit(2), Some(2));
	}
}

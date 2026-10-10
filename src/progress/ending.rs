use super::Ended;

/// Holds a run's end back until both of its halves have happened: the work returning, which says
/// how the run ended, and the window being destroyed. wx destroys a window some time after it is
/// closed, and only then enables the application's other windows again, so a callback that runs
/// earlier cannot focus or open anything in them.
pub(super) struct Ending {
	ended: Option<Ended>,
	window_destroyed: bool,
}

impl Ending {
	pub(super) const fn new() -> Self {
		Self { ended: None, window_destroyed: false }
	}

	/// The work returned; gives how the run ended if its window is gone as well.
	pub(super) const fn work_returned(&mut self, ended: Ended) -> Option<Ended> {
		if self.ended.is_some() {
			return None;
		}
		self.ended = Some(ended);
		if self.window_destroyed { Some(ended) } else { None }
	}

	/// The window was destroyed; gives how the run ended if its work has returned as well.
	pub(super) const fn window_destroyed(&mut self) -> Option<Ended> {
		if self.window_destroyed {
			return None;
		}
		self.window_destroyed = true;
		self.ended
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::progress::Ended;

	#[test]
	fn a_run_whose_window_is_still_there_has_not_ended() {
		let mut ending = Ending::new();
		assert_eq!(ending.work_returned(Ended::Completed), None);
	}

	#[test]
	fn a_run_whose_work_still_runs_has_not_ended() {
		let mut ending = Ending::new();
		assert_eq!(ending.window_destroyed(), None);
	}

	#[test]
	fn a_run_ends_when_its_window_goes_after_its_work_returned() {
		let mut ending = Ending::new();
		ending.work_returned(Ended::Completed);
		assert_eq!(ending.window_destroyed(), Some(Ended::Completed));
	}

	/// Cancel destroys the window at once, while the work runs on until it notices.
	#[test]
	fn a_run_ends_when_its_work_returns_after_its_window_went() {
		let mut ending = Ending::new();
		ending.window_destroyed();
		assert_eq!(ending.work_returned(Ended::Cancelled), Some(Ended::Cancelled));
	}

	#[test]
	fn a_run_ends_once() {
		let mut ending = Ending::new();
		ending.work_returned(Ended::Completed);
		ending.window_destroyed();
		assert_eq!(ending.window_destroyed(), None);
		assert_eq!(ending.work_returned(Ended::Completed), None);
	}
}

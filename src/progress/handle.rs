use std::sync::{
	Arc, Mutex, PoisonError,
	atomic::{AtomicBool, AtomicU64, Ordering},
	mpsc,
};

use wxdragon::window::WxWidget;

use super::lifecycle::percent;

/// What the worker and the UI thread share during one run.
#[derive(Default)]
pub(super) struct Shared {
	done: AtomicU64,
	/// Zero while the total is unknown.
	total: AtomicU64,
	cancelled: AtomicBool,
	finished: AtomicBool,
	message: Mutex<Option<String>>,
}

impl Shared {
	/// The gauge value for the progress so far, or `None` to pulse.
	pub(super) fn percent(&self) -> Option<i32> {
		percent(self.done.load(Ordering::Relaxed), self.total.load(Ordering::Relaxed))
	}

	/// The message set since the last call, if any.
	pub(super) fn take_message(&self) -> Option<String> {
		self.message.lock().unwrap_or_else(PoisonError::into_inner).take()
	}

	pub(super) fn cancel(&self) {
		self.cancelled.store(true, Ordering::Relaxed);
	}

	pub(super) fn is_cancelled(&self) -> bool {
		self.cancelled.load(Ordering::Relaxed)
	}

	/// Marks the work as returned.
	pub(super) fn finish(&self) {
		self.finished.store(true, Ordering::Relaxed);
	}

	/// Whether the work is still going and nobody pressed Cancel.
	pub(super) fn is_running(&self) -> bool {
		!self.finished.load(Ordering::Relaxed) && !self.is_cancelled()
	}
}

type Show = Box<dyn FnOnce(&dyn WxWidget) + Send>;

/// A question from the worker, to be shown on the UI thread.
pub(super) struct Question(Show);

impl Question {
	/// Shows the question over `parent` and hands the answer to the waiting worker.
	pub(super) fn show(self, parent: &dyn WxWidget) {
		(self.0)(parent);
	}
}

/// The worker's side of a run: reports progress, notices Cancel and asks the user questions.
///
/// [`run_with_progress`](super::run_with_progress) passes it to the work.
pub struct Progress {
	shared: Arc<Shared>,
}

impl Progress {
	pub(super) const fn new(shared: Arc<Shared>) -> Self {
		Self { shared }
	}

	/// Moves the bar to `done` out of `total`. Without a total, or with a total of zero, the bar
	/// pulses instead.
	pub fn set(&self, done: u64, total: Option<u64>) {
		self.shared.total.store(total.unwrap_or(0), Ordering::Relaxed);
		self.shared.done.store(done, Ordering::Relaxed);
	}

	/// Replaces the message in the progress window. Pass translated text.
	pub fn set_message(&self, message: &str) {
		*self.shared.message.lock().unwrap_or_else(PoisonError::into_inner) = Some(message.to_owned());
	}

	/// Whether the user pressed Cancel. The work should then return as soon as it can.
	#[must_use]
	pub fn is_cancelled(&self) -> bool {
		self.shared.is_cancelled()
	}

	/// The Cancel flag itself, for functions that watch an `AtomicBool`. It turns `true` when the
	/// user presses Cancel.
	#[must_use]
	pub fn cancel_flag(&self) -> &AtomicBool {
		&self.shared.cancelled
	}

	/// Runs `show` on the UI thread with the progress window as its parent, and waits for its
	/// result. The window stops updating while `show` runs, so `show` can open a modal dialog, such
	/// as a Yes/No question.
	///
	/// Returns `None` without calling `show` when the run is cancelled.
	pub fn ask<R: Send + 'static>(&self, show: impl FnOnce(&dyn WxWidget) -> R + Send + 'static) -> Option<R> {
		self.ask_with(show, |question| super::post_question(Arc::clone(&self.shared), question))
	}

	/// Sends `show` to `post` as a [`Question`] and waits for its answer. `None` when the run is
	/// cancelled or the question is dropped unanswered.
	fn ask_with<R: Send + 'static>(
		&self,
		show: impl FnOnce(&dyn WxWidget) -> R + Send + 'static,
		post: impl FnOnce(Question),
	) -> Option<R> {
		if self.is_cancelled() {
			return None;
		}
		let (reply, answer) = mpsc::channel();
		post(Question(Box::new(move |parent| {
			let _ = reply.send(show(parent));
		})));
		answer.recv().ok()
	}
}

#[cfg(test)]
mod tests {
	use std::{ptr, thread};

	use wxdragon::ffi;

	use super::*;

	struct NoWindow;

	impl WxWidget for NoWindow {
		fn handle_ptr(&self) -> *mut ffi::wxd_Window_t {
			ptr::null_mut()
		}
	}

	fn progress() -> Progress {
		Progress::new(Arc::new(Shared::default()))
	}

	#[test]
	fn known_total_moves_the_bar() {
		let progress = progress();
		progress.set(25, Some(100));
		assert_eq!(progress.shared.percent(), Some(25));
	}

	#[test]
	fn unknown_total_pulses() {
		let progress = progress();
		progress.set(25, Some(100));
		progress.set(30, None);
		assert_eq!(progress.shared.percent(), None);
	}

	#[test]
	fn zero_total_pulses() {
		let progress = progress();
		progress.set(0, Some(0));
		assert_eq!(progress.shared.percent(), None);
	}

	#[test]
	fn latest_message_reaches_the_next_update_once() {
		let progress = progress();
		assert_eq!(progress.shared.take_message(), None);
		progress.set_message("first");
		progress.set_message("second");
		assert_eq!(progress.shared.take_message().as_deref(), Some("second"));
		assert_eq!(progress.shared.take_message(), None);
	}

	#[test]
	fn cancel_reaches_the_worker() {
		let progress = progress();
		assert!(!progress.is_cancelled());
		progress.shared.cancel();
		assert!(progress.is_cancelled());
		assert!(progress.cancel_flag().load(Ordering::Relaxed));
	}

	#[test]
	fn run_stops_running_when_cancelled() {
		let shared = Shared::default();
		assert!(shared.is_running());
		shared.cancel();
		assert!(!shared.is_running());
	}

	#[test]
	fn run_stops_running_when_finished() {
		let shared = Shared::default();
		shared.finish();
		assert!(!shared.is_running());
	}

	#[test]
	fn answer_from_the_ui_thread_reaches_the_worker() {
		let progress = progress();
		let answer = progress.ask_with(
			|_| 42,
			|question| {
				thread::spawn(move || question.show(&NoWindow));
			},
		);
		assert_eq!(answer, Some(42));
	}

	#[test]
	fn question_dropped_without_an_answer_gives_none() {
		let progress = progress();
		assert_eq!(progress.ask_with(|_| 42, drop), None);
	}

	#[test]
	fn cancelled_run_posts_no_question() {
		let progress = progress();
		progress.shared.cancel();
		let answer = progress.ask_with(|_| 42, |_| panic!("question posted after Cancel"));
		assert_eq!(answer, None);
	}
}

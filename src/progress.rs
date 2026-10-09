//! A progress window for work that runs on a worker thread.
//!
//! [`run_with_progress`] shows the window, runs the work and reports back on the UI thread. One
//! window shows at a time: a run started while another shows waits for it.
use std::{
	cell::RefCell,
	ptr,
	sync::{Arc, Mutex, PoisonError},
	thread,
	time::Duration,
};

use wxdragon::{ffi, prelude::*, window::WxWidget};

mod ending;
mod handle;
mod lifecycle;
mod queue;

use ending::Ending;
pub use handle::Progress;
use handle::{Question, Shared};
use lifecycle::ProgressLifecycle;
use queue::RunQueue;

const UPDATE_INTERVAL: Duration = Duration::from_millis(200);

type Start = Box<dyn FnOnce()>;
type OnEnd = Box<dyn FnOnce(Ended)>;

thread_local! {
	static DIALOG: ProgressLifecycle<ProgressDialog> = const { ProgressLifecycle::new() };
	static RUNS: RefCell<RunQueue<Start>> = const { RefCell::new(RunQueue::new()) };
	static ON_END: RefCell<Option<OnEnd>> = const { RefCell::new(None) };
	static ENDING: RefCell<Ending> = const { RefCell::new(Ending::new()) };
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
	/// The work returned and the user did not press Cancel.
	Completed,
	/// The user pressed Cancel. The work still returned its result, which may be partial.
	Cancelled,
}

/// Runs `work` on a worker thread behind a progress window, then calls `done` on the UI thread.
///
/// The window has a gauge, `message` and a Cancel button, and is modal for the whole application.
/// `work` reports through its [`Progress`]. `done` gets the result of `work` and how the run
/// ended, once the window is gone and the application's windows are enabled again, so it can open
/// windows and move focus. After Cancel the window closes at once, but `done` waits until `work`
/// returns. If `work` panics, the window closes and `done` is not called.
///
/// Call it on the UI thread. While another run's window shows, this run waits, and starts after
/// the other run's `done` has returned. `title` and `message` are shown as given, so pass
/// translated text.
///
/// ```no_run
/// use wx_utils::progress::{Ended, run_with_progress};
/// use wxdragon::prelude::*;
///
/// fn count(parent: &Frame) {
///     run_with_progress(
///         parent,
///         "Counting",
///         "Counting to a million...",
///         |progress| {
///             for n in 1..=1_000_000 {
///                 if progress.is_cancelled() {
///                     return n;
///                 }
///                 progress.set(n, Some(1_000_000));
///             }
///             1_000_000
///         },
///         |reached, ended| {
///             if ended == Ended::Cancelled {
///                 println!("Stopped at {reached}");
///             }
///         },
///     );
/// }
/// ```
pub fn run_with_progress<T: Send + 'static>(
	parent: &dyn WxWidget,
	title: &str,
	message: &str,
	work: impl FnOnce(&Progress) -> T + Send + 'static,
	done: impl FnOnce(T, Ended) + 'static,
) {
	let parent = ParentWindow::new(parent);
	let (title, message) = (title.to_owned(), message.to_owned());
	let start: Start = Box::new(move || begin(parent, &title, &message, work, done));
	if let Some(start) = RUNS.with_borrow_mut(|runs| runs.submit(start)) {
		start();
	}
}

fn begin<T: Send + 'static>(
	parent: ParentWindow,
	title: &str,
	message: &str,
	work: impl FnOnce(&Progress) -> T + Send + 'static,
	done: impl FnOnce(T, Ended) + 'static,
) {
	let dialog = ProgressDialog::builder(&parent, title, message, 100)
		.with_style(
			ProgressDialogStyle::AutoHide
				| ProgressDialogStyle::AppModal
				| ProgressDialogStyle::RemainingTime
				| ProgressDialogStyle::CanAbort,
		)
		.build();
	ENDING.set(Ending::new());
	notice_destruction(&dialog);
	DIALOG.with(|lifecycle| lifecycle.set(dialog));
	let shared = Arc::new(Shared::default());
	let result = Arc::new(Mutex::new(None));
	let worker_result = Arc::clone(&result);
	ON_END.set(Some(Box::new(move |ended| {
		let result = result.lock().unwrap_or_else(PoisonError::into_inner).take();
		if let Some(result) = result {
			done(result, ended);
		}
	})));
	spawn_heartbeat(Arc::clone(&shared));
	thread::spawn(move || {
		let _end = EndOnDrop(Arc::clone(&shared));
		let value = work(&Progress::new(shared));
		*worker_result.lock().unwrap_or_else(PoisonError::into_inner) = Some(value);
	});
}

/// Ends the run on the UI thread when the worker is done with it, by returning or by panicking.
struct EndOnDrop(Arc<Shared>);

impl Drop for EndOnDrop {
	fn drop(&mut self) {
		self.0.finish();
		let shared = Arc::clone(&self.0);
		post_to_ui(move || end_run(shared));
	}
}

fn end_run(shared: Arc<Shared>) {
	DIALOG.with(|lifecycle| {
		lifecycle.finish(Box::new(move || {
			DIALOG.with(ProgressLifecycle::clear);
			let ended = if shared.is_cancelled() { Ended::Cancelled } else { Ended::Completed };
			if let Some(ended) = ENDING.with_borrow_mut(|ending| ending.work_returned(ended)) {
				finish_run(ended);
			}
		}));
	});
}

/// Ends the run once `dialog` has been destroyed, which wx does some time after it is closed. A
/// dialog that was never created counts as destroyed already.
fn notice_destruction(dialog: &ProgressDialog) {
	let dialog_ptr = dialog.handle_ptr();
	if dialog_ptr.is_null() {
		ENDING.with_borrow_mut(Ending::window_destroyed);
		return;
	}
	// SAFETY: `dialog_ptr` is the progress dialog just built, which is a wxDialog. The handle is
	// only used to bind an event and does not own the dialog.
	let handler = unsafe { Dialog::from_ptr(dialog_ptr.cast()) };
	handler.bind_internal(EventType::DESTROY, move |event| {
		// Destroy events from the dialog's own children reach it too.
		if event.get_event_object().is_some_and(|object| object.as_ptr() == dialog_ptr) {
			post_to_ui(window_destroyed);
		}
		event.skip(true);
	});
}

fn window_destroyed() {
	if let Some(ended) = ENDING.with_borrow_mut(Ending::window_destroyed) {
		finish_run(ended);
	}
}

fn finish_run(ended: Ended) {
	if let Some(on_end) = ON_END.take() {
		on_end(ended);
	}
	post_to_ui(start_next);
}

fn start_next() {
	if let Some(start) = RUNS.with_borrow_mut(RunQueue::finish) {
		start();
	}
}

/// Updates the window from the UI thread every [`UPDATE_INTERVAL`] until the run stops running.
fn spawn_heartbeat(shared: Arc<Shared>) {
	thread::spawn(move || {
		while shared.is_running() {
			let tick = Arc::clone(&shared);
			post_to_ui(move || refresh(&tick));
			thread::sleep(UPDATE_INTERVAL);
		}
	});
}

fn refresh(shared: &Shared) {
	if !shared.is_running() {
		return;
	}
	DIALOG.with(|lifecycle| {
		let Some(update) = lifecycle.begin_update() else {
			return;
		};
		let dialog = update.dialog();
		let message = shared.take_message();
		let keep_going = shared
			.percent()
			.map_or_else(|| dialog.pulse(message.as_deref()), |percent| dialog.update(percent, message.as_deref()));
		if !keep_going {
			shared.cancel();
			lifecycle.clear();
		}
		// Dropping `update` invokes any completion queued during the native event-loop yield,
		// after Update/Pulse returns and its reference to the dialog is released.
	});
}

fn post_question(shared: Arc<Shared>, question: Question) {
	post_to_ui(move || show_question(shared, question));
}

/// Shows `question` over the progress window, holding the window like an update does, so that
/// no update runs meanwhile. While an update runs, posts the question again.
fn show_question(shared: Arc<Shared>, question: Question) {
	if shared.is_cancelled() {
		return;
	}
	DIALOG.with(|lifecycle| match lifecycle.begin_update() {
		Some(update) => question.show(update.dialog()),
		None => post_question(shared, question),
	});
}

fn post_to_ui(callback: impl FnOnce() + Send + 'static) {
	wxdragon::call_after(Box::new(callback));
	wxdragon::wake_up_idle();
}

/// A window handle that can cross threads. It is only turned back into a pointer on the main
/// thread, inside `call_after` callbacks.
#[derive(Clone, Copy)]
struct ParentWindow(usize);

impl ParentWindow {
	fn new(window: &dyn WxWidget) -> Self {
		Self(window.handle_ptr().expose_provenance())
	}
}

impl WxWidget for ParentWindow {
	fn handle_ptr(&self) -> *mut ffi::wxd_Window_t {
		ptr::with_exposed_provenance_mut(self.0)
	}
}

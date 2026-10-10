//! Fake downloads behind `run_with_progress`, for trying the progress window by hand.
//!
//! "Download" connects (the bar pulses), asks whether to go on, then counts to 50 with the bar
//! moving. "Download twice" starts two runs at once; the second waits for the first. Each run
//! logs how it ended and moves focus to the log.
use std::{thread, time::Duration};

use wx_utils::{
	confirm,
	progress::{Progress, run_with_progress},
};
use wxdragon::prelude::*;

const STEPS: u64 = 50;

fn fake_download(progress: &Progress) -> String {
	progress.set_message("Connecting...");
	for _ in 0..10 {
		if progress.is_cancelled() {
			return "stopped while connecting".to_owned();
		}
		progress.set(0, None);
		thread::sleep(Duration::from_millis(100));
	}
	let go_on = progress.ask(|parent| confirm(parent, "The file is 50 MB. Download it anyway?", "Large file"));
	if go_on != Some(true) {
		return "declined".to_owned();
	}
	progress.set_message("Downloading...");
	for step in 1..=STEPS {
		if progress.is_cancelled() {
			return format!("stopped at {step} of {STEPS}");
		}
		progress.set(step, Some(STEPS));
		thread::sleep(Duration::from_millis(100));
	}
	format!("downloaded {STEPS} of {STEPS}")
}

fn download(parent: &Frame, log: TextCtrl, name: &str) {
	let name = name.to_owned();
	run_with_progress(parent, &format!("Downloading {name}"), "Starting...", fake_download, move |outcome, ended| {
		log.append_text(&format!("{name}: {outcome} ({ended:?})\n"));
		log.set_focus();
	});
}

fn main() {
	let _ = wxdragon::main(|_| {
		let frame = Frame::builder().with_title("Progress example").build();
		let panel = Panel::builder(&frame).build();
		let once = Button::builder(&panel).with_label("&Download").build();
		let twice = Button::builder(&panel).with_label("Download &twice").build();
		let log = TextCtrl::builder(&panel)
			.with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly)
			.with_size(Size::new(400, 200))
			.build();
		once.on_click(move |_| download(&frame, log, "one.epub"));
		twice.on_click(move |_| {
			download(&frame, log, "first.epub");
			download(&frame, log, "second.epub");
		});
		let sizer = BoxSizer::builder(Orientation::Vertical).build();
		sizer.add(&once, 0, SizerFlag::All, 10);
		sizer.add(&twice, 0, SizerFlag::All, 10);
		sizer.add(&log, 1, SizerFlag::Expand | SizerFlag::All, 10);
		panel.set_sizer(sizer, true);
		frame.set_size(Size::new(450, 350));
		frame.show(true);
	});
}

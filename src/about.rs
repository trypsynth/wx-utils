use std::ffi::CString;

use wxdragon::{ffi, prelude::*};

pub struct AboutBoxBuilder<'a> {
	parent: &'a Frame,
	name: Option<String>,
	version: Option<String>,
	description: Option<String>,
	copyright: Option<String>,
	website: Option<String>,
	developers: Vec<String>,
}

impl<'a> AboutBoxBuilder<'a> {
	#[must_use] 
	pub const fn new(parent: &'a Frame) -> Self {
		Self {
			parent,
			name: None,
			version: None,
			description: None,
			copyright: None,
			website: None,
			developers: Vec::new(),
		}
	}

	#[must_use]
	pub fn name(mut self, name: impl Into<String>) -> Self {
		self.name = Some(name.into());
		self
	}

	#[must_use]
	pub fn version(mut self, version: impl Into<String>) -> Self {
		self.version = Some(version.into());
		self
	}

	#[must_use]
	pub fn description(mut self, desc: impl Into<String>) -> Self {
		self.description = Some(desc.into());
		self
	}

	#[must_use]
	pub fn copyright(mut self, copyright: impl Into<String>) -> Self {
		self.copyright = Some(copyright.into());
		self
	}

	#[must_use]
	pub fn website(mut self, url: impl Into<String>) -> Self {
		self.website = Some(url.into());
		self
	}

	#[must_use]
	pub fn add_developer(mut self, dev: impl Into<String>) -> Self {
		self.developers.push(dev.into());
		self
	}

	pub fn show(self) {
		unsafe {
			let info = ffi::wxd_AboutDialogInfo_Create();
			if info.is_null() {
				return;
			}

			// A quick internal macro to handle the CString conversion and FFI call
			macro_rules! apply_str {
				($val:expr, $f:path) => {
					if let Some(s) = $val {
						if let Ok(cs) = CString::new(s) {
							$f(info, cs.as_ptr());
						}
					}
				};
			}

			apply_str!(self.name, ffi::wxd_AboutDialogInfo_SetName);
			apply_str!(self.version, ffi::wxd_AboutDialogInfo_SetVersion);
			apply_str!(self.description, ffi::wxd_AboutDialogInfo_SetDescription);
			apply_str!(self.copyright, ffi::wxd_AboutDialogInfo_SetCopyright);
			apply_str!(self.website, ffi::wxd_AboutDialogInfo_SetWebSite);

			for dev in self.developers {
				if let Ok(cs) = CString::new(dev) {
					ffi::wxd_AboutDialogInfo_AddDeveloper(info, cs.as_ptr());
				}
			}

			ffi::wxd_AboutBox(info, self.parent.handle_ptr());
			ffi::wxd_AboutDialogInfo_Destroy(info);
		}
	}
}

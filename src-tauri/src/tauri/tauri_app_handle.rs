use std::{fmt::Debug, sync::OnceLock};

use tauri::AppHandle;

pub trait TauriAppHandle: Debug {
    fn set_app_handle(&self, app_handle: AppHandle);
}

#[derive(Debug, Default)]
pub(crate) struct ImplTauriAppHandle {
    app_handle: OnceLock<AppHandle>,
}

impl ImplTauriAppHandle {
    fn app_handle(&self) -> &AppHandle {
        self.app_handle
            .get()
            .expect("`app_handle` is not initialized.")
    }
}

impl TauriAppHandle for ImplTauriAppHandle {
    fn set_app_handle(&self, app_handle: AppHandle) {
        self.app_handle
            .set(app_handle)
            .expect("`app_handle` contains already a value.");
    }
}

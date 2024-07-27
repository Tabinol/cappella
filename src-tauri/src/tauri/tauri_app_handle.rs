use std::fmt::Debug;

use tauri::AppHandle;

pub trait TauriAppHandle: Debug {}

#[derive(Debug)]
pub(crate) struct ImplTauriAppHandle {
    app_handle: AppHandle,
}

impl ImplTauriAppHandle {
    pub(crate) fn new(app_handle: AppHandle) -> ImplTauriAppHandle {
        Self { app_handle }
    }
}

impl TauriAppHandle for ImplTauriAppHandle {}

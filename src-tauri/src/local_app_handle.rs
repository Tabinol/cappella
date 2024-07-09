use std::fmt::Debug;

use dyn_clone::DynClone;
use tauri::AppHandle;

pub(crate) trait LocalAppHandle: Debug + DynClone + Send + Sync {}

dyn_clone::clone_trait_object!(LocalAppHandle);

#[derive(Clone, Debug)]
pub(crate) struct ImplLocalAppHandle {
    app_handle: AppHandle,
}

impl ImplLocalAppHandle {
    pub(crate) fn new(app_handle: AppHandle) -> Box<dyn LocalAppHandle> {
        Box::new(Self { app_handle })
    }
}

impl LocalAppHandle for ImplLocalAppHandle {}

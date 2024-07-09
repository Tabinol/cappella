use std::fmt::Debug;

use dyn_clone::DynClone;
use tauri::AppHandle;

pub trait TauriAppHandle: Debug + DynClone + Send + Sync {}

dyn_clone::clone_trait_object!(TauriAppHandle);

#[derive(Clone, Debug)]
pub(crate) struct ImplTauriAppHandle {
    app_handle: AppHandle,
}

impl ImplTauriAppHandle {
    pub(crate) fn new(app_handle: AppHandle) -> Box<dyn TauriAppHandle> {
        Box::new(Self { app_handle })
    }
}

impl TauriAppHandle for ImplTauriAppHandle {}

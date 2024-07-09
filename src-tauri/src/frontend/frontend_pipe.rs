use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::tauri::tauri_app_handle::TauriAppHandle;

const PLAYER_EVENT_NAME: &str = "PLAYER_EVENT";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) enum Message {
    None,
}

pub(crate) trait FrontendPipe: Debug + DynClone + Send + Sync {
    fn send(&self, message: Message);
}

dyn_clone::clone_trait_object!(FrontendPipe);

#[derive(Clone, Debug)]
pub(crate) struct ImplFrontendPipe {
    tauri_app_handle: Box<dyn TauriAppHandle>,
}

impl ImplFrontendPipe {
    pub(crate) fn new(tauri_app_handle: Box<dyn TauriAppHandle>) -> Box<dyn FrontendPipe> {
        Box::new(Self { tauri_app_handle })
    }
}

impl FrontendPipe for ImplFrontendPipe {
    fn send(&self, message: Message) {
        //(*self.app_handle).emit_to(MAIN_WINDOW_LABEL, event, payload);
    }
}

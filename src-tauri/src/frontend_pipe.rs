use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::local_app_handle::LocalAppHandle;

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
    local_app_handle: Box<dyn LocalAppHandle>,
}

impl ImplFrontendPipe {
    pub(crate) fn new(local_app_handle: Box<dyn LocalAppHandle>) -> Box<dyn FrontendPipe> {
        Box::new(Self { local_app_handle })
    }
}

impl FrontendPipe for ImplFrontendPipe {
    fn send(&self, message: Message) {
        //(*self.app_handle).emit_to(MAIN_WINDOW_LABEL, event, payload);
    }
}

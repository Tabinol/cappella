use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use dyn_clone::DynClone;

#[cfg(not(test))]
use tauri::AppHandle;

#[cfg(test)]
use tests::MockAppHandle as AppHandle;

const PLAYER_EVENT_NAME: &str = "PLAYER_EVENT";

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub(crate) enum Message {
    #[default]
    None,
}

pub(crate) trait FrontendPipe: Debug + DynClone + Send + Sync {
    fn set_app_handle(&self, app_handle: &tauri::AppHandle);
    fn send(&self, message: Message);
}

dyn_clone::clone_trait_object!(FrontendPipe);

pub(crate) fn new_boxed() -> Box<dyn FrontendPipe> {
    Box::<FrontendPipe_>::default()
}

#[derive(Clone, Debug, Default)]
struct FrontendPipe_ {
    app_handle: Arc<RwLock<Option<AppHandle>>>,
}

unsafe impl Send for FrontendPipe_ {}
unsafe impl Sync for FrontendPipe_ {}

impl FrontendPipe_ {
    fn set_app_handle(&self, app_handle: &AppHandle) {
        let mut app_handle_lock = self
            .app_handle
            .try_write()
            .expect("Unable to write the app handle in the frontend pipe. Is it locked?");

        if app_handle_lock.is_some() {
            eprintln!("App handle already wrote in the frontend pipe.");
            return;
        }

        *app_handle_lock = Some(app_handle.clone());
    }
}

impl FrontendPipe for FrontendPipe_ {
    fn set_app_handle(&self, app_handle: &tauri::AppHandle) {
        #[cfg(not(test))]
        {
            FrontendPipe_::set_app_handle(&self, app_handle);
        }

        #[cfg(test)]
        {
            panic!(
                "Unable to use this method in test. app_handle={:?}",
                app_handle
            )
        }
    }

    fn send(&self, message: Message) {
        //(*self.app_handle).emit_to(MAIN_WINDOW_LABEL, event, payload);
    }
}

#[cfg(test)]
mod tests {

    #[derive(Clone, Debug, Default)]
    pub(super) struct MockAppHandle {}

    impl MockAppHandle {}
}

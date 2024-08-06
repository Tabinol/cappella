use std::{fmt::Debug, sync::Arc};

use crate::{tauri::tauri_app_handle::TauriAppHandle, MAIN_WINDOW_LABEL};

const PLAYER_EVENT_NAME: &str = "PLAYER_EVENT";

#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub(crate) enum Message {
    #[default]
    None,
    Temp, // TODO Remove
}

pub(crate) trait FrontendPipe: Debug + Send + Sync {
    fn send(&self, message: Message);
}

pub(crate) fn new_arc(app_handle: Box<dyn TauriAppHandle>) -> Arc<dyn FrontendPipe> {
    Arc::new(FrontendPipe_::new(app_handle))
}

#[derive(Debug)]
struct FrontendPipe_ {
    app_handle: Box<dyn TauriAppHandle>,
}

unsafe impl Send for FrontendPipe_ {}
unsafe impl Sync for FrontendPipe_ {}

impl FrontendPipe_ {
    fn new(app_handle: Box<dyn TauriAppHandle>) -> Self {
        Self { app_handle }
    }
}

impl FrontendPipe for FrontendPipe_ {
    fn send(&self, message: Message) {
        if self
            .app_handle
            .emit_to(MAIN_WINDOW_LABEL, PLAYER_EVENT_NAME, message.clone())
            .is_err()
        {
            eprintln!("Unable to send to message to the frontend: {message:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use crate::{
        frontend::frontend_pipe::PLAYER_EVENT_NAME, tauri::tauri_app_handle::TauriAppHandle,
    };

    use super::{FrontendPipe, FrontendPipe_, Message};

    #[derive(Debug, Default)]
    struct AppHandleData {
        window_label: String,
        event: String,
        message: Message,
    }

    #[derive(Debug, Default)]
    struct MockAppHandle {
        data: Arc<RwLock<AppHandleData>>,
    }

    impl MockAppHandle {}

    impl TauriAppHandle for MockAppHandle {
        fn emit_to(
            &self,
            window_label: &str,
            event: &str,
            message: super::Message,
        ) -> Result<(), tauri::Error> {
            if matches!(message, Message::None) {
                return Err(tauri::Error::FailedToReceiveMessage);
            }

            let mut data = self.data.try_write().unwrap();
            (*data).window_label = window_label.to_owned();
            (*data).event = event.to_owned();
            (*data).message = message.to_owned();

            Ok(())
        }
    }

    #[test]
    fn test_send_ok() {
        let app_handle_data = Arc::new(RwLock::new(AppHandleData::default()));
        let app_handle = Box::new(MockAppHandle {
            data: app_handle_data.clone(),
        });
        let frontend_pipe = FrontendPipe_::new(app_handle);

        frontend_pipe.send(Message::Temp);

        let data = app_handle_data.read().unwrap();

        assert_eq!(data.window_label, "main");
        assert_eq!(data.event, PLAYER_EVENT_NAME);
        assert!(matches!(data.message, Message::Temp));
    }

    #[test]
    fn test_send_error() {
        let app_handle_data = Arc::new(RwLock::new(AppHandleData::default()));
        let app_handle = Box::new(MockAppHandle {
            data: app_handle_data.clone(),
        });
        let frontend_pipe = FrontendPipe_::new(app_handle);

        frontend_pipe.send(Message::None);

        let data = app_handle_data.read().unwrap();

        assert_eq!(data.window_label, String::default());
        assert_eq!(data.event, String::default());
        assert!(matches!(data.message, Message::None));
    }
}

pub mod bus;
pub mod element;
pub mod message;
pub mod state;
pub mod structure;
pub mod structure_field;

#[cfg(test)]
mod common_tests {
    use std::{
        collections::{hash_map::Entry, HashMap},
        ffi::{c_char, c_int, CStr},
        ptr::{self, null_mut},
        sync::{
            atomic::{AtomicI64, Ordering},
            Arc, Mutex, OnceLock,
        },
    };

    use glib_sys::{gboolean, GError, GFALSE};
    use gstreamer_sys::{
        GstBus, GstClockTime, GstElement, GstMessage, GstMessageType, GstObject, GstState,
        GstStateChangeReturn, GST_STATE_CHANGE_SUCCESS, GST_STATE_NULL,
    };

    use crate::local::mutex_lock_timeout::{MutexLockTimeout, LOCK_STANDARD_TIMEOUT_DURATION};

    pub const UNASSIGNED: i64 = -1;

    static TEST_COUNTER: AtomicI64 = AtomicI64::new(0);
    static TEST_NB_TO_TEST_STRUCTURE: OnceLock<Mutex<HashMap<i64, Arc<Mutex<TestStructure>>>>> =
        OnceLock::new();

    #[derive(Debug, Eq, Hash, PartialEq)]
    pub enum ObjectType {
        GstBus,
        GstElement,
        GstMesage,
    }

    #[derive(Debug)]
    pub struct Object {
        test_structure: Arc<Mutex<TestStructure>>,
        is_unref: bool,
    }

    pub trait RcRefCellTestStructure {
        fn test_nb(&self) -> i64;
        fn faked_gst_bus(&self) -> *mut GstBus;
        fn faked_gst_element(&self) -> *mut GstElement;
        fn faked_gst_message(&self) -> *mut GstMessage;
        fn element_state(&self) -> GstState;
        fn set_gst_bus_post_return(&self, value: gboolean);
        fn set_pop_message(&self, value: bool);
        fn is_unref(&self, object_type: ObjectType) -> bool;
    }

    #[derive(Debug)]
    pub struct TestStructure {
        test_nb: i64,
        type_to_object: HashMap<ObjectType, Object>,
        element_state: GstState,
        gst_bus_post_return: gboolean,
        pop_message: bool,
    }

    impl TestStructure {
        pub fn new_arc_mutex(test_nb: i64) -> Arc<Mutex<TestStructure>> {
            Arc::new(Mutex::new(Self {
                test_nb,
                type_to_object: HashMap::new(),
                element_state: GST_STATE_NULL,
                gst_bus_post_return: GFALSE,
                pop_message: false,
            }))
        }

        pub fn new_arc_mutex_assigned() -> Arc<Mutex<TestStructure>> {
            let test_nb = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
            let self_arc_mutex = Self::new_arc_mutex(test_nb);
            let test_nb_to_test_structure_mutex =
                TEST_NB_TO_TEST_STRUCTURE.get_or_init(|| Mutex::new(HashMap::new()));
            let mut test_nb_to_test_structure_lock = test_nb_to_test_structure_mutex
                .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap();
            test_nb_to_test_structure_lock.insert(test_nb, Arc::clone(&self_arc_mutex));

            self_arc_mutex
        }

        pub fn from_test_nb(test_nb: i64) -> Arc<Mutex<TestStructure>> {
            TEST_NB_TO_TEST_STRUCTURE
                .get()
                .expect("No test structure created.")
                .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap()
                .get(&test_nb)
                .expect("the test structure is not created for this test number.")
                .clone()
        }
    }

    impl RcRefCellTestStructure for Arc<Mutex<TestStructure>> {
        fn test_nb(&self) -> i64 {
            self.try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap()
                .test_nb
        }

        fn faked_gst_bus(&self) -> *mut GstBus {
            let mut self_lock = self
                .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap();
            let object = self_lock
                .type_to_object
                .entry(ObjectType::GstBus)
                .or_insert(Object {
                    test_structure: Arc::clone(self),
                    is_unref: false,
                });

            ptr::addr_of_mut!(*object) as *mut GstBus
        }

        fn faked_gst_element(&self) -> *mut GstElement {
            let mut self_lock = self
                .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap();
            let object = self_lock
                .type_to_object
                .entry(ObjectType::GstElement)
                .or_insert(Object {
                    test_structure: Arc::clone(self),
                    is_unref: false,
                });

            ptr::addr_of_mut!(*object) as *mut GstElement
        }

        fn faked_gst_message(&self) -> *mut GstMessage {
            let mut self_lock = self
                .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap();
            let object = self_lock
                .type_to_object
                .entry(ObjectType::GstMesage)
                .or_insert(Object {
                    test_structure: Arc::clone(self),
                    is_unref: false,
                });

            ptr::addr_of_mut!(*object) as *mut GstMessage
        }

        fn element_state(&self) -> GstState {
            self.try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap()
                .element_state
        }

        fn set_gst_bus_post_return(&self, value: gboolean) {
            self.try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap()
                .gst_bus_post_return = value;
        }

        fn set_pop_message(&self, value: bool) {
            self.try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap()
                .pop_message = value;
        }

        fn is_unref(&self, object_type: ObjectType) -> bool {
            match self
                .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap()
                .type_to_object
                .entry(object_type)
            {
                Entry::Occupied(object) => object.get().is_unref,
                Entry::Vacant(_) => false,
            }
        }
    }

    #[no_mangle]
    pub extern "C" fn gst_bus_post(bus: *mut GstBus, message: *mut GstMessage) -> gboolean {
        assert!(!bus.is_null());
        assert!(!message.is_null());

        let object = unsafe { &*(bus as *mut Object) };

        object
            .test_structure
            .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
            .unwrap()
            .gst_bus_post_return
    }

    #[no_mangle]
    pub extern "C" fn gst_bus_timed_pop_filtered(
        bus: *mut GstBus,
        _timeout: GstClockTime,
        _types: GstMessageType,
    ) -> *mut GstMessage {
        assert!(!bus.is_null());

        let object = unsafe { &*(bus as *mut Object) };
        let test_structure = object.test_structure.clone();

        assert!(
            test_structure
                .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
                .unwrap()
                .type_to_object
                .get(&ObjectType::GstMesage)
                .is_none(),
            "Gst Message already inserted."
        );

        if test_structure
            .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
            .unwrap()
            .pop_message
        {
            return test_structure.faked_gst_message();
        }

        null_mut()
    }

    #[no_mangle]
    pub extern "C" fn gst_element_get_bus(element: *mut GstElement) -> *mut GstBus {
        assert!(!element.is_null());

        let object = unsafe { &*(element as *mut Object) };
        let test_structure = object.test_structure.clone();

        if test_structure.test_nb() == UNASSIGNED {
            return null_mut();
        }

        test_structure.faked_gst_bus()
    }

    #[no_mangle]
    pub extern "C" fn gst_element_set_state(
        element: *mut GstElement,
        state: GstState,
    ) -> GstStateChangeReturn {
        let object_ref = unsafe { &mut *(element as *mut Object) };
        object_ref
            .test_structure
            .try_lock_timeout(LOCK_STANDARD_TIMEOUT_DURATION)
            .unwrap()
            .element_state = state;

        GST_STATE_CHANGE_SUCCESS
    }

    #[no_mangle]
    pub extern "C" fn gst_init(_argc: *mut c_int, _argv: *mut *mut *mut c_char) {}

    #[no_mangle]
    pub extern "C" fn gst_message_unref(msg: *mut GstMessage) {
        assert!(!msg.is_null());

        let object_ref = unsafe { &mut *(msg as *mut Object) };
        assert!(!object_ref.is_unref);
        object_ref.is_unref = true;
    }

    #[no_mangle]
    pub extern "C" fn gst_object_unref(object: *mut GstObject) {
        assert!(!object.is_null());

        let object_ref = unsafe { &mut *(object as *mut Object) };
        assert!(!object_ref.is_unref);
        object_ref.is_unref = true;
    }

    #[no_mangle]
    pub extern "C" fn gst_parse_launch(
        pipeline_description: *const c_char,
        _error: *mut *mut GError,
    ) -> *mut GstElement {
        let pipeline_description_string = unsafe { CStr::from_ptr(pipeline_description) };

        let test_nb = pipeline_description_string
            .to_str()
            .unwrap()
            .to_owned()
            .replace("playbin uri=\"", "")
            .replace("\"", "")
            .parse::<i64>()
            .expect("The uri doesn't contain a test number.");

        let test_structure = if test_nb != UNASSIGNED {
            TestStructure::from_test_nb(test_nb)
        } else {
            TestStructure::new_arc_mutex(UNASSIGNED)
        };

        test_structure.faked_gst_element()
    }
}

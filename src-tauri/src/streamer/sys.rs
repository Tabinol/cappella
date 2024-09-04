pub mod bus;
pub mod element;
pub mod message;
pub mod state;
pub mod structure;
pub mod structure_field;

#[cfg(test)]
mod common_tests {
    use std::{
        collections::{HashMap, HashSet},
        ffi::{c_char, c_int, CStr, CString},
        ptr::{self, null_mut},
        sync::{
            atomic::{AtomicI64, Ordering},
            Arc, OnceLock,
        },
    };

    use glib_sys::{gboolean, GError, GFALSE};
    use gstreamer_sys::{
        GstBus, GstClockTime, GstElement, GstMessage, GstMessageType, GstObject, GstState,
        GstStateChangeReturn, GstStructure, GST_STATE_CHANGE_SUCCESS, GST_STATE_NULL,
        GST_STATE_PAUSED, GST_STATE_PLAYING,
    };
    use parking_lot::{Mutex, MutexGuard};

    use crate::local::mutex_lock_timeout::MutexLockTimeout;

    pub const STRUCTURE_NAME: &str = "STRUCTURE_NAME";
    pub const UNASSIGNED: i64 = -1;

    static TEST_COUNTER: AtomicI64 = AtomicI64::new(0);
    static TEST_NB_TO_TEST_STRUCTURE: OnceLock<Mutex<HashMap<i64, Arc<Mutex<TestStructure>>>>> =
        OnceLock::new();

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    pub enum TestObjectType {
        GstBus,
        GstElement,
        GstMessage,
        GstStructure,
    }

    #[derive(Clone, Debug)]
    pub struct TestObject {
        test_object_type: TestObjectType,
        test_structure: Arc<Mutex<TestStructure>>,
    }

    impl TestObject {
        pub fn from_raw_ptr(raw_ptr: *const Self) -> Self {
            assert!(!raw_ptr.is_null());

            unsafe { (*raw_ptr).clone() }
        }
    }

    pub trait RcRefCellTestStructure {
        fn test_nb(&self) -> i64;
        fn faked_gst_bus(&self) -> *mut GstBus;
        fn faked_gst_element(&self) -> *mut GstElement;
        fn faked_gst_message(&self) -> *mut GstMessage;
        fn faked_gst_structure(&self) -> *mut GstStructure;
        fn element_state(&self) -> GstState;
        fn set_gst_bus_post_return(&self, value: gboolean);
        fn set_pop_message(&self, value: bool);
        fn is_unref(&self, test_object_type: TestObjectType) -> bool;
        fn try_lock_unwrap(&self) -> MutexGuard<TestStructure>;
    }

    #[derive(Debug)]
    pub struct TestStructure {
        test_nb: i64,
        type_to_object: HashMap<TestObjectType, TestObject>,
        c_strings: Vec<CString>,
        unrefs: HashSet<TestObjectType>,
        element_state: GstState,
        gst_bus_post_return: gboolean,
        pop_message: bool,
    }

    impl TestStructure {
        pub fn new_arc_mutex(test_nb: i64) -> Arc<Mutex<TestStructure>> {
            Arc::new(Mutex::new(Self {
                test_nb,
                type_to_object: HashMap::new(),
                c_strings: Vec::new(),
                unrefs: HashSet::new(),
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
                .try_lock_default_duration()
                .unwrap();
            test_nb_to_test_structure_lock.insert(test_nb, Arc::clone(&self_arc_mutex));

            self_arc_mutex
        }

        pub fn from_test_nb(test_nb: i64) -> Arc<Mutex<TestStructure>> {
            TEST_NB_TO_TEST_STRUCTURE
                .get()
                .expect("No test structure created.")
                .try_lock_default_duration()
                .unwrap()
                .get(&test_nb)
                .expect("the test structure is not created for this test number.")
                .clone()
        }

        pub fn from_raw_ptr(raw_ptr: *const TestObject) -> Arc<Mutex<TestStructure>> {
            assert!(!raw_ptr.is_null());

            TestObject::from_raw_ptr(raw_ptr).test_structure.clone()
        }

        fn faked_gst<G>(
            arc_mutex_self: &Arc<Mutex<Self>>,
            test_object_type: TestObjectType,
        ) -> *mut G {
            let mut self_lock = arc_mutex_self.try_lock_default_duration().unwrap();
            let test_object = self_lock
                .type_to_object
                .entry(test_object_type.clone())
                .or_insert(TestObject {
                    test_object_type: test_object_type.clone(),
                    test_structure: arc_mutex_self.clone(),
                });

            ptr::addr_of!(*test_object) as *mut G
        }
    }

    impl RcRefCellTestStructure for Arc<Mutex<TestStructure>> {
        fn test_nb(&self) -> i64 {
            self.try_lock_unwrap().test_nb
        }

        fn faked_gst_bus(&self) -> *mut GstBus {
            TestStructure::faked_gst(self, TestObjectType::GstBus)
        }

        fn faked_gst_element(&self) -> *mut GstElement {
            TestStructure::faked_gst(self, TestObjectType::GstElement)
        }

        fn faked_gst_message(&self) -> *mut GstMessage {
            TestStructure::faked_gst(self, TestObjectType::GstMessage)
        }

        fn faked_gst_structure(&self) -> *mut GstStructure {
            TestStructure::faked_gst(self, TestObjectType::GstStructure)
        }

        fn element_state(&self) -> GstState {
            self.try_lock_unwrap().element_state
        }

        fn set_gst_bus_post_return(&self, value: gboolean) {
            self.try_lock_unwrap().gst_bus_post_return = value;
        }

        fn set_pop_message(&self, value: bool) {
            self.try_lock_unwrap().pop_message = value;
        }

        fn is_unref(&self, test_object_type: TestObjectType) -> bool {
            self.try_lock_unwrap().unrefs.contains(&test_object_type)
        }

        fn try_lock_unwrap(&self) -> MutexGuard<TestStructure> {
            self.try_lock_default_duration().unwrap()
        }
    }

    #[no_mangle]
    pub extern "C" fn gst_bus_post(bus: *mut GstBus, message: *mut GstMessage) -> gboolean {
        assert!(!bus.is_null());
        assert!(!message.is_null());

        let test_structure = TestStructure::from_raw_ptr(bus as *const TestObject);

        let result = test_structure.try_lock_unwrap().gst_bus_post_return;

        result
    }

    #[no_mangle]
    pub extern "C" fn gst_bus_timed_pop_filtered(
        bus: *mut GstBus,
        _timeout: GstClockTime,
        _types: GstMessageType,
    ) -> *mut GstMessage {
        assert!(!bus.is_null());

        let test_structure = TestStructure::from_raw_ptr(bus as *const TestObject);

        if test_structure.try_lock_unwrap().pop_message {
            return test_structure.faked_gst_message();
        }

        null_mut()
    }

    #[no_mangle]
    pub extern "C" fn gst_element_get_bus(element: *mut GstElement) -> *mut GstBus {
        assert!(!element.is_null());

        let test_structure = TestStructure::from_raw_ptr(element as *const TestObject);

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
        let test_structure = TestStructure::from_raw_ptr(element as *const TestObject);
        test_structure.try_lock_unwrap().element_state = state;

        GST_STATE_CHANGE_SUCCESS
    }

    #[no_mangle]
    pub extern "C" fn gst_init(_argc: *mut c_int, _argv: *mut *mut *mut c_char) {}

    #[no_mangle]
    pub fn gst_message_parse_state_changed(
        message: *mut GstMessage,
        oldstate: *mut GstState,
        newstate: *mut GstState,
        pending: *mut GstState,
    ) {
        assert!(!message.is_null());

        unsafe {
            *oldstate = GST_STATE_PAUSED;
            *newstate = GST_STATE_PLAYING;
            *pending = GST_STATE_NULL;
        }
    }

    #[no_mangle]
    pub extern "C" fn gst_message_get_structure(message: *mut GstMessage) -> *const GstStructure {
        assert!(!message.is_null());

        let test_structure = TestStructure::from_raw_ptr(message as *const TestObject);

        if test_structure.test_nb() == UNASSIGNED {
            return null_mut();
        }

        test_structure.faked_gst_structure()
    }

    #[no_mangle]
    pub extern "C" fn gst_message_unref(msg: *mut GstMessage) {
        assert!(!msg.is_null());

        let test_structure = TestStructure::from_raw_ptr(msg as *const TestObject);
        assert!(
            test_structure
                .try_lock_unwrap()
                .unrefs
                .insert(TestObjectType::GstMessage),
            "`GstMessage` already unref."
        );
    }

    #[no_mangle]
    pub extern "C" fn gst_object_unref(object: *mut GstObject) {
        assert!(!object.is_null());

        let test_object = TestObject::from_raw_ptr(object as *const TestObject);
        let test_object_type = test_object.test_object_type.clone();
        let test_structure = test_object.test_structure.clone();
        let mut test_structure_lock = test_structure.try_lock_unwrap();

        assert!(
            test_structure_lock.unrefs.insert(test_object_type.clone()),
            "`{test_object_type:?}` already unref."
        );
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

    #[no_mangle]
    pub extern "C" fn gst_structure_get_name(structure: *const GstStructure) -> *const c_char {
        assert!(!structure.is_null());

        let test_object = TestObject::from_raw_ptr(structure as *const TestObject);
        let test_structure = test_object.test_structure.clone();
        let mut test_structure_lock = test_structure.try_lock_unwrap();

        let c_string_name = CString::new(STRUCTURE_NAME).unwrap();
        let c_string_name_ptr = c_string_name.as_ptr();
        test_structure_lock.c_strings.push(c_string_name);

        c_string_name_ptr
    }
}

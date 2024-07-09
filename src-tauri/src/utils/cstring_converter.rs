use std::ffi::{CStr, CString};

pub(crate) fn str_to_cstring(str: &str) -> CString {
    CString::new(str).unwrap()
}

pub(crate) fn string_to_cstring(string: String) -> CString {
    CString::new(string).unwrap()
}

pub(crate) unsafe fn cstring_ptr_to_str<'a>(ptr: *const i8) -> &'a str {
    CStr::from_ptr(ptr).to_str().unwrap()
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use crate::utils::cstring_converter::{cstring_ptr_to_str, str_to_cstring, string_to_cstring};

    #[test]
    fn test_str_to_cstring() {
        let str = "abcd";
        let cstring = str_to_cstring(str);

        assert_eq!(cstring, CString::new("abcd").unwrap());
    }

    #[test]
    fn test_string_to_cstring() {
        let str = "abcd".to_string();
        let cstring = string_to_cstring(str);

        assert_eq!(cstring, CString::new("abcd").unwrap());
    }

    #[test]
    fn test_cstring_ptr_to_str() {
        let cstring = CString::new("abcd").unwrap();
        let cstring_ptr = cstring.as_ptr();
        let str = unsafe { cstring_ptr_to_str(cstring_ptr) };

        assert_eq!(str, "abcd");
    }
}

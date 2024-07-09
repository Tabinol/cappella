use std::fmt::Debug;

#[derive(Clone, Debug)]
pub(crate) struct PointerConst<T>(*const T);

impl<T> PointerConst<T> {
    pub(crate) fn new(t: *const T) -> Self {
        Self(t)
    }

    pub(crate) fn get(&self) -> *const T {
        self.0
    }
}

unsafe impl<T> Send for PointerConst<T> {}
unsafe impl<T> Sync for PointerConst<T> {}

#[derive(Clone, Debug)]
pub(crate) struct PointerMut<T>(*mut T);

impl<T> PointerMut<T> {
    pub(crate) fn new(t: *mut T) -> Self {
        Self(t)
    }

    pub(crate) fn get(&self) -> *mut T {
        self.0
    }

    pub(crate) fn replace(&mut self, t: *mut T) {
        self.0 = t;
    }
}

unsafe impl<T> Send for PointerMut<T> {}
unsafe impl<T> Sync for PointerMut<T> {}

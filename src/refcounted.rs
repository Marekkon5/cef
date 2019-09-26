use std::{
    sync::atomic::{AtomicUsize, Ordering},
    os::raw::c_int,
    ops::{Deref, DerefMut},
    mem::ManuallyDrop,
};
use cef_sys::cef_base_ref_counted_t;

use crate::ptr_hash::Hashed;

pub(crate) trait RefCounter {
    type Wrapper;
    fn set_base(&mut self, base: cef_base_ref_counted_t);
}

// The code for RefCounted<C,R> assumes that it can cast *mut cef_base_ref_counted_t to *mut C to *mut RefCounted<C,R>
// this is true as long as everything is #[repr(C)] and the corresponding structs are the first in the list.
// It might sound like a hack, but I think that CEF assumes that you do it like this. It's a C API after all.
#[repr(C)]
pub(crate) struct RefCounted<C: RefCounter + Sized, R> {
    cefobj: C,
    refcount: AtomicUsize,
    object: R,
}

unsafe impl<C: RefCounter + Sized, R> Sync for RefCounted<C, R> {}
unsafe impl<C: RefCounter + Sized, R> Send for RefCounted<C, R> {}

impl<C: RefCounter + Sized, R> Deref for RefCounted<C, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        &self.object
    }
}

impl<C: RefCounter + Sized, R> DerefMut for RefCounted<C, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object
    }
}

impl<C: RefCounter + Sized, R> RefCounted<C, R> {
    pub(crate) unsafe fn make_temp(ptr: *mut C) -> ManuallyDrop<Box<Self>> {
        ManuallyDrop::new(unsafe { Box::from_raw(ptr as *mut Self) })
    }

    pub(crate) fn new(mut cefobj: C, object: R) -> *mut Self {
        cefobj.set_base(cef_base_ref_counted_t {
            size: std::mem::size_of::<C>(),
            add_ref: Some(Self::add_ref),
            release: Some(Self::release),
            has_one_ref: Some(Self::has_one_ref),
            has_at_least_one_ref: Some(Self::has_at_least_one_ref),
        });

        Box::into_raw(Box::new(Self {
            cefobj,
            refcount: AtomicUsize::new(1),
            object,
        }))
    }

    pub(crate) fn get_cef(&mut self) -> *mut C {
        &mut self.cefobj as *mut C
    }

    pub(crate) extern "C" fn add_ref(ref_counted: *mut cef_base_ref_counted_t) {
        let mut this = unsafe { Self::make_temp(ref_counted as *mut C) };
        this.refcount.fetch_add(1, Ordering::AcqRel);
    }
    pub(crate) extern "C" fn release(ref_counted: *mut cef_base_ref_counted_t) -> c_int {
        let mut this = unsafe { Self::make_temp(ref_counted as *mut C) };
        if this.refcount.fetch_sub(1, Ordering::AcqRel) > 1 {
            ManuallyDrop::into_inner(this);
            0
        } else { 1 }
    }
    extern "C" fn has_one_ref(ref_counted: *mut cef_base_ref_counted_t) -> c_int {
        let mut this = unsafe { Self::make_temp(ref_counted as *mut C) };
        let counter = this.refcount.load(Ordering::Acquire);
        if counter == 1 { 1 } else { 0 }
    }
    extern "C" fn has_at_least_one_ref(ref_counted: *mut cef_base_ref_counted_t) -> c_int {
        let mut this = unsafe { Self::make_temp(ref_counted as *mut C) };
        let counter = this.refcount.load(Ordering::Acquire);
        if counter >= 1 { 1 } else { 0 }
    }
}
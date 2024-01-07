#![cfg_attr(not(feature = "std"), no_std)]
#![deny(clippy::alloc_instead_of_core)]
#![deny(clippy::std_instead_of_core)]

//! Rust binding for [tcc](https://repo.or.cz/w/tinycc.git)
//!
//! # Example
//! ```
//! use std::ffi::CString;
//!
//! use tcc::{Context, Guard, OutputType};
//! let p = CString::new(
//!     r#"
//!     int add(int a, int b){
//!         return a+b;
//!     }
//!     "#
//!     .as_bytes(),
//! )
//! .unwrap();
//! let mut g = Guard::new().unwrap();
//! let mut ctx = Context::new(&mut g).unwrap();
//! assert!(ctx.compile_string(&p).is_ok());
//! ```

#[cfg(not(feature = "std"))] extern crate alloc;

#[cfg(feature = "std")] extern crate std as alloc;

use alloc::{boxed::Box, ffi::CString, rc::Rc, string::ToString, vec::Vec};
use core::{
    ffi::{c_char, c_int, c_void, CStr},
    mem::ManuallyDrop,
    ptr::null_mut,
};
#[cfg(feature = "std")] use std::path::Path;
#[cfg(feature = "std")] use std::sync::Mutex;

#[cfg(not(feature = "std"))] use spin::Mutex;
use tcc_sys::*;
use typed_arena::Arena;
#[cfg(not(feature = "std"))] use unix_path::Path;

static LOCK: Mutex<()> = Mutex::new(());

pub struct ContextGuard<'err, T> {
    #[allow(unused)]
    inner: ManuallyDrop<Rc<Scoped<'err>>>,
    data:  ManuallyDrop<T>,
}

impl<'err, T> Drop for ContextGuard<'err, T> {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.data);
            ManuallyDrop::drop(&mut self.inner);
        }
    }
}

impl<'err, T> ContextGuard<'err, T> {
    pub fn get(&self) -> &T {
        &self.data
    }
}

pub struct Scoped<'err>(Arena<Context<'err>>);

impl<'err> Default for Scoped<'err> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'err> Scoped<'err> {
    pub fn new() -> Self {
        Scoped(Arena::new())
    }

    pub fn spawn(&self) -> Result<&mut Context<'err>, ()> {
        if let Ok(context) = Context::new() {
            Ok(self.0.alloc(context))
        } else {
            Err(())
        }
    }
}

#[cfg(feature = "std")]
pub fn try_scoped<'err, F, T>(func: F) -> Result<ContextGuard<'err, T>, &'static str>
where
    F: FnOnce(Rc<Scoped>) -> T,
{
    match LOCK.try_lock() {
        Ok(_) => {
            let scoped = Rc::new(Scoped::new());
            Ok(ContextGuard {
                inner: ManuallyDrop::new(scoped.clone()),
                data:  ManuallyDrop::new(func(scoped)),
            })
        }
        Err(_) => Err("lock failed"),
    }
}

#[cfg(not(feature = "std"))]
pub fn try_scoped<'err, F, T>(func: F) -> Result<ContextGuard<'err, T>, &'static str>
where
    F: FnOnce(Rc<Scoped>) -> T,
{
    match LOCK.try_lock() {
        Some(_) => {
            let scoped = Rc::new(Scoped::new());
            Ok(ContextGuard {
                inner: ManuallyDrop::new(scoped.clone()),
                data:  ManuallyDrop::new(func(scoped)),
            })
        }
        None => Err("lock failed"),
    }
}

#[cfg(feature = "std")]
pub fn scoped<'err, F, T>(func: F) -> Result<ContextGuard<'err, T>, &'static str>
where
    F: FnOnce(Rc<Scoped>) -> T,
{
    let _lock = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let scoped = Rc::new(Scoped::new());
    Ok(ContextGuard {
        inner: ManuallyDrop::new(scoped.clone()),
        data:  ManuallyDrop::new(func(scoped)),
    })
}

#[cfg(not(feature = "std"))]
pub fn scoped<'err, F, T>(func: F) -> Result<ContextGuard<'err, T>, &'static str>
where
    F: FnOnce(Rc<Scoped>) -> T,
{
    let _lock = LOCK.lock();
    let scoped = Rc::new(Scoped::new());
    Ok(ContextGuard {
        inner: ManuallyDrop::new(scoped.clone()),
        data:  ManuallyDrop::new(func(scoped)),
    })
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
/// Output type of the compilation.
pub enum OutputType {
    /// output in memory (default)
    Memory = TCC_OUTPUT_MEMORY,

    /// executable file
    Exe = TCC_OUTPUT_EXE,

    /// dynamic library
    Dll = TCC_OUTPUT_DLL,

    /// object file
    Obj = TCC_OUTPUT_OBJ,

    /// only preprocess (used internally)
    Preprocess = TCC_OUTPUT_PREPROCESS,
}

/// Compilation context.
pub struct Context<'err> {
    inner:    *mut TCCState,
    err_func: Option<Box<Box<dyn 'err + FnMut(&CStr)>>>,
}

/// Real call back of tcc.
extern "C" fn call_back(opaque: *mut c_void, msg: *const c_char) {
    let func: *mut &mut dyn FnMut(&CStr) = opaque as *mut &mut dyn FnMut(&CStr);
    unsafe { (*func)(CStr::from_ptr(msg)) }
}

impl<'err> Context<'err> {
    /// Create a new context builder
    ///
    /// Context can not live together, mutable reference to guard makes compiler
    /// check this. Out of memory is only possible reason of failure.
    pub fn new() -> Result<Self, ()> {
        let inner = unsafe { tcc_new() };
        if inner.is_null() {
            // OOM
            Err(())
        } else {
            Ok(Self {
                inner,
                err_func: None,
            })
        }
    }

    /// set CONFIG_TCCDIR at runtime
    pub fn set_lib_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        let path = to_cstr(path);
        unsafe {
            tcc_set_lib_path(self.inner, path.as_ptr());
        }
        self
    }

    /// set options as from command line (multiple supported)
    pub fn set_options(&mut self, option: &CStr) -> &mut Self {
        unsafe {
            tcc_set_options(self.inner, option.as_ptr());
        }
        self
    }

    /// set error/warning display callback
    pub fn set_call_back<T>(&mut self, f: T) -> &mut Self
    where
        T: FnMut(&CStr) + 'err,
    {
        let mut user_err_func: Box<Box<dyn FnMut(&CStr)>> = Box::new(Box::new(f));
        // user_err_func.as_mut().
        unsafe {
            tcc_set_error_func(
                self.inner,
                user_err_func.as_mut() as *mut _ as *mut c_void,
                Some(call_back),
            )
        }
        self.err_func = Some(user_err_func);
        self
    }

    /// add include path
    pub fn add_include_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        let path = to_cstr(path);
        let ret = unsafe { tcc_add_include_path(self.inner, path.as_ptr()) };
        // this api only returns 0.
        assert_eq!(ret, 0);
        self
    }

    /// add in system include path
    pub fn add_sys_include_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        let path = to_cstr(path);
        let ret = unsafe { tcc_add_sysinclude_path(self.inner, path.as_ptr()) };
        // this api only returns 0.
        assert_eq!(ret, 0);
        self
    }

    /// define preprocessor symbol 'sym'. Can put optional value
    pub fn define_symbol(&mut self, sym: &CStr, val: &CStr) -> *mut Self {
        unsafe {
            tcc_define_symbol(self.inner, sym.as_ptr(), val.as_ptr());
        }
        self
    }

    /// undefine preprocess symbol 'sym'
    pub fn undefine_symbol(&mut self, sym: &CStr) -> &mut Self {
        unsafe { tcc_undefine_symbol(self.inner, sym.as_ptr()) }
        self
    }

    /// output an executable, library or object file. DO NOT call tcc_relocate()
    /// before
    pub fn set_output_type(&mut self, output: OutputType) -> &mut Self {
        let ret = unsafe { tcc_set_output_type(self.inner, output as c_int) };
        assert_eq!(ret, 0);
        self
    }

    /// add a file (C file, dll, object, library, ld script).
    pub fn add_file<T: AsRef<Path>>(&mut self, file: T) -> Result<(), ()> {
        let file = to_cstr(file);
        let ret = unsafe { tcc_add_file(self.inner, file.as_ptr()) };
        map_c_ret(ret)
    }

    ///  compile a string containing a C source.
    pub fn compile_string(&mut self, p: &CStr) -> Result<(), ()> {
        let ret = unsafe { tcc_compile_string(self.inner, p.as_ptr()) };
        map_c_ret(ret)
    }

    /// Equivalent to -Lpath option.
    pub fn add_library_path<T: AsRef<Path>>(&mut self, path: T) -> &mut Self {
        let path = to_cstr(path);
        let ret = unsafe { tcc_add_library_path(self.inner, path.as_ptr()) };
        assert_eq!(ret, 0);
        self
    }

    /// The library name is the same as the argument of the '-l' option.
    pub fn add_library(&mut self, lib_name: &CStr) -> Result<(), ()> {
        let ret = unsafe { tcc_add_library(self.inner, lib_name.as_ptr()) };
        map_c_ret(ret)
    }

    /// Add a symbol to the compiled program.
    ///
    /// # Safety
    /// Symbol need satisfy ABI requirement.
    pub unsafe fn add_symbol(&mut self, sym: &CStr, val: *const c_void) {
        let ret = tcc_add_symbol(self.inner, sym.as_ptr(), val);
        assert_eq!(ret, 0);
    }

    /// output an executable, library or object file.
    pub fn output_file<T: AsRef<Path>>(&mut self, file_name: T) -> Result<(), ()> {
        let file_name = to_cstr(file_name);
        let ret = unsafe { tcc_output_file(self.inner, file_name.as_ptr()) };

        map_c_ret(ret)
    }

    /// do all relocations (needed before get symbol)
    pub fn relocate<'a>(&'a mut self) -> Result<RelocatedCtx<'a, 'err>, ()> {
        // pass null ptr to get required length
        let len = unsafe { tcc_relocate(self.inner, null_mut()) };
        if len == -1 {
            return Err(());
        };
        let mut bin = Vec::with_capacity(len as usize);
        let ret = unsafe { tcc_relocate(self.inner, bin.as_mut_ptr() as *mut c_void) };
        if ret != 0 {
            return Err(());
        }
        unsafe {
            bin.set_len(len as usize);
        }

        Ok(RelocatedCtx {
            inner: self,
            _bin:  bin,
        })
    }
}

#[cfg(target_family = "unix")]
fn to_cstr<T: AsRef<Path>>(p: T) -> CString {
    use std::os::unix::ffi::OsStrExt;
    CString::new(p.as_ref().as_os_str().as_bytes()).unwrap()
}

#[cfg(target_family = "windows")]
fn to_cstr<T: AsRef<Path>>(p: T) -> CString {
    CString::new(p.as_ref().to_string_lossy().to_string().as_bytes()).unwrap()
}

// preprocessor
impl<'err> Drop for Context<'err> {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { tcc_delete(self.inner) }
        }
    }
}

fn map_c_ret(code: c_int) -> Result<(), ()> {
    if code == 0 {
        Ok(())
    } else {
        Err(())
    }
}

/// Relocated compilation context
pub struct RelocatedCtx<'a, 'err> {
    inner: &'a mut Context<'err>,
    _bin:  Vec<u8>,
}

impl<'a, 'err> RelocatedCtx<'a, 'err> {
    /// return symbol value or None if not found
    ///
    /// # Safety
    /// Returned addr can not outlive RelocatedCtx itself. It's caller's
    /// responsibility to take care of validity of addr.
    pub unsafe fn get_symbol(&mut self, sym: &CStr) -> Option<*mut c_void> {
        let addr = tcc_get_symbol(self.inner.inner, sym.as_ptr());
        if addr.is_null() {
            None
        } else {
            Some(addr)
        }
    }
}

#[cfg(test)] mod tests;

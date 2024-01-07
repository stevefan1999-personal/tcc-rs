/* automatically generated by rust-bindgen 0.69.1 */

pub const TCC_OUTPUT_MEMORY: u32 = 1;
pub const TCC_OUTPUT_EXE: u32 = 2;
pub const TCC_OUTPUT_DLL: u32 = 4;
pub const TCC_OUTPUT_OBJ: u32 = 3;
pub const TCC_OUTPUT_PREPROCESS: u32 = 5;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TCCState {
    _unused: [u8; 0],
}
pub type TCCErrorFunc = ::core::option::Option<
    unsafe extern "C" fn(opaque: *mut ::core::ffi::c_void, msg: *const ::core::ffi::c_char),
>;
extern "C" {
    pub fn tcc_new() -> *mut TCCState;
}
extern "C" {
    pub fn tcc_delete(s: *mut TCCState);
}
extern "C" {
    pub fn tcc_set_lib_path(s: *mut TCCState, path: *const ::core::ffi::c_char);
}
extern "C" {
    pub fn tcc_set_error_func(
        s: *mut TCCState,
        error_opaque: *mut ::core::ffi::c_void,
        error_func: TCCErrorFunc,
    );
}
extern "C" {
    pub fn tcc_get_error_func(s: *mut TCCState) -> TCCErrorFunc;
}
extern "C" {
    pub fn tcc_get_error_opaque(s: *mut TCCState) -> *mut ::core::ffi::c_void;
}
extern "C" {
    pub fn tcc_set_options(
        s: *mut TCCState,
        str_: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_add_include_path(
        s: *mut TCCState,
        pathname: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_add_sysinclude_path(
        s: *mut TCCState,
        pathname: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_define_symbol(
        s: *mut TCCState,
        sym: *const ::core::ffi::c_char,
        value: *const ::core::ffi::c_char,
    );
}
extern "C" {
    pub fn tcc_undefine_symbol(s: *mut TCCState, sym: *const ::core::ffi::c_char);
}
extern "C" {
    pub fn tcc_add_file(
        s: *mut TCCState,
        filename: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_compile_string(
        s: *mut TCCState,
        buf: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_set_output_type(
        s: *mut TCCState,
        output_type: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_add_library_path(
        s: *mut TCCState,
        pathname: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_add_library(
        s: *mut TCCState,
        libraryname: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_add_symbol(
        s: *mut TCCState,
        name: *const ::core::ffi::c_char,
        val: *const ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_output_file(
        s: *mut TCCState,
        filename: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_run(
        s: *mut TCCState,
        argc: ::core::ffi::c_int,
        argv: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_relocate(s1: *mut TCCState, ptr: *mut ::core::ffi::c_void) -> ::core::ffi::c_int;
}
extern "C" {
    pub fn tcc_get_symbol(
        s: *mut TCCState,
        name: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_void;
}
extern "C" {
    pub fn tcc_list_symbols(
        s: *mut TCCState,
        ctx: *mut ::core::ffi::c_void,
        symbol_cb: ::core::option::Option<
            unsafe extern "C" fn(
                ctx: *mut ::core::ffi::c_void,
                name: *const ::core::ffi::c_char,
                val: *const ::core::ffi::c_void,
            ),
        >,
    );
}

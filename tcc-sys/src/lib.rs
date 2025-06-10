#![cfg_attr(not(feature = "std"), no_std)]
#![feature(c_variadic, map_try_insert)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

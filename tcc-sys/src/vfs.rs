use core::ffi::CStr;
use std::{
    collections::HashMap,
    ffi::VaList,
    io::{Cursor, Read, Seek, SeekFrom},
    slice,
};

use const_random::const_random;
use libc::{c_char, c_int, c_void, off_t, size_t, ssize_t, SEEK_CUR, SEEK_END, SEEK_SET};
use once_cell::sync::Lazy;
use rand::{rngs::SmallRng, Rng, SeedableRng};

extern "C" {
    fn open(path: *const c_char, oflag: c_int, ap: VaList) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    fn lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t;
    fn close(fd: c_int) -> c_int;
}

pub trait VFS {
    fn read(&mut self, buf: &mut [u8]) -> Result<ssize_t, ()>;
    fn seek(&mut self, from: SeekFrom) -> Result<off_t, ()>;
    fn close(&mut self) -> Result<c_int, ()>;
}

#[derive(Clone, Copy)]
pub struct PosixVFS {
    fd: c_int,
}

impl PosixVFS {
    pub fn new(fd: c_int) -> Self {
        PosixVFS { fd }
    }
}

impl VFS for PosixVFS {
    fn read(&mut self, buf: &mut [u8]) -> Result<ssize_t, ()> {
        unsafe { Ok(read(self.fd, buf.as_mut_ptr().cast::<c_void>(), buf.len())) }
    }

    fn seek(&mut self, from: SeekFrom) -> Result<off_t, ()> {
        let (offset, whence) = match from {
            SeekFrom::Start(pos) => (pos.try_into().unwrap(), SEEK_SET),
            SeekFrom::End(pos) => (pos.try_into().unwrap(), SEEK_END),
            SeekFrom::Current(pos) => (pos.try_into().unwrap(), SEEK_CUR),
        };

        unsafe { Ok(lseek(self.fd, offset, whence)) }
    }

    fn close(&mut self) -> Result<c_int, ()> {
        unsafe { Ok(close(self.fd)) }
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum MemoryVFS {
    Static(Cursor<&'static [u8]>),
    Heap(Cursor<Vec<u8>>),
}

#[allow(dead_code)]
impl MemoryVFS {
    pub fn from_static(data: &'static [u8]) -> Self {
        MemoryVFS::Static(Cursor::new(data))
    }

    pub fn new(data: &[u8]) -> Self {
        MemoryVFS::Heap(Cursor::new(data.to_vec()))
    }
}

impl VFS for MemoryVFS {
    fn read(&mut self, buf: &mut [u8]) -> Result<ssize_t, ()> {
        if let Ok(n) = match self {
            MemoryVFS::Static(cursor) => cursor.read(buf),
            MemoryVFS::Heap(cursor) => cursor.read(buf),
        } {
            Ok(n.try_into().map_err(|_| ())?)
        } else {
            Err(())
        }
    }

    fn seek(&mut self, from: SeekFrom) -> Result<off_t, ()> {
        Ok(match self {
            MemoryVFS::Static(cursor) => cursor.seek(from),
            MemoryVFS::Heap(cursor) => cursor.seek(from),
        }
        .map_err(|_| ())?
        .try_into()
        .map_err(|_| ())?)
    }

    fn close(&mut self) -> Result<c_int, ()> {
        // noop
        Ok(0)
    }
}

static mut FILES: Lazy<HashMap<c_int, Box<dyn VFS + 'static + Sync + Send>>> =
    Lazy::new(|| HashMap::new());

static mut RNG: Lazy<SmallRng> = Lazy::new(|| SmallRng::seed_from_u64(const_random!(u64)));

#[no_mangle]
pub unsafe extern "C" fn vfs_open(path: *const c_char, oflag: c_int, mut args: ...) -> c_int {
    fn insert_vfs(vfs: Box<impl VFS + Send + Sync + Clone + 'static>) -> c_int {
        loop {
            let vfs = vfs.clone();
            let key: c_int = unsafe { RNG.gen_range(0..c_int::MAX) };
            if let Ok(_) = unsafe { FILES.try_insert(key, vfs) } {
                return key;
            }
        }
    }

    #[cfg(any(feature = "embed-headers", feature = "embed-libraries"))]
    if let Ok(path) = CStr::from_ptr(path).to_str() {
        #[cfg(feature = "embed-headers")]
        {
            let prefix = "memory:///headers/";

            if path.starts_with(prefix) {
                let path = path.strip_prefix(prefix).unwrap();

                if let Some(file) = crate::assets::headers::ASSETS.get_str(path) {
                    return insert_vfs(Box::new(MemoryVFS::from_static(file)));
                }
            }
        }

        #[cfg(feature = "embed-libraries")]
        {
            let prefix = "memory:///libraries/";

            if path.starts_with(prefix) {
                let path = path.strip_prefix(prefix).unwrap();
                if let Some(file) = crate::assets::libraries::ASSETS.get_str(path) {
                    return insert_vfs(Box::new(MemoryVFS::from_static(file)));
                }
            }
        }
    }

    let fd = open(path, oflag, args.as_va_list());
    if fd >= 0 {
        insert_vfs(Box::new(PosixVFS::new(fd)))
    } else {
        fd
    }
}

#[no_mangle]
pub unsafe extern "C" fn vfs_read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t {
    let buf = slice::from_raw_parts_mut(buf.cast::<u8>(), count);
    if let Some(vfs) = FILES.get_mut(&fd) {
        vfs.read(buf).unwrap_or(-1)
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn vfs_lseek(fd: c_int, offset: off_t, whence: c_int) -> off_t {
    if let Some(vfs) = FILES.get_mut(&fd) {
        vfs.seek(match whence {
            SEEK_SET => SeekFrom::Start(offset.try_into().unwrap()),
            SEEK_END => SeekFrom::End(offset.try_into().unwrap()),
            SEEK_CUR => SeekFrom::Current(offset.try_into().unwrap()),
            _ => return -1,
        })
        .unwrap_or(-1)
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn vfs_close(fd: c_int) -> c_int {
    if let Some(vfs) = FILES.get_mut(&fd) {
        let ret = vfs.close().unwrap_or(-1);
        FILES.remove(&fd);
        ret
    } else {
        -1
    }
}

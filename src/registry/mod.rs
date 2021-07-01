
mod ext;
mod key;
mod value;
mod maybe_uninit_value;

use std::{
    marker::PhantomData,
    ffi::{
        OsStr,
        OsString
    },
};

use winapi::{
    shared::{
        minwindef::HKEY,
        winerror::{
            ERROR_INVALID_DATA,
        },
    },
};

use super::{Error,Result,to_wstr};
use maybe_uninit_value::MaybeUninitValue;

pub type Name = OsStr;
pub type NameBuf = OsString;

pub use key::Key;
pub use ext::{KeyExt,ValueIterator,KeyNameIterator,ValueNameIterator};
pub use value::ValueBuf;

pub use key::HKEY_CURRENT_USER;
pub use key::HKEY_LOCAL_MACHINE;

struct KeyRef<'a>(HKEY, PhantomData<&'a Key>);

impl<'a> KeyExt for KeyRef<'a> {
    unsafe fn handle(&self) -> HKEY { self.0 }
}

impl<'a> From<&'a Key> for KeyRef<'a> {
    fn from(src: &'a Key) -> Self {
        Self(src.0, Default::default())
    }
}

// transmute [u8] slice to [u16] if size and alignment are correct
unsafe fn _to_wide_slice(src: &[u8]) -> Result<&[u16]> {
    let (head,body,tail) = src.align_to::<u16>();
    if head.is_empty() && tail.is_empty() {
        Ok(body)
    } else {
        Err(Error{code:ERROR_INVALID_DATA})
    }
}

unsafe fn _to_narrow_slice(src: &[u16]) -> Result<&[u8]> {
    let (head,body,tail) = src.align_to::<u8>();
    if head.is_empty() && tail.is_empty() {
        Ok(body)
    } else {
        Err(Error{code:ERROR_INVALID_DATA})
    }
}
unsafe fn to_narrow_slice_mut(src: &mut[u16]) -> Result<&mut[u8]> {
    let (head,body,tail) = src.align_to_mut::<u8>();
    if head.is_empty() && tail.is_empty() {
        Ok(body)
    } else {
        Err(Error{code:ERROR_INVALID_DATA})
    }
}
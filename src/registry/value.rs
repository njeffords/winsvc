
use std::{
    ffi::OsString,
    convert::TryFrom,
    os::windows::ffi::OsStringExt,
    mem::size_of,
 };
use super::{Result,to_wstr};
use winapi::{
    shared::winerror::ERROR_INVALID_DATA,
    um::winnt::{
        REG_SZ,
        REG_BINARY,
        REG_DWORD,
        REG_QWORD,
    },
};

#[derive(Clone,Debug,Eq,PartialEq)]
pub enum ValueBuf {
    String(Vec<u16>),
    Binary(Vec<u8>),
    Dword(u32),
    Qword(u64),
}

impl ValueBuf {
    pub fn as_string(&self) -> Result<OsString> {
        match self {
            Self::String(string) => Ok(OsString::from_wide(string)),
            _ => panic!()
        }
    }

    pub(super) fn type_ptr_len(&self) -> Result<(u32, (*const u8, u32))> {
        fn slice_ptr_len<T:Sized>(v: &[T]) -> (*const u8, u32) {
            (v.as_ptr() as *const _, (v.len()*size_of::<T>()) as u32)
        }
        fn any_ptr_len<T:Sized>(v: &T) -> (*const u8, u32) {
            (v as *const _ as *const _, size_of::<T>() as u32)
        }
        match self {
           ValueBuf::String(string) => Ok((REG_SZ, slice_ptr_len(string))),
           ValueBuf::Binary(value) => Ok((REG_BINARY, slice_ptr_len(value))),
           ValueBuf::Dword(value) => Ok((REG_DWORD, any_ptr_len(value))),
           ValueBuf::Qword(value) => Ok((REG_QWORD, any_ptr_len(value))),
        }
    }

}

const DATA_ERR : crate::Error = crate::Error{code:ERROR_INVALID_DATA};

macro_rules! u32s {
    ( $( $t:ty ),+ $(,)? ) => { $(

        impl From<$t> for ValueBuf {
            fn from(value: $t) -> Self {
                Self::Dword(value as _)
            }
        }

        impl TryFrom<ValueBuf> for $t {
            type Error = crate::Error;
            fn try_from(value: ValueBuf) -> Result<Self> {
                if let ValueBuf::Dword(value) = value {
                    Ok(value as $t)
                } else {
                    Err(DATA_ERR)
                }
            }
        }

    )+ }
}

macro_rules! u64s {
    ( $( $t:ty ),+ $(,)? ) => { $(

        impl From<$t> for ValueBuf {
            fn from(value: $t) -> Self {
                Self::Qword(value as _)
            }
        }

        impl TryFrom<ValueBuf> for $t {
            type Error = crate::Error;
            fn try_from(value: ValueBuf) -> Result<Self> {
                if let ValueBuf::Qword(value) = value {
                    Ok(value as $t)
                } else {
                    Err(DATA_ERR)
                }
            }
        }

    )+ }
}


u32s!{ u8, u16, u32, i8, i16, i32 }
u64s!{ u64, i64 }

impl From<bool> for ValueBuf {
    fn from(value: bool) -> Self {
        Self::Qword(if value { 1 } else { 0 })
    }
}

impl TryFrom<ValueBuf> for bool {
    type Error = crate::Error;
    fn try_from(value: ValueBuf) -> Result<Self> {
        match value {
            ValueBuf::Dword(value) => Ok(value != 0),
            ValueBuf::Qword(value) => Ok(value != 0),
            _ => Err(DATA_ERR)
        }
    }
}

impl From<char> for ValueBuf {
    fn from(_value: char) -> Self {
        todo!()
    }
}

impl From<&str> for ValueBuf {
    fn from(value: &str) -> Self {
        Self::String(to_wstr(value))
    }
}

impl From<String> for ValueBuf {
    fn from(value: String) -> Self {
        Self::String(to_wstr(value))
    }
}

impl TryFrom<ValueBuf> for String {
    type Error = crate::Error;
    fn try_from(value: ValueBuf) -> Result<Self> {
        if let ValueBuf::String(value) = value {
            Ok(OsString::from_wide(&value).into_string().map_err(|_|DATA_ERR)?)
        } else {
            Err(DATA_ERR)
        }
    }
}

impl From<&[u8]> for ValueBuf {
    fn from(value: &[u8]) -> Self {
        Self::Binary(value.into())
    }
}

impl From<Vec<u8>> for ValueBuf {
    fn from(value: Vec<u8>) -> Self {
        Self::Binary(value)
    }
}

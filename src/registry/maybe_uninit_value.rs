
use std::{
    mem::MaybeUninit,
};

use winapi::{
    shared::{
        winerror::{
            ERROR_INVALID_DATA,
        },
    },
    um::{
        winnt::{
            REG_SZ,
            REG_BINARY,
            REG_DWORD,
            REG_QWORD,
        },
    },
};

use super::{
    ValueBuf, Error, Result,
    to_narrow_slice_mut,
};

pub enum MaybeUninitValue {
    String(Vec<u16>),
    Binary(Vec<u8>),
    Dword(MaybeUninit<u32>),
    Qword(MaybeUninit<u64>),
}

impl MaybeUninitValue {
    pub fn uninit(ty: u32, len: u32) -> Result<Self> {
        match ty {
            REG_SZ => Ok(Self::String({ let mut v = Vec::new(); v.resize((len / 2) as usize, 0); v })),
            REG_BINARY => Ok(Self::Binary({ let mut v = Vec::new(); v.resize(len as usize, 0); v })),
            REG_DWORD => Ok(Self::Dword(MaybeUninit::uninit())),
            REG_QWORD => Ok(Self::Qword(MaybeUninit::uninit())),
            _ => Err(Error{code: ERROR_INVALID_DATA})
        }
    }

    pub unsafe fn ptr_and_len(&mut self) ->  Result<(*mut u8, u32)> {
        match self {
            Self::String(value) => {
                let slice = to_narrow_slice_mut(value)?;
                Ok((slice.as_mut_ptr(), (slice.len () * 2) as u32))
            },
            Self::Binary(value) => {
                Ok((value.as_mut_ptr(), value.len() as u32))
            }
            Self::Dword(value) => {
                Ok((value.as_mut_ptr() as *mut () as *mut u8, 4))
            }
            Self::Qword(value) => {
                Ok((value.as_mut_ptr() as *mut () as *mut u8, 8))
            }
        }
    }

    pub unsafe fn assume_init(self, value_type: u32) -> Result<ValueBuf> {
        match self {
            Self::String(mut value) => {
                if value_type == REG_SZ {
                    if let Some(0) = value.last() {
                        value.truncate(value.len() - 1);
                    }
                    Ok(ValueBuf::String(value))
                } else {
                    Err(Error{code:ERROR_INVALID_DATA})
                }
            },
            Self::Binary(value) => {
                if value_type == REG_BINARY {
                    Ok(ValueBuf::Binary(value))
                } else {
                    Err(Error{code:ERROR_INVALID_DATA})
                }
            },
            Self::Dword(value) => {
                if value_type == REG_DWORD {
                    Ok(ValueBuf::Dword(value.assume_init()))
                } else {
                    Err(Error{code:ERROR_INVALID_DATA})
                }
            },
            Self::Qword(value) => {
                if value_type == REG_QWORD {
                    Ok(ValueBuf::Qword(value.assume_init()))
                } else {
                    Err(Error{code:ERROR_INVALID_DATA})
                }
            },
        }
    }
}

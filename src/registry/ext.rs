
use std::{
    ptr::null_mut,
    marker::PhantomData,
    ffi::OsString,
    mem::MaybeUninit,
    os::windows::ffi::{
        //OsStrExt,
        OsStringExt,
    },
};

use winapi::{
    shared::{
        minwindef::HKEY,
        winerror::{
            ERROR_NO_MORE_ITEMS,
        },
    },
    um::{
        winreg::{
            RegQueryValueExW,
            RegSetValueExW,
            RegEnumValueW,
            RegEnumKeyW,
        }
    },
};

use super::{
    Key, Name, NameBuf, ValueBuf,
    MaybeUninitValue,
    Error, Result, to_wstr,
};

#[cfg(feature = "serde_config")]
use serde::{Serialize};

pub trait KeyExt {

    #[doc(hidden)]
    unsafe fn handle(&self) -> HKEY;

    fn try_clone(&self) -> Result<Key> {
        unsafe { Key::clone_handle(self.handle()) }
    }

    fn set_value(&self, name: impl AsRef<Name>, value: &ValueBuf) -> Result<()> {
        unsafe { raw_set_value(self.handle(), name, value.type_ptr_len()?) }
    }

    fn query_value(&self, name: impl AsRef<Name>) -> Result<ValueBuf> {
        unsafe {
            let name = name.as_ref();

            let (value_type, value_len) = raw_query_value(self.handle(), name, (null_mut(), 0))?;

            let mut value = MaybeUninitValue::uninit(value_type, value_len)?;

            let (value_type, _) = raw_query_value(self.handle(), name, value.ptr_and_len()?)?;

            value.assume_init(value_type)
        }
    }

    #[cfg(feature = "serde_config")]
    fn set_object(&self, name: impl Into<String>, value: &impl Serialize) -> Result<()> {
        use crate::serde_config::Serializer;

        let key = self.try_clone()?;

        let mut ser = Serializer::new(key, name.into());

        value.serialize(&mut ser)
    }

    #[cfg(feature = "serde_config")]
    fn query_object<V>(&self, name: impl Into<String>) -> Result<V> where V: serde::de::DeserializeOwned {
        use crate::serde_config::Deserializer;

        let name = name.into();

        tracing::trace!("query_object(name={:?})", name);

        let key = self.try_clone()?;

        tracing::trace!("  cloned");

        let mut des = Deserializer::new(key, name.into());

        V::deserialize(&mut des)
    }

    fn iter_values(&self) -> ValueIterator {
        ValueIterator(unsafe {self.handle()}, 0, Default::default())
    }

    fn iter_key_names(&self) -> KeyNameIterator {
        KeyNameIterator(unsafe {self.handle()}, 0, Default::default())
    }

    fn iter_value_names(&self) -> ValueNameIterator {
        ValueNameIterator(unsafe {self.handle()}, 0, Default::default())
    }
}

pub struct ValueIterator<'a>(HKEY,u32,PhantomData<&'a Key>);

pub struct KeyNameIterator<'a>(HKEY,u32,PhantomData<&'a Key>);
pub struct ValueNameIterator<'a>(HKEY,u32,PhantomData<&'a Key>);

unsafe fn raw_query_value(key: HKEY, name: impl AsRef<Name>, (value_ptr, mut value_len): (*mut u8, u32)) -> Result<(u32,u32)> {
    let value_name: &[u16] = &to_wstr(name);
    let mut value_type = MaybeUninit::uninit();
    Error::check_code(RegQueryValueExW(
        /* hKey        */ key,
        /* lpValueName */ value_name.as_ptr(),
        /* lpReserved  */ null_mut(),
        /* lpType      */ value_type.as_mut_ptr(),
        /* lpData      */ value_ptr,
        /* lpcbData    */ &mut value_len,
    ))?;
    let value_type = value_type.assume_init();
    Ok((value_type,value_len))
}

unsafe fn raw_set_value(key: HKEY, name: impl AsRef<Name>, (value_type, (value_ptr, value_len)): (u32, (*const u8, u32))) -> Result<()> {
    let value_name: &[u16] = &to_wstr(name);
    Error::check_code(RegSetValueExW(
        /* hKey        */ key,
        /* lpValueName */ value_name.as_ptr(),
        /* Reserved    */ 0u32,
        /* dwType      */ value_type,
        /* lpData      */ value_ptr,
        /* cbData      */ value_len,
    ))
}

fn extra_raw_enum_value(key: HKEY, index: u32, name: &mut [u16], (value_ptr, value_len): (*mut u8, u32)) -> Result<(u32,u32,u32)> {

    let (name, mut name_len) = (name.as_mut_ptr(), name.len () as u32);
    let mut value_type : u32 = 0;
    let mut value_len = value_len;

    Error::check_code(unsafe { RegEnumValueW(
        /* hKey */ key,
        /* dwIndex */ index,
        /* lpValueName */ name,
        /* lpcchValueName */ &mut name_len,
        /* lpReserved */ null_mut(),
        /* lpType */ &mut value_type,
        /* lpData */ value_ptr,
        /* lpcbData */ &mut value_len,
    ) })?;

    Ok((name_len, value_type, value_len))
}

fn raw_enum_value(key: HKEY, index: u32) -> Result<(NameBuf,ValueBuf)> {
    unsafe {

        let mut name_buffer : [u16;512] = std::mem::zeroed();

        let (name_len, value_type, value_len) = extra_raw_enum_value(key, index, &mut name_buffer, (null_mut(), 0))?;

        let name : &[u16] = &name_buffer[0..(name_len as usize)];
        let name = Vec::from(name);

        let mut value = MaybeUninitValue::uninit(value_type, value_len)?;

        let (_, value_type, _) = extra_raw_enum_value(
            key,
            index,
            &mut name_buffer,
            value.ptr_and_len()?
        )?;

        let name = OsString::from_wide (std::slice::from_raw_parts (name.as_ptr(), name.len()));
        let value = value.assume_init(value_type)?;

        Ok((name, value))
    }
}

unsafe fn raw_enum_key(key: HKEY, index: u32, name: &mut [u16]) -> Result<u32> {

    let name_len = name.len() as u32;

    Error::check_code(RegEnumKeyW(
        /* hKey    */ key,
        /* dwIndex */ index,
        /* lpName  */ name.as_mut_ptr(),
        /* cchName */ name_len,
    ))?;

    Ok(match name.iter().position(|v|*v == 0) {
        Some(index) => index,
        None => name.len ()
    } as u32)
}

fn name_buffer_to_string(buf: &[u16], len: u32) -> OsString {
    OsString::from_wide (&buf[0..(len as usize)])
}

fn raw_enum_value_name(key: HKEY, index: u32) -> Result<NameBuf> {
    unsafe {

        let mut name_buf : [u16;512] = std::mem::zeroed();

        let (name_len, _, _) = extra_raw_enum_value(key, index, &mut name_buf, (null_mut(), 0))?;

        Ok(name_buffer_to_string(&mut name_buf, name_len))
    }
}

impl<'a> Iterator for ValueIterator<'a> {
    type Item=Result<(NameBuf,ValueBuf)>;

    fn next(&mut self) -> Option<Self::Item> {
        match raw_enum_value(self.0, self.1) {
            Ok(named_value) => {
                self.1 += 1;
                Some(Ok(named_value))
            },
            Err(error) => {
                if error.code == ERROR_NO_MORE_ITEMS {
                    None
                } else {
                    Some(Err(error))
                }
            }
        }
    }
}

impl<'a> Iterator for KeyNameIterator<'a> {
    type Item=Result<NameBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let mut name_buf : [u16;512] = std::mem::zeroed();
            match raw_enum_key(self.0, self.1, &mut name_buf) {
                Ok(name_len) => {
                    self.1 += 1;
                    Some(Ok(name_buffer_to_string(&mut name_buf, name_len)))
                },
                Err(error) => {
                    if error.code == ERROR_NO_MORE_ITEMS {
                        None
                    } else {
                        Some(Err(error))
                    }
                }
            }
        }
    }
}

impl<'a> Iterator for ValueNameIterator<'a> {
    type Item=Result<NameBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        match raw_enum_value_name(self.0, self.1) {
            Ok(named_value) => {
                self.1 += 1;
                Some(Ok(named_value))
            },
            Err(error) => {
                if error.code == ERROR_NO_MORE_ITEMS {
                    None
                } else {
                    Some(Err(error))
                }
            }
        }
    }
}

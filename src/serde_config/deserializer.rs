
use winapi::shared::winerror::ERROR_INVALID_DATA;

use serde::{self, de, de::IntoDeserializer};

use std::{ffi::OsString, vec::IntoIter,convert::TryInto};

use crate::{
    Result, Error, registry::{
        Key, KeyExt, ValueBuf, KeyNameIterator, ValueNameIterator
    }
};

impl de::Error for Error {
    fn custom<T>(_err: T) -> Self {
        Error{code:ERROR_INVALID_DATA}
    }
}

pub struct Deserializer {
    keys: Vec<Key>,
    name: Option<String>
}

impl Deserializer {

    pub fn new(key: Key, name: String) -> Self {
        Self{ keys: vec![ key ], name: Some(name) }
    }

    pub fn push(&mut self, name: String) -> Result<()> {
        eprintln!("pushing: {}", name);
        self.open()?;
        self.name = Some(name);
        Ok(())
    }

    pub fn pop(&mut self) {
        if self.name.take().is_none() {
            eprintln!("closing key");
            self.keys.pop().unwrap();
        } else {
            eprintln!("poping name");
        }
    }

    pub fn query_value(&mut self) -> Result<ValueBuf> {
        let name = self.name.take().unwrap();
        eprintln!("query-popping: {}", name);
        self.keys.last().unwrap().query_value(name)
    }

    pub fn iter_keys(&mut self) -> Result<KeyNameIterator> {
        self.open()?;
        Ok(self.keys.last().unwrap().iter_key_names())
    }

    pub fn iter_values(&mut self) -> Result<ValueNameIterator> {
        self.open()?;
        Ok(self.keys.last().unwrap().iter_value_names())
    }

    pub fn iter_names(&mut self) -> Result<Vec<OsString>> {
        let mut names = Vec::with_capacity(64);
        for name in self.iter_keys()? {
            names.push(name?);
        }
        for name in self.iter_values()? {
            names.push(name?);
        }
        Ok(names)
    }

    fn open(&mut self) -> Result<()> {
        if let Some(name) = self.name.take() {
            eprintln!("open key: {}", name);
            let subkey = self.keys.last().unwrap().open(name)?;
            self.keys.push(subkey);
        }
        Ok(())
    }
}

struct StructMapAccess<'a>{
    des: &'a mut Deserializer,
    iter: IntoIter<String>,
}

impl<'a> StructMapAccess<'a> {
    fn new(des: &'a mut Deserializer, _fields: &'static [&'static str]) -> Result<Self> {
        let raw_names = des.iter_names()?;
        let mut names = Vec::with_capacity(raw_names.len());
        for raw_name in raw_names {
            names.push(raw_name.into_string().map_err(|_|Error{code:ERROR_INVALID_DATA})?);
        }
        let iter = names.into_iter();
        // todo - verify names match fields
        Ok(Self{des,iter})
    }
}

impl<'de,'a> de::MapAccess<'de> for StructMapAccess<'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>> where K: de::DeserializeSeed<'de> {
        if let Some(key) = self.iter.next() {
            let value = { let de : &str = &key; seed.deserialize(de.into_deserializer()).map(|v|Some(v))? };
            self.des.push(key)?;
            Ok(value)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value> where V: de::DeserializeSeed<'de> {
        seed.deserialize(&mut*self.des)
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_bool(self.query_value()?.try_into()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_i8(self.query_value()?.try_into()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_i16(self.query_value()?.try_into()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_i32(self.query_value()?.try_into()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_i64(self.query_value()?.try_into()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_u8(self.query_value()?.try_into()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_u16(self.query_value()?.try_into()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_u32(self.query_value()?.try_into()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_u64(self.query_value()?.try_into()?)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_string(self.query_value()?.try_into()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        visitor.visit_string(self.query_value()?.try_into()?)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_struct<V>(self, _name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        let value = visitor.visit_map(StructMapAccess::new(&mut*self, fields)?)?;
        self.pop();
        Ok(value)
    }

    fn deserialize_enum<V>(self, _name: &'static str, _variants: &'static [&'static str],  _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value> where V: de::Visitor<'de> {
        panic!()
    }

}

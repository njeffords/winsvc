
use serde::ser;

use crate::{Error,Result,registry::{Key,KeyExt,ValueBuf}};

pub struct Serializer{
    keys: Vec<Key>,
    name: Option<String>,
}

pub struct PanicSerializer;
pub struct SeqSerializer<'a>(&'a mut Serializer,u32);

impl Serializer {
    pub fn new(parent_key: Key, value_name: String) -> Self {
        Self{keys: vec![ parent_key ], name: Some(value_name)}
    }

    fn push(&mut self, name: String) -> Result<()> {
        if let Some(parent_name) = std::mem::replace(&mut self.name, None) {
            tracing::trace!("open: {:?}", parent_name);
            let sub_key = self.keys.last().unwrap().create(parent_name)?;
            self.keys.push(sub_key);
        }
        tracing::trace!("push: {:?}", name);
        self.name = Some(name);
        Ok(())
    }

    fn write(&mut self, value: impl Into<ValueBuf>) -> Result<()> {
        if let Some(name) = std::mem::replace(&mut self.name, None) {
            let value = value.into();
            tracing::trace!("set value: {:?}={:?}", name, value);
            self.keys.last().unwrap().set_value(name, &value)
        } else {
            panic!()
        }
    }

    fn pop(&mut self) {
        if self.name.is_some() {
            tracing::trace!("pop name: {:?}", self.name.as_ref().unwrap());
            self.name = None;
        } else {
            tracing::trace!("pop key");
            self.keys.pop().unwrap();
        }
    }
}

impl ser::Error for Error {
   fn custom<T>(_err: T) -> Self {
       todo!()
   }
}

impl<'a> ser::Serializer for &'a mut Serializer {

    // error handling

    type Ok = ();
    type Error = Error;

    // subtype serializers

    type SerializeSeq = SeqSerializer<'a>;
    type SerializeTuple = SeqSerializer<'a>;
    type SerializeTupleStruct = SeqSerializer<'a>;
    type SerializeTupleVariant = PanicSerializer;
    type SerializeMap = PanicSerializer;
    type SerializeStruct = Self;
    type SerializeStructVariant = PanicSerializer;

    // basic types...

    fn serialize_bool(self, v: bool) -> Result<()> { self.write(v) }
    fn serialize_i8(self, v: i8) -> Result<()> { self.write(v) }
    fn serialize_i16(self, v: i16) -> Result<()> { self.write(v) }
    fn serialize_i32(self, v: i32) -> Result<()> { self.write(v) }
    fn serialize_i64(self, v: i64) -> Result<()> { self.write(v) }
    fn serialize_u8(self, v: u8) -> Result<()> { self.write(v) }
    fn serialize_u16(self, v: u16) -> Result<()> { self.write(v) }
    fn serialize_u32(self, v: u32) -> Result<()> { self.write(v) }
    fn serialize_u64(self, v: u64) -> Result<()> { self.write(v) }

    fn serialize_f32(self, _: f32) -> Result<()> { panic!() }
    fn serialize_f64(self, _: f64) -> Result<()> { panic!() }

    fn serialize_char(self, v: char) -> Result<()> { self.write(v) }
    fn serialize_str(self, v: &str) -> Result<()> { self.write(v) }
    fn serialize_bytes(self, v: &[u8]) -> Result<()> { self.write(v) }

    fn serialize_none(self) -> Result<()> { Ok(()) }
    fn serialize_some<T: ?Sized + ser::Serialize> (self, value: &T) -> Result<()> { value.serialize(self) }

    fn serialize_unit(self) -> Result<()> { panic!() }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> { panic!() }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        panic!()
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut*self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        panic!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SeqSerializer(&mut*self,0))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(SeqSerializer(&mut*self,0))
    }

    // Tuple structs look just like sequences in JSON.
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SeqSerializer(&mut*self,0))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(PanicSerializer)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(PanicSerializer)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Ok(&mut*self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(PanicSerializer)
    }
}

impl<'a> ser::SerializeSeq for SeqSerializer<'a> {

    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.1 += 1;
        self.0.push(self.1.to_string())?;
        value.serialize(&mut*self.0)
    }

    // Close the sequence.
    fn end(self) -> Result<()> {
        self.0.pop();
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.1 += 1;
        self.0.push(self.1.to_string())?;
        value.serialize(&mut*self.0)
    }

    fn end(self) -> Result<()> {
        self.0.pop();
        Ok(())
    }
}


impl<'a> ser::SerializeTupleStruct for SeqSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.1 += 1;
        self.0.push(self.1.to_string())?;
        value.serialize(&mut*self.0)
    }

    fn end(self) -> Result<()> {
        self.0.pop();
        Ok(())
    }
}


impl ser::SerializeTupleVariant for PanicSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        panic!() /*
        if !self.output.ends_with('[') {
            self.output += ",";
        }
        value.serialize(&mut **self)
        */
    }

    fn end(self) -> Result<()> {
        panic!() /*
        self.output += "]}";
        Ok(())
        */
    }
}


impl<'a> ser::SerializeMap for PanicSerializer {
    type Ok = ();
    type Error = Error;

    // This can be done by using a different Serializer to serialize the key
    // (instead of `&mut **self`) and having that other serializer only
    // implement `serialize_str` and return an error on any other data type.
    fn serialize_key<T>(&mut self, _key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        panic!() /*
        if !self.output.ends_with('{') {
            self.output += ",";
        }
        key.serialize(&mut **self)
        */
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        panic!() /*
        self.output += ":";
        value.serialize(&mut **self)
        */
    }

    fn end(self) -> Result<()> {
        panic!() /*
        self.output += "}";
        Ok(())
        */
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.push(key.into())?;
        value.serialize(&mut**self)?;
        Ok(())
    }

    fn end(self) -> Result<()> {
        self.pop();
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for PanicSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        panic!() /*
        if !self.output.ends_with('{') {
            self.output += ",";
        }
        key.serialize(&mut **self)?;
        self.output += ":";
        value.serialize(&mut **self)
        */
    }

    fn end(self) -> Result<()> {
        panic!() /*
        self.output += "}}";
        Ok(())
        */
    }
}

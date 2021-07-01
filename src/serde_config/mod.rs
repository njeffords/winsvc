
mod serializer;
mod deserializer;

pub use serializer::*;
pub use deserializer::*;

#[cfg(test)]
mod test {
    use super::*;
    use crate::registry::HKEY_CURRENT_USER;

    use serde::{Serialize,Deserialize};

    #[derive(Serialize, Deserialize, Eq, PartialEq)]
    struct Simplest {
        value_1: String,
        value_2: u32,
    }

    impl Default for Simplest {
        fn default() -> Self {
            Self{
                value_1: "Hello World".into (),
                value_2: 42,
            }
        }
    }

    #[derive(Serialize, Deserialize, Eq, PartialEq)]
    struct SimpleNested {
        value_1: u8,
        value_2: i32,
        value_3: Simplest,
    }

    impl Default for SimpleNested {
        fn default() -> Self {
            Self{
                value_1: 1,
                value_2: 2,
                value_3: Default::default(),
            }
        }
    }

    fn ser(name: &str, value: &impl Serialize) {

        let mut serializer = Serializer::new(HKEY_CURRENT_USER.create("SOFTWARE\\n8ware\\test\\winsvc").unwrap(), name.into());

        value.serialize(&mut serializer).unwrap();

    }

    fn de<'de,T>(name: &str) -> T where T: Deserialize<'de> {
        let mut deserializer = Deserializer::new(HKEY_CURRENT_USER.create("SOFTWARE\\n8ware\\test\\winsvc").unwrap(), name.into());

        T::deserialize(&mut deserializer).unwrap()
    }

    fn check_ser_de<'de,T>(name: &str, org_value: T) -> bool where T: Serialize + Deserialize<'de> + PartialEq {

        ser(name, &org_value);

        let new_value = de::<T>(name);

        return org_value == new_value;

    }

    #[test]
    fn simplest() { check_ser_de("simplest", Simplest::default()); }

    #[test]
    fn simple_nested() { check_ser_de("simple_nested", SimpleNested::default()); }

    #[test]
    fn sequence() {
        let mut v = Vec::new();
        v.push(10);
        v.push(20);
        v.push(30);
        check_ser_de("sequence", v);
    }

    #[test]
    fn nested_sequence() {
        let mut v = Vec::new();
        v.push(Simplest::default());
        v.push(Simplest::default());
        v.push(Simplest::default());
        check_ser_de("nested_sequence", v);
    }

}

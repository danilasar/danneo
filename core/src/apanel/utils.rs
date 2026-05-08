use serde::{Deserialize, Deserializer};

pub fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let s = String::deserialize(de)?;
    if s.is_empty() {
        Ok(None)
    } else {
        match s.parse::<T>() {
            Ok(val) => Ok(Some(val)),
            Err(e) => Err(serde::de::Error::custom(e)),
        }
    }
}

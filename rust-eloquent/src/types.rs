use std::ops::{Deref, DerefMut};

/// Transparent wrapper for JSON columns
#[derive(Debug, Clone)]
pub struct Json<T>(pub T);

impl<T> Deref for Json<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// Decode logic
impl<'r, T: serde::de::DeserializeOwned> sqlx::Decode<'r, sqlx::Any> for Json<T> {
    fn decode(value: sqlx::any::AnyValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let text = <String as sqlx::Decode<sqlx::Any>>::decode(value)?;
        let parsed = serde_json::from_str(&text)?;
        Ok(Json(parsed))
    }
}

// Encode logic
impl<'q, T: serde::Serialize> sqlx::Encode<'q, sqlx::Any> for Json<T> {
    fn encode_by_ref(&self, buf: &mut <sqlx::Any as sqlx::Database>::ArgumentBuffer<'q>) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let text = serde_json::to_string(&self.0)?;
        <String as sqlx::Encode<sqlx::Any>>::encode(text, buf)
    }
}

// Type logic
impl<T> sqlx::Type<sqlx::Any> for Json<T> {
    fn type_info() -> sqlx::any::AnyTypeInfo {
        <String as sqlx::Type<sqlx::Any>>::type_info()
    }
}

impl<T: serde::Serialize> serde::Serialize for Json<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Json<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Json)
    }
}

use std::ops::{Deref, DerefMut};

/// A wrapper for JSON columns in the database.
/// This type allows users to easily cast a column to a struct that implements Serialize and Deserialize.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
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

#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql", feature = "strict-sqlite")))]
impl<'r, T: serde::de::DeserializeOwned> sqlx::Decode<'r, sqlx::Any> for Json<T> {
    fn decode(value: sqlx::any::AnyValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let text = <String as sqlx::Decode<sqlx::Any>>::decode(value)?;
        let parsed: T = serde_json::from_str(&text)?;
        Ok(Json(parsed))
    }
}

#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql", feature = "strict-sqlite")))]
impl<'q, T: serde::Serialize> sqlx::Encode<'q, sqlx::Any> for Json<T> {
    fn encode_by_ref(&self, buf: &mut <sqlx::Any as sqlx::database::Database>::ArgumentBuffer) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let stringified = serde_json::to_string(&self.0)?;
        <String as sqlx::Encode<sqlx::Any>>::encode(stringified, buf)
    }
}

#[cfg(not(any(feature = "strict-postgres", feature = "strict-mysql", feature = "strict-sqlite")))]
impl<T> sqlx::Type<sqlx::Any> for Json<T> {
    fn type_info() -> sqlx::any::AnyTypeInfo {
        <String as sqlx::Type<sqlx::Any>>::type_info()
    }
}

#[cfg(any(feature = "strict-postgres", feature = "strict-mysql", feature = "strict-sqlite"))]
impl<'r, T: serde::de::DeserializeOwned> sqlx::Decode<'r, crate::database::EloquentDatabase> for Json<T> {
    fn decode(value: <crate::database::EloquentDatabase as sqlx::database::Database>::ValueRef<'r>) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let text = <String as sqlx::Decode<crate::database::EloquentDatabase>>::decode(value)?;
        let parsed: T = serde_json::from_str(&text)?;
        Ok(Json(parsed))
    }
}

#[cfg(any(feature = "strict-postgres", feature = "strict-mysql", feature = "strict-sqlite"))]
impl<'q, T: serde::Serialize> sqlx::Encode<'q, crate::database::EloquentDatabase> for Json<T> {
    fn encode_by_ref(&self, buf: &mut <crate::database::EloquentDatabase as sqlx::database::Database>::ArgumentBuffer) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>> {
        let stringified = serde_json::to_string(&self.0)?;
        <String as sqlx::Encode<crate::database::EloquentDatabase>>::encode(stringified, buf)
    }
}

#[cfg(any(feature = "strict-postgres", feature = "strict-mysql", feature = "strict-sqlite"))]
impl<T> sqlx::Type<crate::database::EloquentDatabase> for Json<T> {
    fn type_info() -> <crate::database::EloquentDatabase as sqlx::database::Database>::TypeInfo {
        <String as sqlx::Type<crate::database::EloquentDatabase>>::type_info()
    }
}

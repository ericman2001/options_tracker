/// Defines a string-backed enum together with the boilerplate needed to move
/// its variants in and out of SQLite and to/from `String`.
///
/// For each variant a canonical lowercase string is provided. The macro
/// generates:
/// - `as_str` returning the canonical string,
/// - `rusqlite`'s `ToSql`/`FromSql` (stored as text),
/// - `Display`,
/// - `From<Self> for String`,
/// - `From<String> for Self` (case-insensitive, falling back to `default`).
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident => $text:literal),+ $(,)?
        }
        default = $default:ident,
        invalid = $invalid:literal $(,)?
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $($variant),+
        }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            fn from_str(value: &str) -> ::std::result::Result<Self, ::rusqlite::types::FromSqlError> {
                match value {
                    $($text => Ok(Self::$variant),)+
                    _ => Err(::rusqlite::types::FromSqlError::Other(Box::from($invalid))),
                }
            }
        }

        impl ::rusqlite::types::ToSql for $name {
            fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                Ok(::rusqlite::types::ToSqlOutput::from(self.as_str()))
            }
        }

        impl ::rusqlite::types::FromSql for $name {
            fn column_result(
                value: ::rusqlite::types::ValueRef<'_>,
            ) -> ::rusqlite::types::FromSqlResult<Self> {
                match value {
                    ::rusqlite::types::ValueRef::Text(text) => {
                        let value = ::std::str::from_utf8(text)
                            .map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)?;
                        Self::from_str(value)
                    }
                    _ => Err(::rusqlite::types::FromSqlError::InvalidType),
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.to_string()
            }
        }

        impl From<String> for $name {
            fn from(s: String) -> Self {
                match s.to_lowercase().as_str() {
                    $($text => Self::$variant,)+
                    _ => Self::$default,
                }
            }
        }
    };
}

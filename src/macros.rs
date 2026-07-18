/// Defines a string-backed enum together with the boilerplate needed to move
/// its variants in and out of SQLite and to/from strings.
///
/// For each variant a canonical lowercase string is provided. The macro
/// generates:
/// - `as_str` returning the canonical string,
/// - `variants` returning all variants in declaration order,
/// - a case-insensitive `FromStr` (erroring with `Invalid <error>: <input>`),
/// - `rusqlite`'s `ToSql`/`FromSql` (stored as text, parsed via `FromStr`),
/// - `Display`,
/// - `From<Self> for String`.
macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident => $text:literal),+ $(,)?
        }
        error = $error:literal $(,)?
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

            /// All variants in declaration order. Used to populate UI selectors
            /// so they never drift from the enum definition.
            pub fn variants() -> &'static [Self] {
                &[$(Self::$variant),+]
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = String;

            fn from_str(value: &str) -> ::std::result::Result<Self, Self::Err> {
                match value.to_lowercase().as_str() {
                    $($text => Ok(Self::$variant),)+
                    other => Err(format!(concat!("Invalid ", $error, ": {}"), other)),
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
                        value
                            .parse()
                            .map_err(|e| ::rusqlite::types::FromSqlError::Other(Box::from(e)))
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
    };
}

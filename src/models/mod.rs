pub mod record;
pub mod settings;
pub mod transfer;

pub use record::*;
pub use settings::*;
pub use transfer::*;

/// 宏定义：自动生成 SQLite 桥接代码
#[macro_export]
macro_rules! impl_sql_for_enum {
    ($enum:ty, $($variant:ident => $value:expr),+ $(,)?) => {
        impl rusqlite::ToSql for $enum {
            fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
                let s = match self {
                    $( Self::$variant => $value, )+
                };
                Ok(rusqlite::types::ToSqlOutput::from(s))
            }
        }

        impl rusqlite::types::FromSql for $enum {
            fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
                value.as_str().and_then(|s| match s {
                    $( $value => Ok(Self::$variant), )+
                    _ => Err(rusqlite::types::FromSqlError::Other(
                        format!("invalid value for {}: {}", stringify!($enum), s).into(),
                    )),
                })
            }
        }
    };
}

// 使用宏一键生成 ContentType 的 SQLite 桥接实现
impl_sql_for_enum!(record::ContentType,
    Text => "text",
    Image => "image",
    Html => "html",
    Link => "link",
);

// 使用宏一键生成 Theme 的 SQLite 桥接实现
impl_sql_for_enum!(settings::Theme,
    Light => "light",
    Dark => "dark",
    System => "system",
);

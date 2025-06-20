/// A macro to implement the `From` trait for a specified enum variant.
///
/// This macro simplifies the process of implementing the `From` trait for enums where
/// a particular variant wraps a type.
///
/// # Parameters
/// - `$dst_enum`: The destination enum type for which the `From` implementation is generated.
/// - `$variant`: The variant of the enum that will wrap the payload.
/// - `$payload`: The type that the specified enum variant will wrap.
///
/// # Examples
///
/// ```
/// use utils::impl_from_variant;
///
/// enum Message {
///     Text(String),
///     Integer(i32),
/// }
///
/// impl_from_variant!(Message, Text, String);
/// impl_from_variant!(Message, Integer, i32);
///
/// let text_message: Message = String::from("Hello").into();
/// let number_message: Message = 32i32.into();
/// ```
///
/// This will create implementations of `From<String>` for `Message` converting a `String`
/// to `Message::Text`, and `From<i32>` for `Message` converting an `i32` to `Message::Integer`.
#[macro_export]
macro_rules! impl_from_variant {
    ($dst_enum:ident, $variant:ident, $payload:ident) => {
        impl From<$payload> for $dst_enum {
            fn from(value: $payload) -> Self {
                Self::$variant(value)
            }
        }
    };
}

// When you are okay with using a nightly compiler it's better to use https://docs.rs/static_cell/2.1.0/static_cell/macro.make_static.html
#[macro_export]
macro_rules! mk_static {
    ($t:ty, $val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

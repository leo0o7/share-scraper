#[macro_export]
macro_rules! generate_from_element {
    ($struct_name:ident, { $($field_name:ident: $field_type:ty),* $(,)? }) => {
        impl $struct_name {
            pub fn from_element(isin: &str, table: ElementRef) -> Option<$struct_name> {
                Some($struct_name {
                    isin: isin.to_owned(),
                    $(
                        $field_name: {
                            select_for_prop(stringify!($field_name), table).map(|el| el.default_parse()).unwrap_or_default()
                        }
                    ),*
                })
            }
        }
    };
}

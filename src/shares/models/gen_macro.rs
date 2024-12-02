#[macro_export]
macro_rules! generate_scrapable_struct {
    ($struct_name:ident, { $($field_name:ident: $field_type:ty),* $(,)? }) => {
        impl ScrapableStruct for $struct_name {
            fn from_element(share_isin: &ShareIsin, table: ElementRef) -> Self{
                    Self {
                        isin: share_isin.isin.get_str(),
                        $(
                            $field_name: select_for_prop(stringify!($field_name), table).map(|el| el.safe_parse()).flatten(),
                        )*
                    }
            }
            fn with_isin(share_isin: &ShareIsin) -> $struct_name {
                $struct_name {
                    isin: share_isin.isin.get_str(),
                    $($field_name: None),*
                }
            }
        }
    };
}

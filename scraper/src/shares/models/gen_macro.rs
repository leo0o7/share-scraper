pub use crate::isins::types::ShareIsin;
pub use crate::shares::models::ScrapableStruct;
pub use crate::shares::parsers::SafeParse;
pub use crate::shares::property_selector::PropertySelector;
pub use chrono::NaiveDateTime;
pub use tracing::warn;

#[macro_export]
macro_rules! generate_scrapable_struct {
    ($struct_name:ident, { $($field_name:ident: $field_type:ty),* $(,)? }) => {
        impl ScrapableStruct for $struct_name {
            fn from_selector(share_isin: &ShareIsin, selector: &PropertySelector) -> Self {
                Self {
                    isin: share_isin.isin.to_string(),
                    updated_at: chrono::offset::Utc::now().naive_utc(),
                    $(
                        $field_name: selector.get_property(stringify!($field_name)).map(|el| el.safe_parse()).flatten(),
                    )*
                }
            }
            fn with_isin(share_isin: &ShareIsin) -> $struct_name {
                warn!("Creating empty {}", stringify!($struct_name));
                $struct_name {
                    isin: share_isin.isin.to_string(),
                    $($field_name: None,)*
                    updated_at: chrono::offset::Utc::now().naive_utc(),
                }
            }
        }
    };
}

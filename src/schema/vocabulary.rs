use super::keyword::{Get, Keyword, QuerySchema};
use super::keywords::*;

macro_rules! define_vocabulary {
    ($vocab:ident, keywords: [$($kw:ty),* $(,)?]) => {
        pub trait $vocab: QuerySchema $(+ Get<$kw>)* + Sized {}

        impl<T> $vocab for T where T: QuerySchema $(+ Get<$kw>)* {}

        pub fn check_keywords<Sub, Sup, E>(
            sub: &Sub,
            sup: &Sup,
            is_subtype: &mut impl FnMut(&Sub, &Sup) -> bool,
        ) -> Result<bool, E>
        where
            Sub: $vocab,
            Sup: $vocab,
            Sub::Error: Into<E>,
            Sup::Error: Into<E>,
        {
            $(
                {
                    let sub_val = <Sub as Get<$kw>>::get(sub).map_err(Into::into)?;
                    let sup_val = <Sup as Get<$kw>>::get(sup).map_err(Into::into)?;
                    if !<$kw as Keyword>::check_subtype(&sub_val, &sup_val, is_subtype) {
                        return Ok(false);
                    }
                }
            )*
            Ok(true)
        }
    };
}

define_vocabulary! {
    Draft2020_12,
    keywords: [
        TypeKw,
        UpperBoundKw, LowerBoundKw,
        MultipleOfKw,
        MaxLengthKw, MinLengthKw,
        MaxItemsKw, MinItemsKw,
        UniqueItemsKw,
        MaxContainsKw, MinContainsKw,
        MaxPropertiesKw, MinPropertiesKw,
        RequiredKw,
        ConstKw, EnumKw,
        PropertiesKw, PatternPropertiesKw,
        AdditionalPropertiesKw, PropertyNamesKw,
        ItemsKw, PrefixItemsKw, ContainsKw,
    ]
}

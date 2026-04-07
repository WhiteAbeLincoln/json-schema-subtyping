use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{SubtypeError, SubtypeRelation};
use crate::located::LocatedValue;

/// Context provided to extension subtype checks for recursive checking.
pub struct SubtypeContext<'a> {
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> SubtypeContext<'a> {
    pub(crate) fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

/// A vocabulary extension that adds custom keywords, extraction, and subtype checking.
pub trait VocabExtension: Send + Sync + 'static {
    type Extracted: Send + Sync + 'static;

    /// Keywords this extension handles.
    fn keywords(&self) -> &[&str];

    /// Extract typed data from the keyword values found in a schema object.
    fn extract(
        &self,
        keywords: &HashMap<&str, &LocatedValue>,
    ) -> Result<Self::Extracted, SubtypeError>;

    /// Check subtype relationship between extracted data.
    fn check_subtype(
        &self,
        sub: &Self::Extracted,
        sup: &Self::Extracted,
        ctx: &mut SubtypeContext<'_>,
    ) -> Result<SubtypeRelation, SubtypeError>;
}

/// Type-erased wrapper around a VocabExtension.
pub struct ExtensionSlot {
    pub keywords: Vec<String>,
    extract_fn: Box<dyn Fn(&HashMap<&str, &LocatedValue>) -> Result<Box<dyn Any + Send + Sync>, SubtypeError> + Send + Sync>,
    check_fn: Box<dyn Fn(&dyn Any, &dyn Any, &mut SubtypeContext<'_>) -> Result<SubtypeRelation, SubtypeError> + Send + Sync>,
}

impl ExtensionSlot {
    pub fn new<E: VocabExtension>(ext: E) -> Self {
        let keywords = ext.keywords().iter().map(|s| s.to_string()).collect();
        let ext = Arc::new(ext);
        let ext_extract = ext.clone();
        let ext_check = ext;

        Self {
            keywords,
            extract_fn: Box::new(move |kws| {
                let data = ext_extract.extract(kws)?;
                Ok(Box::new(data))
            }),
            check_fn: Box::new(move |sub, sup, ctx| {
                let sub = sub.downcast_ref::<E::Extracted>().expect("extension type mismatch");
                let sup = sup.downcast_ref::<E::Extracted>().expect("extension type mismatch");
                ext_check.check_subtype(sub, sup, ctx)
            }),
        }
    }

    pub fn extract(&self, kws: &HashMap<&str, &LocatedValue>) -> Result<Box<dyn Any + Send + Sync>, SubtypeError> {
        (self.extract_fn)(kws)
    }

    pub fn check(&self, sub: &dyn Any, sup: &dyn Any, ctx: &mut SubtypeContext<'_>) -> Result<SubtypeRelation, SubtypeError> {
        (self.check_fn)(sub, sup, ctx)
    }
}

/// Storage for extracted extension data on a reduced schema.
pub struct ExtensionData {
    pub slots: Vec<Box<dyn Any + Send + Sync>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DetailedOutput;

    struct TestExt;
    struct TestData(Option<String>);

    impl VocabExtension for TestExt {
        type Extracted = TestData;

        fn keywords(&self) -> &[&str] {
            &["x-test"]
        }

        fn extract(
            &self,
            keywords: &HashMap<&str, &LocatedValue>,
        ) -> Result<TestData, SubtypeError> {
            let val = keywords
                .get("x-test")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Ok(TestData(val))
        }

        fn check_subtype(
            &self,
            sub: &TestData,
            sup: &TestData,
            _ctx: &mut SubtypeContext<'_>,
        ) -> Result<SubtypeRelation, SubtypeError> {
            match (&sub.0, &sup.0) {
                (_, None) => Ok(SubtypeRelation::Subtype),
                (None, Some(_)) => Ok(SubtypeRelation::NotSubtype(DetailedOutput {
                    message: "sub missing x-test".into(),
                })),
                (Some(s), Some(p)) => {
                    if s == p {
                        Ok(SubtypeRelation::Subtype)
                    } else {
                        Ok(SubtypeRelation::NotSubtype(DetailedOutput {
                            message: format!("x-test mismatch: {s} vs {p}"),
                        }))
                    }
                }
            }
        }
    }

    #[test]
    fn extension_slot_roundtrip() {
        let slot = ExtensionSlot::new(TestExt);
        assert_eq!(slot.keywords, vec!["x-test".to_string()]);

        // Test extraction with empty keywords
        let kws = HashMap::new();
        let data = slot.extract(&kws).unwrap();
        let test_data = data.downcast_ref::<TestData>().unwrap();
        assert!(test_data.0.is_none());
    }

    #[test]
    fn extension_subtype_check() {
        let slot = ExtensionSlot::new(TestExt);
        let kws = HashMap::new();
        let sub = slot.extract(&kws).unwrap();
        let sup = slot.extract(&kws).unwrap();
        let mut ctx = SubtypeContext::new();
        let result = slot.check(&*sub, &*sup, &mut ctx).unwrap();
        assert!(result.is_subtype());
    }
}

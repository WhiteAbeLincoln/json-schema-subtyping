use std::convert::Infallible;

use crate::output::Location;

use super::node::SchemaF;
use super::traits::{JsonSchema, Located};

/// Plain schema tree -- the fixed point of `SchemaF<String, _>`.
///
/// This is `Fix SchemaF` from recursion scheme theory.
/// Owns all its data (String keys, boxed children).
pub struct Schema(pub Box<SchemaF<String, Schema>>);

impl JsonSchema for Schema {
    type ViewError = Infallible;

    fn try_view(&self) -> Result<SchemaF<&str, &Self>, Infallible> {
        Ok(self.0.borrow())
    }
}

impl Located for Schema {}

/// Annotated schema tree -- the cofree comonad over `SchemaF<String, _>`.
///
/// This is `Cofree SchemaF Ann` from recursion scheme theory.
/// Each node carries an annotation of type `Ann` alongside the schema layer.
pub struct Annotated<Ann>(pub Ann, pub Box<SchemaF<String, Annotated<Ann>>>);

impl<Ann> Annotated<Ann> {
    /// Access the annotation at this node.
    pub fn annotation(&self) -> &Ann {
        &self.0
    }
}

impl<Ann> JsonSchema for Annotated<Ann> {
    type ViewError = Infallible;

    fn try_view(&self) -> Result<SchemaF<&str, &Self>, Infallible> {
        Ok(self.1.borrow())
    }
}

impl Located for Annotated<Location> {
    fn location(&self) -> Option<&Location> {
        Some(&self.0)
    }
}

impl Located for Annotated<Option<Location>> {
    fn location(&self) -> Option<&Location> {
        self.0.as_ref()
    }
}

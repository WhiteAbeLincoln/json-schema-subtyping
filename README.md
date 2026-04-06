# json-schema-subtyping

A Rust library for determining subtyping relationships between JSON Schemas.
This library provides a function `is_subtype` that checks if one JSON Schema is a
subtype of another, following the rules defined in the JSON Schema specification.

The library supports rich errors, giving the exact location in the input
schemas that caused the subtyping check to fail, along with a human-readable message
explaining the reason for the failure.

Supports JSON Schema Edition 2020-12. Other editions can be supported through
rewriting the schemas to conform to the 2020-12 specification, but this requires
an explicit `$schema` field in the input schemas.

Recursive references are not supported at the moment, though recurive types can
follow subtyping rules (see [Subtyping recursive types](https://dl.acm.org/doi/10.1145/155183.155231)).
This may be added in a future version.

Custom vocabularies are supported if they can be simplified and canonicalized to
our reduced schema representation. The library provides a pluggable API for adding
additional rewrite rules.

If a vocabulary contains semantically different keywords, then custom inference
rules can also be added through the pluggable API.

## Future work
- Support for recursive references.
- Support for more JSON Schema features, such as `unevaluatedProperties` and `unevaluatedItems`.
- Better inputs - right now we accept JSON schema as a string and parse internally. It would be better
  to accept an already deserialized JSON value if it implements a trait allowing rewrites and schema lookups.
  This would allow us to use other JSON representations like `serde_json::Value` or even arbitrary Rust structs.
- Incremental parsing and checking? Would be useful to use something like salsa, then we can integrate this into
  a language server or other compilers. This will help with performance, the rewriting steps can be expensive for
  large schemas.

## References

- [Type Safety with JSON Subschema](https://arxiv.org/abs/1911.12651)
- [Finding data compatibility bugs with JSON subschema checking](https://dl.acm.org/doi/10.1145/3460319.3464796) - The same paper as above
  as presented in ISSTA 2021.

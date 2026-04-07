// Test suite translated from IBM/jsonsubschema (Python).
//
// Original source: https://github.com/IBM/jsonsubschema
// Copyright 2017-2018 IBM Corporation
// Licensed under the Apache License, Version 2.0
// SPDX-License-Identifier: Apache-2.0
//
// Original test authors: Andrew Habib et al.
//
// These tests have been mechanically translated from Python to Rust with
// the following adaptations for JSON Schema draft 2020-12:
//   - `exclusiveMinimum: true` (draft-4 boolean) → `exclusiveMinimum: <value>` (2020-12 numeric)
//   - `items: [list]` + `additionalItems` → `prefixItems` + `items`
//   - `dependencies` → `dependentSchemas` / `dependentRequired`
//
// Tests that require features not yet implemented (allOf meet, negation
// elimination) are marked `#[ignore]` with a comment explaining why.

mod null;
mod string;
mod numeric;
mod boolean;
mod array;
mod object;
mod r#enum;
mod r#const;
mod mix;

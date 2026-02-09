{
  name = "type keyword";
  description = ''
    Tests for schemas consisting only of the `type` keyword.

    A schema with a `type` keyword is a subtype of another schema
    if the set of types it allows is a subset of the set of types
    allowed by the other schema.
  '';
  tests = let
    types = [
      "null"
      "boolean"
      "object"
      "array"
      "number"
      "string"
      "integer"
    ];
  in [
    # TODO: this is ideal for a property-based test...
    # Property: a `type` schema A is a subtype of a `type` schema B iff the
    # set of types allowed by A is a subset of the set of types allowed by B.
    {
      cases = [
        {
          sup = [
            {type = types;}
            {type = ["null" "boolean" "object"];}
          ];
          sub = [
            {type = "null";}
            {type = ["boolean" "object"];}
          ];
        }
      ];
    }
    {
      name = "a `type` schema is a subtype of itself";
      cases = [
        {
          sup = {type = types;};
          sub = {type = types;};
          result = true;
        }
      ];
    }
    {
      name = "A is not a subtype of B if it allows a type that B does not allow";
      cases = [
        {
          sup = {type = ["null" "boolean" "object"];};
          sub = {type = ["null" "boolean" "object" "array"];};
          result = false;
        }
      ];
    }
  ];
}

{
  description = "Tests for top and bottom types in JSON Schema";
  tests = let
    # top types can be written in many ways
    top-types = [
      {}
      true
      # because type applies to all types of values and we've
      # formed a union of every possible type, this is also the top type
      {type = ["null" "boolean" "object" "array" "number" "string" "integer"];}
      # applying not to the bottom type also gives us the top type
      {not = false;}
      {not = {not = true;};}
    ];
    bottom-types =
      [
        false
      ]
      ++ (map (t: {not = t;}) top-types);
    everything-else = [
      # these keywords apply no matter the kind of
      # value compared to them
      {type = "null";}
      {type = "boolean";}
      {type = "object";}
      {type = "array";}
      {type = "number";}
      {type = "string";}
      {type = "integer";}
      {enum = [1 2];}
      {const = "foo";}
      # following apply only when the instance is a number
      {multipleOf = 10;}
      {maximum = 10;}
      {exclusiveMaximum = 10;}
      {minimum = 10;}
      {exclusiveMinimum = 10;}
      # following only applies for string instances
      {maxLength = 10;}
      {minLength = 10;}
      {pattern = "foo";}
      # array instances
      {maxItems = 10;}
      {minItems = 10;}
      {uniqueItems = true;}
      {maxContains = 10;}
      {minContains = 10;}
      # object instances
      {maxProperties = 10;}
      {minProperties = 10;}
      {required = ["foo"];}
      # skipped: dependentRequired
      # skipped: format
      # skipped: contentEncoding, contentMediaType, contentSchema
    ];
  in [
    {
      name = "everything is subtype of top";
      cases = [
        {
          result = true;
          sup = top-types;
          sub =
            top-types
            ++ bottom-types
            ++ everything-else;
        }
      ];
    }
    {
      name = "bottom is subtype of everything";
      cases = [
        {
          result = true;
          sup =
            top-types
            ++ bottom-types
            ++ everything-else;
          sub = bottom-types;
        }
      ];
    }
    {
      name = "bottom is supertype of nothing (except bottom)";
      cases = [
        {
          result = false;
          sup = bottom-types;
          sub = top-types ++ everything-else;
        }
        {
          result = true;
          sup = bottom-types;
          sub = bottom-types;
        }
      ];
    }
  ];
}

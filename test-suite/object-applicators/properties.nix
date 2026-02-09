{
  description = ''
    Tests for the `properties` keyword, which applies subschema
    validation to specific properties of an object.

    Property: a schema S is a subtype of a schema T iff
    the properties in S are a superset of the properties in T, and for each property in S,
    the schema for that property is a subtype of the corresponding property in T.

    Unmentioned properties in a schema are implicitly unconstrained (i.e. the top type),
    but this behavior depends on the `additionalProperties` and `unevaluatedProperties` keywords.
  '';
  tests = [
    {
      name = "sub has more properties than sup";
      cases = [
        {
          name = "sub-schemas are subtypes";
          result = true;
          sup = {
            properties = {
              a = {type = "string";};
              b = {type = "number";};
            };
          };
          sub = {
            # same properties, but b in sub is a subtype
            # of b in sub (integer is a subtype of number)
            properties = {
              a = {type = "string";};
              b = {type = "integer";};
            };
          };
        }
        {
          result = true;
          sup = {
            properties = {
              a = {type = "string";};
              b = {type = "number";};
            };
          };
          sub = [
            {
              properties = {
                a = {type = "string";};
                b = {type = "number";};
              };
            }

            {
              properties = {
                a = {type = "string";};
                b = {type = "number";};
                c = {type = "boolean";};
              };
            }
          ];
        }
      ];
    }
    {
      name = "sub has fewer properties than sup";
      cases = [
        {
          name = "sub has no properties";
          description = ''
            Any unmentioned property in sub is implicitly unconstrained (i.e. the top type)
            and so it is not a subtype of the corresponding property in sup (unless that property in sup is also unconstrained).
          '';
          result = false;
          sup = {
            properties = {
              a = {type = "string";};
              b = {type = "number";};
            };
          };
          sub = {
            properties = {};
          };
        }
        {
          name = "property in sup is unconstrained";
          result = true;
          sup = {
            properties = {
              a = {type = "string";};
              b = {};
            };
          };
          sub = {
            properties = {
              a = {type = "string";};
            };
          };
        }
      ];
    }
  ];
}

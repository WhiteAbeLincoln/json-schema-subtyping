{
  description = ''
    Tests for object keywords not involving subschemas: `maxProperties`, `minProperties`, and `required`.
  '';
  tests = [
    {
      # Property: a schema with maxProperties = n is a subtype of a schema with maxProperties = m iff n <= m.
      name = "maxProperties";
      cases = [
        {
          sup = {maxProperties = 10;};
          sub = {maxProperties = 5;};
          result = true;
        }
        {
          sup = {maxProperties = 5;};
          sub = {maxProperties = 10;};
          result = false;
        }
      ];
    }
    {
      name = "minProperties";
      # Property: a schema with minProperties = n is a subtype of a schema with minProperties = m iff n >= m.
      cases = [
        {
          sup = {minProperties = 5;};
          sub = {minProperties = 10;};
          result = true;
        }
        {
          sup = {minProperties = 10;};
          sub = {minProperties = 5;};
          result = false;
        }
      ];
    }
    {
      name = "required";
      # Property: a schema with required = [a1 ... an] is a subtype of a schema with required = [b1 ... bm] iff {a1 ... an} is a superset of {b1 ... bm}.
      # i.e. a accepts at least the required properties of b.
      cases = [
        {
          sup = {required = ["foo"];};
          sub = {required = ["foo" "bar"];};
          result = true;
        }
        {
          sup = [{required = ["foo" "bar"];} {required = ["foo"];}];
          sub = [{required = ["baz"];} {required = [];}];
          # no properties in common, sub doesn't
          # require all the properties that sup requires, so sub is not a subtype of sup
          result = false;
        }
      ];
    }
    {
      name = "unsatisfiable schemas";
      description = ''
        As with numeric and string schemas, if both minProperties and maxProperties are present,
        the schema is unsatisfiable if minProperties > maxProperties.

        An unsatisfiable schema is a subtype of every schema, but only subsumes other unsatisfiable schemas.
      '';
      cases = [
        {
          sup = {
            minProperties = 5;
            maxProperties = 10;
          };
          sub = {
            minProperties = 10;
            maxProperties = 5;
          };
          result = true;
        }
        {
          sup = {
            minProperties = 10;
            maxProperties = 5;
          };
          sub = {
            minProperties = 5;
            maxProperties = 10;
          };
          result = false;
        }
        {
          sup = false;
          sub = {
            minProperties = 10;
            maxProperties = 5;
          };
          result = true;
        }
      ];
    }
  ];
}

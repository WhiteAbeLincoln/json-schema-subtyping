{
  description = ''
    Tests for schemas that only apply to numeric instances.
    These schemas are subtypes of each other if they apply to
    the same set of numeric instances.

    For example, {multipleOf: 10} is a subtype of {multipleOf: 5}
    because every number that is a multiple of 10 is also a multiple of 5,
    but not vice versa.
  '';
  # the general property is:
  # A is a subtype of B if the set of numeric instances that satisfy A
  # is a subset of the set of numeric instances that satisfy B.
  # But that doesn't say much, it's essentially the definition of subtyping (for numbers).
  tests = [
    {
      name = "multipleOf";
      # another property-based test candidate
      # Property: a schema with multipleOf = n is a subtype of a schema
      # with multipleOf = m iff n is a multiple of m.
      cases = [
        {
          sup = {multipleOf = 5;};
          sub = {multipleOf = 10;};
          result = true;
        }
        {
          sup = {multipleOf = 10;};
          sub = {multipleOf = 5;};
          result = false;
        }
      ];
    }
    {
      name = "maximum";
      # Property: a schema with maximum = n is a subtype
      # of a schema with maximum = m iff n <= m.
      cases = [
        {
          # x <= 10
          sup = {maximum = 10;};
          # x <= 5
          sub = {maximum = 5;};
          result = true;
        }
        {
          sup = {maximum = 5;};
          sub = {maximum = 10;};
          result = false;
        }
      ];
    }
    {
      name = "exclusiveMaximum";
      # Property: a schema with exclusiveMaximum = n is a subtype
      # of a schema with exclusiveMaximum = m iff n <= m.
      cases = [
        {
          sup = {exclusiveMaximum = 10;};
          sub = {exclusiveMaximum = 5;};
          result = true;
        }
        {
          sup = {exclusiveMaximum = 5;};
          sub = {exclusiveMaximum = 10;};
          result = false;
        }
      ];
    }
    {
      name = "exclusiveMaximum vs maximum";
      # exclusiveMaximum: n is equal to maximum: n + epsilon for some small epsilon.
      # so maximum: n is a subtype of exclusiveMaximum: n, but not vice versa
      cases = [
        {
          # x <= 10
          sup = {maximum = 10;};
          # x < 10
          sub = {exclusiveMaximum = 10;};
          result = true;
        }
        {
          # x < 10 - does not include 10
          sup = {exclusiveMaximum = 10;};
          # x <= 10 - includes 10
          sub = {maximum = 10;};
          # because sub accepts more values than sup,
          # sub is not a subtype of sup
          result = false;
        }
        # when restricting to integer instances,
        # maximum: x is equivalent to exclusiveMaximum: x + 1
        # x <= 10 is equal to x < 11
        (let
          schemas = [
            {
              maximum = 10;
              type = "integer";
            }
            {
              exclusiveMaximum = 11;
              type = "integer";
            }
            {
              maximum = 10;
              multipleOf = 1;
            }
            {
              exclusiveMaximum = 11;
              multipleOf = 1;
            }
          ];
        in {
          name = "integer instances";
          sup = schemas;
          sub = schemas;
          result = true;
        })
      ];
    }
    {
      name = "minimum";
      # Property: a schema with minimum = n is a subtype
      # of a schema with minimum = m iff n >= m.
      cases = [
        {
          sup = {minimum = 5;};
          sub = {minimum = 10;};
          result = true;
        }
        {
          sup = {minimum = 10;};
          sub = {minimum = 5;};
          result = false;
        }
      ];
    }
    {
      name = "exclusiveMinimum";
      # Property: a schema with exclusiveMinimum = n is a subtype
      # of a schema with exclusiveMinimum = m iff n >= m.
      cases = [
        {
          sup = {exclusiveMinimum = 5;};
          sub = {exclusiveMinimum = 10;};
          result = true;
        }
        {
          sup = {exclusiveMinimum = 10;};
          sub = {exclusiveMinimum = 5;};
          result = false;
        }
      ];
    }
    {
      name = "minimum vs exclusiveMinimum";
      # minimum: n is equal to exclusiveMinimum: n - epsilon for some small epsilon.
      # so exclusiveMinimum: n is a subtype of minimum: n, but not vice versa.
      cases = [
        {
          # x >= 10
          sup = {minimum = 10;};
          # x > 10
          sub = {exclusiveMinimum = 10;};
          result = true;
        }
        {
          # x > 10 - does not include 10
          sup = {exclusiveMinimum = 10;};
          # x >= 10 - includes 10
          sub = {minimum = 10;};
          # because sub accepts more values than sup,
          # sub is not a subtype of sup
          result = false;
        }
        # when restricting to integer instances,
        # minimum: x is equivalent to exclusiveMinimum: x - 1
        # x >= 10 is equal to x > 9
        (let
          schemas = [
            {
              minimum = 10;
              type = "integer";
            }
            {
              exclusiveMinimum = 9;
              type = "integer";
            }
            # we can also write "integer"
            # using multipleOf: 1
            {
              minimum = 10;
              multipleOf = 1;
            }
            {
              exclusiveMinimum = 9;
              multipleOf = 1;
            }
          ];
        in {
          name = "integer instances";
          sup = schemas;
          sub = schemas;
          result = true;
        })
      ];
    }
    {
      name = "both minimum and maximum";
      # Property: a schema with minimum = m and maximum = n is a subtype of
      # a schema with minimum = m' and maximum = n' iff m >= m' and n <= n'.
      # We can just compare the minimums and maximums separately,
      # since the set of instances that satisfy
      # minimum = m and maximum = n is the intersection of the sets of instances
      # that satisfy minimum = m and maximum = n separately.
      cases = [
        {
          # 5 <= x <= 10
          sup = {
            minimum = 5;
            maximum = 10;
          };
          # 7 <= x <= 9
          sub = {
            minimum = 7;
            maximum = 9;
          };
          result = true;
        }
        # same for exclusiveMinimum and exclusiveMaximum
        {
          # 5 < x < 10
          sup = {
            exclusiveMinimum = 5;
            exclusiveMaximum = 10;
          };
          # 7 < x < 9
          sub = {
            exclusiveMinimum = 7;
            exclusiveMaximum = 9;
          };
          result = true;
        }
        {
          # 5 <= x <= 10
          sup = {
            minimum = 5;
            maximum = 10;
          };
          # 3 <= x <= 9
          sub = {
            minimum = 3;
            maximum = 9;
          };
          result = false;
        }
      ];
    }
    {
      name = "unsatisfiable schemas";
      description = ''
        If both minimum and maximum are specified, but minimum > maximum, then the schema is unsatisfiable.
        This is the bottom type, so it is a subtype of every schema, but no schema (except itself) is a subtype of it.
      '';
      cases = [
        {
          sup = [
            {
              minimum = 10;
              maximum = 5;
            }
            false
          ];
          # x >= 10 and x <= 5 is unsatisfiable
          sub = {
            minimum = 10;
            maximum = 5;
          };
          result = true;
        }
      ];
    }
  ];
}

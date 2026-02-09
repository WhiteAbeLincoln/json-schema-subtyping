{
  description = ''
    Tests for array keywords not involving subschemas: `maxItems`, `minItems`, `uniqueItems`, `maxContains`, and `minContains`.
  '';
  tests = [
    {
      name = "maxItems";
      # Property: a schema with maxItems = n is a subtype of a schema with maxItems = m iff n <= m.
      cases = [
        {
          sup = {maxItems = 10;};
          sub = {maxItems = 5;};
          result = true;
        }
        {
          sup = {maxItems = 5;};
          sub = {maxItems = 10;};
          result = false;
        }
      ];
    }
    {
      name = "minItems";
      # Property: a schema with minItems = n is a subtype of a schema with minItems = m iff n >= m.
      cases = [
        {
          sup = {minItems = 5;};
          sub = {minItems = 10;};
          result = true;
        }
        {
          sup = {minItems = 10;};
          sub = {minItems = 5;};
          result = false;
        }
      ];
    }
    {
      name = "uniqueItems";
      # Property: a schema with uniqueItems = true is a subtype of a schema with uniqueItems = false, but not vice versa.
      cases = [
        {
          sup = {uniqueItems = false;};
          sub = {uniqueItems = true;};
          result = true;
        }
        {
          sup = {uniqueItems = true;};
          sub = {uniqueItems = false;};
          result = false;
        }
      ];
    }
    {
      name = "maxContains";
      # Property: a schema with maxContains = n is a subtype of a schema with maxContains = m iff n <= m.
      cases = [
        {
          sup = {maxContains = 10;};
          sub = {maxContains = 5;};
          result = true;
        }
        {
          sup = {maxContains = 5;};
          sub = {maxContains = 10;};
          result = false;
        }
      ];
    }
    {
      name = "minContains";
      # Property: a schema with minContains = n is a subtype of a schema with minContains = m iff n >= m.
      cases = [
        {
          sup = {minContains = 5;};
          sub = {minContains = 10;};
          result = true;
        }
        {
          sup = {minContains = 10;};
          sub = {minContains = 5;};
          result = false;
        }
      ];
    }
    {
      name = "unsatisfiable schemas";
      cases = [
        rec {
          sup = [
            {
              maxItems = 5;
              minItems = 10;
            }
            {
              maxContains = 5;
              minContains = 10;
            }
            false
          ];
          sub = sup;
          result = true;
        }
      ];
    }
  ];
}

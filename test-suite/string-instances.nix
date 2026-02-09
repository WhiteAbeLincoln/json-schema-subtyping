{
  description = ''
    Tests for schemas against string instances.
    Pattern is not supported, so we only test minLength and maxLength.

    Determining if one regex is a subset of another is possible for truly regular
    languages, but a JSON schema accepts a regex in the ECMA 262 syntax, which includes
    features that make it impossible to determine if one regex is a subset of another.
    Additionally, there are not many libraries out there that support regex subset testing.
    It may be worth implementing as it's own crate in the future.
  '';
  tests = [
    # very similar to the number instances
    # minLength and maxLength are equivalent to minimum and maximum
    # except that they apply to the length of the string rather
    # than the value of the instance
    {
      name = "minLength";
      # Property: a schema with minLength m is a subtype of
      # a schema with minLength n iff m >= n
      cases = [
        {
          result = true;
          # len(x) >= 10
          sup = {minLength = 10;};
          # len(x) >= 15
          sub = {minLength = 15;};
        }
        {
          result = false;
          # len(x) >= 10
          sup = {minLength = 10;};
          # len(x) >= 5
          sub = {minLength = 5;};
        }
      ];
    }
    {
      name = "maxLength";
      # Property: a schema with maxLength m is a subtype of
      # a schema with maxLength n iff m <= n
      cases = [
        {
          result = true;
          # len(x) <= 10
          sup = {maxLength = 10;};
          # len(x) <= 5
          sub = {maxLength = 5;};
        }
        {
          result = false;
          # len(x) <= 10
          sup = {maxLength = 10;};
          # len(x) <= 15
          sub = {maxLength = 15;};
        }
      ];
    }
    {
      name = "minLength and maxLength together";
      cases = [
        {
          result = true;
          # 5 <= len(x) <= 15
          sup = {
            minLength = 5;
            maxLength = 15;
          };
          # 10 <= len(x) <= 12
          sub = {
            minLength = 10;
            maxLength = 12;
          };
        }
        {
          result = false;
          # 5 <= len(x) <= 15
          sup = {
            minLength = 5;
            maxLength = 15;
          };
          # len(x) >= 10
          sub = {minLength = 10;};
        }
      ];
    }
    {
      name = "unsatisfiable schemas";
      description = ''
        If both minLength and maxLength are specified, but minLength > maxLength,
        then the schema is unsatisfiable. This makes it the bottom type, so it is
        a subtype of every schema, including itself.
      '';
      cases = [
        {
          result = true;
          # the only types that are subtypes of bottom are bottom itself
          # so this test confirms that {minLength = 10; maxLength = 5;} is bottom
          sup = [{maxLength = 1;} false];
          sub = {
            minLength = 10;
            maxLength = 5;
          };
        }
      ];
    }
  ];
}

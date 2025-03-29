This is a test suite to evaluate the correctness of OXVG's optimisation presets.

Follows [svgcleaner's method](https://github.com/RazrFalcon/svgcleaner/blob/master/docs/testing_notes.rst)

# Tests

- extract [w3c](http://www.w3.org/Graphics/SVG/Test/20110816/archives/W3C_SVG_11_TestSuite.tar.gz) into `w3c/`
- extract [oxygen icons](https://www.archlinux.org/packages/extra/any/oxygen-icons-svg/) into `oxygen/`

```sh
# run tests
pnpm run test
```

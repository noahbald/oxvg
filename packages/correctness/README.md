This is a test suite to evaluate the correctness of OXVG's optimisation presets.

Follows [svgcleaner's method](https://github.com/RazrFalcon/svgcleaner/blob/master/docs/testing_notes.rst)

# Tests

- extract [w3c](http://www.w3.org/Graphics/SVG/Test/20110816/archives/W3C_SVG_11_TestSuite.tar.gz) into `w3c/`
- extract [oxygen icons](https://www.archlinux.org/packages/extra/any/oxygen-icons-svg/) into `oxygen/`

```sh
# run tests
pnpm run test
```

## Flaky Tests

There seems to be bugs in napi-rs that differ in rendering when compared to Chrome.

### True Positives

- w3c: svg/styling-css-04-f.svg - reason: nested selector lost by collapse_groups
- w3c: svg/struct-use-11-f.svg - reason: sibling selector lost by remove_empty_containers
- w3c: svg/styling-css-10-f.svg - reason: external css not loaded by inline_styles

### False Positives

- w3c: svg/styling-css-08-f.svg
- w3c: svg/struct-dom-12-b.svg
- w3c: svg/styling-pres-03-f.svg
- w3c: svg/styling-pres-04-f.svg
- w3c: svg/types-basic-02-f.svg
- oxygen: many - reason: https://github.com/Brooooooklyn/canvas/issues/1150

Each transform action appends to the selected elements' `transform` attribute. Has no effect on elements that don't accept `transform` as part of their [content-model](https://www.w3.org/TR/2011/REC-SVG11-20110816/attindex.html). Overwrites transforms with an `"inherit"` value. Applies transforms in SVG 1.1 format, including units.

The rotate action accepts an angle (in degrees) and x/y origin as floating-point numbers. If the `x`/`y` value are omitted, then the origin will be treated as `0`/`0`.

```sh
# Effects: History, Document
-rotate angle x y
```

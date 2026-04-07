Each transform action appends to the selected elements' `transform` attribute. Has no effect on elements that don't accept `transform` as part of their [content-model](https://www.w3.org/TR/2011/REC-SVG11-20110816/attindex.html). Overwrites transforms with an `"inherit"` value. Applies transforms in SVG 1.1 format, including units.

The matrix action accepts six floating-point numbers.

```sh
# Effects: History, Document
-matrix a b c d e f
```

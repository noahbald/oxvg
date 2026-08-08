For selected elements with path data, merges the paths by some operation. Each merge will be applied to the selections sequentially. Merges will be effective on the front-most element.

A selected element will be filtered if the element is not a `path` element, has no `d` attribute, or has children. The merge will keep only the front-most element's attributes, omitting any attributes applied to other elements.

```sh
# Effects: History, Document, Selection
-path-intersect
```

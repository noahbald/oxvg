Creates a new element and inserts it into the current selection. Selects the new elements. Has no effect if the parent doesn't accept the tag as a child of it's [content-model](https://svgwg.org/svg2-draft/eltindex.html).

```sh
# Effects: history, document, selection, tree
-insert-ns "<uri>" "<qual-name>"
# Alias
-create-element-ns "<uri>" "<qual-name>"
```

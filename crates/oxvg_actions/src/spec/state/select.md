Selects elements in the document by some given query by setting `oxvg:state > oxvg:selections`. Selections are always applied from the root element (i.e. the document).

A query can either be a CSS query-selector or a comma/space separated list of allocation ids.

```sh
# Effects: History, Selection
-select "<query>"
```

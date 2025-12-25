# FIXMEs

List of issues to fix, no particular order.

## Scrollbar shown unnecessarily in keys list

Even when there's nothing to scroll, we still show the scrollbar in keys list.
In value viewer it's also inconsistent for lists and z-sets, especially for ones
where items are multiline.

```
 Keys                        1 of 1,430
 ▸ doc:tag:stop                    set ▲
   doc:tag:story                   set █
   doc:tag:stock                   set █
   doc:tag:station                 set █
                                       │
                                       ▼
```

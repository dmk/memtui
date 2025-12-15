# FIXMEs

List of issues to fix, no particular order.

## Remote search results displayed incorrectly

The `blob:random` and `blob:text_mixed` are remote results, can not be clicked/selected.

```
 Keys │ blob▍                                           3 found
 ▸ blob:png_magic                                       string ▲
   ◦ blob:random                                        string █
   ◦ blob:text_mixed                                    string █
```

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

## Can scroll un-focused elements

E.g. can scroll keys list while value viewer is active.
Which is incorrect, as the keys list won't even re-render.


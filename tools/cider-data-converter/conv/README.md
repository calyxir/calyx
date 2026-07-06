
## vague goals:
- a limited number of conversions should be defined. conversions shouldn't be general by default, but can be made general.
  - ex: 'float to int' conversion should be defined for certain width pairs by default, and *possible* to expand to more widths, rather than the other way around.
- operations should be pretty explicit, and make certain types of granularity *possible* to express
  - ex: the rust 'as' keyword elides several aspects of behaviour.
    - 'as' can be a bitcast, a truncation, a value-preserving operation, or a rounding operation
    - not to say that 'as' doesn't cover the practical use-cases, but, for example, it should be possible to separate sign-extension from whether the underlying type is unsigned or not
- extension of 'operations being explicit' -- somewhat existing error detection / correction
  - i.e. not all strings are let through, impossible conversions should not fail silently
- discourage operations which take advantage of 'bad typing'
  - ex: trying to 'downcast' to bits / string at file level should be obvious, so when it fails it does so for a known reason
  - trying to 'downcast' to bits at IR level and reinterpreting as another type should be obvious.
  - essentially, want to avoid C-style type fudgery.
  - 'preventing' these mistakes is impossible, but they should at least be apparent.
- the binary is the 'source of truth', once data is loaded to bits they shouldn't get lost inadvertently
  - or, users should think of the bits first rather than 'value first': operations should have clear consequences in terms of bits
- relevant abstractions
  - memories are flattened, and can be 'reshaped' if demanded by the format. reduces weird iterator handling within main loop
  - defining an 'IR' rather than directly doing format -> format conversions: avoids considering ``(format * type)^2`` variants in one 'level'
  - adding new formats should be relatively easy. adding new data types should be possible and not Horrible, but not necessarily easier.
- reduce / minimise dynamic dispatch in critical path

## questions
- how to reduce badness / runtime inefficiency / etc. of everything in numimpl?
- should more checks be compile-time / 'inherent' to the defined types?
- thoughts on adding 'generalisation' for operations? how to go about it?

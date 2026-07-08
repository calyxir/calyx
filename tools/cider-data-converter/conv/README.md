
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

## notes on feature support / deficiencies
- endianness support is spotty and untested.
- abstractions for specific formats are weak and require user to do a lot. i wanted to make these far more structured so users would have to write less code, but alas.
- there is the semblance of support for 'nuanced' typecasts, but the functionality is unimplemented (see ``conv/src/cast.rs``)
  - future work will have to write the casts, add relevant option(s) to the CLI, and create structures which might be necessary.
- the ops defined on the ``SingleMem`` type are sign-extension, truncation, and zero-padded 'bitcasting'. the former two change the type of the ``SingleMem`` to ``Bits``, the latter changes to a user-supplied type.
  - this may limit how much is possible in a single invocation: sign-extension plus a bitcast has to be run through the CLI twice
- specifying operations between types is annoying:
  - the above operations don't really depend on the input / output ``TypeClass``.
  - specifying an operation between (for example) ints and floats requires checking the source's ``TypeClass`` and destination's ``TypeClass`` *somewhere* in the chain.
  - it'd be cool to avoid this, *maybe* by doing something at compile-time?
- operations are always 'checked' for validity before being performed. for performance, it may be worth making 'unchecked' functions which are faster but can panic / have weird behaviour
- the sim data directory backend is flawed
  - there's not a particularly good reason for this. the previous version of cider-data-converter borrowed components from ``CiderDataDump`` to implement its data directory operations.
  - however, this *does* mean that a cider-compatible header file describing the memories within the directory needs to be supplied. this obviously may not always exist.
- hex string support kind of doesn't exist
  - this means facilities for reading in a string like ``0x1234`` and writing out a number in a similar format.
  - reading-side: ``serde_json``, the primary text format, may not recognise such strings correctly. otherwise, the ``{type}_read`` functions in ``numimpl`` don't have explicit support for these. a new ``read`` function which reads exclusively hexstrings, and which checks length, should be implemented; and used in all the existing type-specific ``read`` functions to handle parsing hex.
- ``serde_json`` is probably not the right answer for the fud2 format.
  - the ``arbitrary`` feature flag is used so we get json numbers as 'strings', which we then parse using our own functions; rather letting ``serde`` handle it for us.
  - this is roundabout, and also might be brittle: does it break on hex strings? who knows!

## questions
- how to reduce badness / runtime inefficiency / etc. of everything in numimpl?
- should more checks be compile-time / 'inherent' to the defined types?
- thoughts on adding 'generalisation' for operations? how to go about it?

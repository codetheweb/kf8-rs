# Structure

A tag map is comprised of several parts:

- A set of tag definitions (may not be collocated with the values).
- A set of entries, where each entry has a set of values that map to a subset of the defined tags (usually each entry contains all the defined tags).

An entry has:

- One or more "control bytes" that describe the following values.
- A set of variable-width encoded values, in the same order as the set of tag definitions. There is no boundary between values, and they are layed out like: `tag0_value0, tag0_value1, tag1_value0, tag2_value0, ...`.


# Byte layout

A tag definition is a 3-byte structure:

- `tag`: the tag identifier (u8)
- `values_per_entry`: the number of values expected for this tag (u8). I'm not entirely sure how this field works, as it's possible to have a tag with `2 * values_per_entry` values as long as a correct mask is chosen (one of the pre-defined tag definitions works like this).
- `mask`: each tag definition in a given set needs to have a unique mask (u8)

`mask` must be chosen such that `mask & values_per_entry != mask`.

Currently, tests are run for each type of tag map rather than generating arbitrary tag maps for arbitrary tag definitions, as it's fairly non-trivial to randomly construct pairs that are possible to correctly serialize/deserialize under this spec with proptest.

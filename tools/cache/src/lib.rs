use calyx_writer::{
    build_cells, build_control, build_group, declare_group, Control,
    PortProvider, RRC,
};

/// I henceforth declare that a byte has 8 bits.
const BYTE_WIDTH: usize = 8;

/// The number of bits needed to represent `x` distinct values.
fn get_repr_bits(x: usize) -> usize {
    x.next_power_of_two().trailing_zeros() as usize
}

struct CacheLevel<'a> {
    name: &'a str,
    tag_bits: usize,
    index_bits: usize,
    k: usize,
    offset_bits: usize,
}

impl CacheLevel<'_> {
    fn name(&self) -> String {
        format!("cache_level_{}", self.name)
    }

    fn build(&self, address_width: usize, comp: RRC<calyx_writer::Component>) {
        let extra_bits = 2; // dirty + valid
        let block_size = 1 << self.offset_bits;
        let set_width = self.k * block_size * BYTE_WIDTH;
        let entry_width = extra_bits + self.tag_bits + set_width;
        let num_entries = 1 << self.index_bits;

        // bit position where index starts in address
        let addr_index_offset = self.offset_bits;

        // bit position where tag starts in address
        let addr_tag_offset = self.offset_bits + self.index_bits;

        let entry_tag_offset = set_width;

        for (name, width) in [
            ("read_en", 1),
            ("write_en", 1),
            ("in", BYTE_WIDTH),
            ("addr", address_width),
            ("fetch_en", 1),
            ("fetch_in", set_width),
        ] {
            comp.borrow_mut().add_input(name, width);
        }
        for (name, width) in
            [("out", BYTE_WIDTH), ("fetch_addr", address_width)]
        {
            comp.borrow_mut().add_output(name, width);
        }

        build_cells!(comp;
            entries = comb_mem_d1(entry_width, num_entries, self.index_bits);
            addr_to_index = std_bit_slice(address_width, addr_index_offset, addr_index_offset + self.index_bits, self.index_bits);
            addr_to_tag = std_bit_slice(address_width, addr_tag_offset, addr_tag_offset + self.tag_bits, self.tag_bits);
            entry_to_tag = std_bit_slice(entry_width, entry_tag_offset, entry_tag_offset + self.tag_bits, self.tag_bits);
            tag_matches = std_eq(self.tag_bits);
        );

        declare_group!(comp; comb group check_tag_matches: "checks if the given `addr` exists in the cache");
        build_group!(check_tag_matches;
            addr_to_index.in = comp.addr;
            addr_to_tag.in = comp.addr;
            entry_to_tag.in = entries.read_data;

            entries.addr0 = addr_to_index.out;
            tag_matches.left = addr_to_tag.out;
            tag_matches.right = entry_to_tag.out;
        );

        declare_group!(comp; group read_cached: "we've cached this, so we can just look it up");
        declare_group!(comp; group read_uncached: "we need to ask the level below for this data");

        let control = build_control!(
            [par {
                [if comp.read_en {
                    [if tag_matches.out with check_tag_matches {
                        [read_cached]
                    } else {
                        [read_uncached]
                    }]
                }],
                [if comp.write_en {

                }]
            }]
        );
        comp.borrow_mut().set_control(control);
    }
}

/// A hierarchical set-assocative cache in calyx.
pub struct Cache<'a> {
    address_width: usize,
    levels: Vec<CacheLevel<'a>>,
}

impl<'a> Cache<'a> {
    /// Begins a cache for a memory addressed by `address_width` bits. You
    /// *must* call [`Cache::add_level`] at least once before you call
    /// [`Cache::build`].
    pub fn for_memory(address_width: usize) -> Self {
        Self {
            address_width,
            levels: vec![],
        }
    }

    /// Adds a level below all current levels storing `size` bytes in sets of
    /// `k` blocks, each of which can store `block_size` bytes.
    ///
    /// Requires: `name` is a calyx identifier not already bound to an existing
    /// cache level, `size` is a power of two, and `k * block_size` divides
    /// `size`.
    pub fn add_level<'b: 'a>(
        mut self,
        name: &'b str,
        size: usize,
        k: usize,
        block_size: usize,
    ) -> Self {
        assert!(
            name.chars().next().map_or(true, |c| !c.is_numeric())
                && name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "invalid name '{}'",
            name
        );
        assert!(
            self.levels
                .iter()
                .find(|level| level.name == name)
                .is_none(),
            "there is already a level named '{}'",
            name
        );
        assert!(
            size.is_power_of_two(),
            "cache size {} is not a power of 2",
            size
        );

        let bytes_per_set = k * block_size;
        assert!(size % bytes_per_set == 0, "blocks cannot fit into size");

        let num_sets = size / bytes_per_set;

        let index_bits = get_repr_bits(num_sets);
        let offset_bits = get_repr_bits(block_size);
        let tag_bits = self.address_width - index_bits - offset_bits;

        self.levels.push(CacheLevel {
            name,
            tag_bits,
            index_bits,
            k,
            offset_bits,
        });
        self
    }

    /// Emits the calyx code for the cache.
    pub fn build(self) -> String {
        assert!(!self.levels.is_empty(), "no levels were added");
        let mut program = calyx_writer::Program::new();
        program.import("primitives/core.futil");
        program.import("primitives/memories/comb.futil");
        program.comp("main");

        for level in self.levels {
            level.build(self.address_width, program.comp(level.name()));
        }
        program.to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::get_repr_bits;

    #[test]
    fn test_get_repr_bits() {
        assert_eq!(0, get_repr_bits(0), "no values need no bits");
        assert_eq!(0, get_repr_bits(1), "1 value needs no bits");
        assert_eq!(1, get_repr_bits(2), "2 values need 1 bit");
        assert_eq!(2, get_repr_bits(3), "3 values need 2 bits");
        assert_eq!(2, get_repr_bits(4), "4 values need 2 bits");
        assert_eq!(4, get_repr_bits(16), "16 values need 4 bits");
        assert_eq!(10, get_repr_bits(1024), "1024 values need 10 bits");
    }
}

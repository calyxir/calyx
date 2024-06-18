use cache::Cache;

fn main() {
    const ADDRESS_WIDTH: usize = 16;
    let cache = Cache::for_memory(ADDRESS_WIDTH)
        .add_level("L1", 1024, 4, 4)
        .build();
    println!("{}", cache);
}

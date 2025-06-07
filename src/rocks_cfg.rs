use rocksdb::{Options, BlockBasedOptions, SliceTransform};

#[must_use] pub fn tuned() -> Options {
    let mut opts = Options::default();
    opts.set_max_open_files(-1);
    opts.optimize_for_point_lookup(8 * 1024 * 1024);

    let mut bb = BlockBasedOptions::default();
    bb.set_block_size(16 * 1024);
    bb.set_cache_index_and_filter_blocks(true);
    bb.set_pin_l0_filter_and_index_blocks_in_cache(true);
    bb.set_bloom_filter(10.0, false);           // <-- changed line
    opts.set_block_based_table_factory(&bb);

    opts.set_prefix_extractor(SliceTransform::create_fixed_prefix(11));
    opts.set_memtable_whole_key_filtering(true);

    opts.create_if_missing(true);
    opts
}

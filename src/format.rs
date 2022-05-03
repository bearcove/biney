use binary_layout::define_layout;

define_layout!(header, BigEndian, {
    flags: u16,
    num_records: u16,
});

define_layout!(record, BigEndian, {
    value: u64,
});

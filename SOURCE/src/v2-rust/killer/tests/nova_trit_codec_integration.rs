//! Integration tests: NOVT trit codec is **separate** from `nova_compress` / NOVZ.

use killer_native::nova_trit_codec::{
    pack_trits_novd, pack_trits_novt, unpack_novd_trits, unpack_novt_trits, NOVD_MAGIC, NOVT_MAGIC,
};

#[test]
fn novt_smaller_than_naive_i8_slice_meta() {
    let trits: Vec<i8> = vec![1, -1, 0, 1, 1, 0, -1, -1, 0, 1];
    let novt = pack_trits_novt(&trits).expect("pack");
    let naive = trits.len() + 8;
    assert!(
        novt.len() < naive,
        "NOVT should be smaller than a raw i8 dump with a minimal u64 length prefix"
    );
    assert_eq!(&novt[0..4], NOVT_MAGIC.as_slice());

    let novd = pack_trits_novd(&trits).expect("pack novd");
    assert_eq!(&novd[0..4], NOVD_MAGIC.as_slice());
    assert!(novd.len() <= novt.len());
    assert_eq!(unpack_novd_trits(&novd).expect("unpack novd"), trits);
}

#[test]
fn novt_round_trip_large_pattern() {
    let trits: Vec<i8> = (0..2000)
        .map(|i| match i % 3 {
            0 => -1i8,
            1 => 0,
            _ => 1,
        })
        .collect();
    let blob = pack_trits_novt(&trits).expect("pack");
    let out = unpack_novt_trits(&blob).expect("unpack");
    assert_eq!(out, trits);
}

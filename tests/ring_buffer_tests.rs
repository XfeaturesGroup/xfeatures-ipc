use xfeatures_ipc::IpcBuilder;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct DataItem {
    value: u32,
}

#[test]
fn test_ring_buffer_basic() {
    let (mut prod, mut cons) = IpcBuilder::<DataItem>::new("test_basic")
        .capacity(4)
        .build()
        .unwrap();

    let data_in = DataItem { value: 42 };
    assert!(prod.push(data_in).is_ok());

    let data_out = cons.pop().expect("Expected data to be present");
    assert_eq!(data_out.value, 42);
}

#[test]
fn test_ring_buffer_overflow() {
    let (mut prod, _cons) = IpcBuilder::<DataItem>::new("test_overflow")
        .capacity(2) // Must be power of 2
        .build()
        .unwrap();

    let data_in = DataItem { value: 99 };

    assert!(prod.push(data_in).is_ok());
    assert!(prod.push(data_in).is_ok());

    // Third push should fail (capacity is 2)
    assert!(prod.push(data_in).is_err());
}

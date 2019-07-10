use ebml::vint::{read_vint, write_vint};

#[test]
fn test_ebml_id() {
    let data = vec![
        (0, 1),
        (i64::pow(2, 7) - 1, 1),
        (i64::pow(2, 7), 2),
        (i64::pow(2, 14) - 1, 2),
        (i64::pow(2, 14), 3),
        (i64::pow(2, 21) - 1, 3),
        (i64::pow(2, 21), 4),
        (i64::pow(2, 28) - 1, 4),
    ];
    for (value, lenfth) in data {
        let id: ebml::ebml::EbmlId = value.into();
        let buf: Vec<u8> = id.into();
        let o = ebml::vint::read_vint(&buf, 0).unwrap().unwrap();
        assert_eq!(o.value, value);
        assert_eq!(o.length, lenfth);
    }
}

#[test]
fn test_read_vint() {
    // bits, big-endian
    // 1xxx xxxx                                                                              - value 0 to  2^7-2
    // 01xx xxxx  xxxx xxxx                                                                   - value 0 to 2^14-2
    // 001x xxxx  xxxx xxxx  xxxx xxxx                                                        - value 0 to 2^21-2
    // 0001 xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx                                             - value 0 to 2^28-2
    // 0000 1xxx  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx                                  - value 0 to 2^35-2
    // 0000 01xx  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx                       - value 0 to 2^42-2
    // 0000 001x  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx            - value 0 to 2^49-2
    // 0000 0001  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx  xxxx xxxx - value 0 to 2^56-2
    dotenv::dotenv().ok();
    env_logger::try_init().ok();
    // should read the correct value for 1 byte int min/max values
    {
        {
            let buf = vec![0b_1000_0000];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, 0);
            assert_eq!(vint.length as usize, buf.len());
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual 1 byte int max value
            let buf = vec![0b_1111_1110];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 7) - 2);
            assert_eq!(vint.length as usize, buf.len());
            // reserved id
            let buf = vec![0b_1111_1111];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 7) - 1);
            assert_eq!(vint.length as usize, buf.len());
            let buf = vec![0b_0100_0000, 0b_0111_1111];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 7) - 1);
            assert_eq!(vint.length as usize, buf.len());
        }
        // should read the correct value for 1 byte int min/max values
        for i in 0..0b_0010_0000_u8 {
            let buf = vec![i | 0b_1000_0000];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i as i64);
            assert_eq!(vint.length as usize, buf.len());
        }
        // should read the correct value for 1 byte int with non-zero start
        {
            let buf = vec![0b_0000_0000, 0b_0100_00001];
            let vint = read_vint(&buf, 1).unwrap().unwrap();
            assert_eq!(vint.value, 1);
            assert_eq!(vint.length, 1);
        }
    }
    // should read the correct value for 2 byte int min/max values
    {
        {
            let buf = vec![0b_0100_0000, 0b_1000_0000];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 7));
            assert_eq!(vint.length as usize, buf.len());
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual 2 byte int max value
            let buf = vec![0b_0111_1111, 0b_1111_1110];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 14) - 2);
            assert_eq!(vint.length as usize, buf.len());
            // reserved id
            let buf = vec![0b_0111_1111, 0b_1111_1111];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 14) - 1);
            assert_eq!(vint.length as usize, buf.len());
            let buf = vec![0b_0010_0000, 0b_0011_1111, 0b_1111_1111];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 14) - 1);
            assert_eq!(vint.length as usize, buf.len());
        }
        // should read the correct value for all 2 byte integers
        for i in 0..0b_0100_0000_u8 {
            for j in 0..0b_0111_11111_u8 {
                let buf = vec![i | 0b_0100_0000, j];
                let vint = read_vint(&buf, 0).unwrap().unwrap();
                assert_eq!(vint.value, (((i as usize) << 8) + (j as usize)) as i64);
                assert_eq!(vint.length as usize, buf.len());
            }
        }
    }
    // should read the correct value for 3 byte int min/max values
    {
        {
            let buf = vec![0b_0010_0000, 0b_0100_0000, 0b_0000_0000];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 14));
            assert_eq!(vint.length as usize, buf.len());
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual 3 byte int max value
            let buf = vec![0b_0011_1111, 0b_1111_1111, 0b_1111_1110];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 21) - 2);
            assert_eq!(vint.length as usize, buf.len());
            // reserved id
            let buf = vec![0b_0011_1111, 0b_1111_1111, 0b_1111_1111];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 21) - 1);
            assert_eq!(vint.length as usize, buf.len());
            let buf = vec![0b_0001_0000, 0b_000_11111, 0b_1111_1111, 0b_1111_1111];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 21) - 1);
            assert_eq!(vint.length as usize, buf.len());
        }
        // should read the correct value for all 3 byte integers
        for i in 0..0b_0010_0000_u8 {
            for j in 0..0b_1111_1111_u8 {
                for k in 0..0b_1111_1111_u8 {
                    let buf = vec![i | 0b_0010_0000, j, k];
                    let vint = read_vint(&buf, 0).unwrap().unwrap();
                    assert_eq!(
                        vint.value,
                        (((i as usize) << 16) + ((j as usize) << 8) + (k as usize)) as i64
                    );
                    assert_eq!(vint.length as usize, buf.len());
                }
            }
        }
    }
    // should read the correct value for 4 byte int min/max values
    {
        {
            let buf = vec![0b_0001_0000, 0b_0010_0000, 0b_0000_0000, 0b_0000_0000];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 21));
            assert_eq!(vint.length as usize, buf.len());
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual 4 byte int max value
            let buf = vec![0b_000_11111, 0b_1111_1111, 0b_1111_1111, 0b_1111_1110];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 28) - 2);
            // reserved id
            let buf = vec![0b_000_11111, 0b_1111_1111, 0b_1111_1111, 0b_1111_1111];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 28) - 1);
            assert_eq!(vint.length as usize, buf.len());
            let buf = vec![
                0b_0000_1000,
                0b_0000_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 28) - 1);
            assert_eq!(vint.length as usize, buf.len());
        }
    }
    // should read the correct value for 5 byte int min/max values
    {
        {
            let buf = vec![
                0b_0000_1000,
                0b_0001_0000,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 28));
            assert_eq!(vint.length as usize, buf.len());
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual 5 byte int max value
            let buf = vec![
                0b_0000_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1110,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 35) - 2);
            assert_eq!(vint.length as usize, buf.len());
            // reserved id
            let buf = vec![
                0b_0000_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 35) - 1);
            assert_eq!(vint.length as usize, buf.len());
            let buf = vec![
                0b_0000_0100,
                0b_0000_0111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 35) - 1);
            assert_eq!(vint.length as usize, buf.len());
        }
    }
    // should read the correct value for 6 byte int min/max values
    {
        {
            let buf = vec![
                0b_0000_0100,
                0b_0000_1000,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 35));
            assert_eq!(vint.length as usize, buf.len());
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual 6 byte int max value
            let buf = vec![
                0b_0000_0111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1110,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 42) - 2);
            assert_eq!(vint.length as usize, buf.len());
            // reserved id
            let buf = vec![
                0b_0000_0111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 42) - 1);
            assert_eq!(vint.length as usize, buf.len());
            let buf = vec![
                0b_0000_0010,
                0b_0000_0011,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 42) - 1);
            assert_eq!(vint.length as usize, buf.len());
        }
    }
    // should read the correct value for 7 byte int min/max values
    {
        {
            let buf = vec![
                0b_0000_0010,
                0b_0000_0100,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 42));
            assert_eq!(vint.length as usize, buf.len());
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual 7 byte int max value
            let buf = vec![
                0b_0000_0011,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1110,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 49) - 2);
            assert_eq!(vint.length as usize, buf.len());
            // reserved id
            let buf = vec![
                0b_0000_0011,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 49) - 1);
            assert_eq!(vint.length as usize, buf.len());
            let buf = vec![
                0b_0000_0001,
                0b_0000_0001,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 49) - 1);
            assert_eq!(vint.length as usize, buf.len());
        }
    }
    // should read the correct value for 8 byte int min/max values
    {
        {
            let buf = vec![
                0b_0000_0001,
                0b_0000_0010,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
                0b_0000_0000,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 49));
            assert_eq!(vint.length as usize, buf.len());
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual 8 byte int max value
            let buf = vec![
                0b_0000_0001,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1110,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 56) - 2);
            assert_eq!(vint.length as usize, buf.len());
            // reserved id
            let buf = vec![
                0b_0000_0001,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            let vint = read_vint(&buf, 0).unwrap().unwrap();
            assert_eq!(vint.value, i64::pow(2, 56) - 1);
            assert_eq!(vint.length as usize, buf.len());
            // out of range
            let buf = vec![
                0b_0000_0000,
                0b_1000_0000,
                0b_0111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
                0b_1111_1111,
            ];
            assert!(read_vint(&buf, 0).is_err());
        }
    }
    // should throw for 9+ byte int values
    {
        let buf = vec![
            0b_0000_0000,
            0b_1000_0000,
            0b_0000_0000,
            0b_0000_0000,
            0b_0000_0000,
            0b_0000_0000,
            0b_0000_0000,
            0b_0000_0000,
            0b_0000_0000,
        ];
        let maybe_err = read_vint(&buf, 0);
        assert!(maybe_err.is_err());
    }
}

#[test]
fn test_write_vint() {
    dotenv::dotenv().ok();
    env_logger::try_init().ok();
    // should throw when writing -1
    {
        assert!(write_vint(-1).is_err());
    }
    // should write 1 byte int min/max values
    {
        {
            let buf = write_vint(0).unwrap();
            assert_eq!(buf, vec![0b_1000_0000]);
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual max
            let buf = write_vint(i64::pow(2, 7) - 2).unwrap();
            assert_eq!(buf, vec![0b_1111_1110]);
            // reserved id
            let buf = write_vint(i64::pow(2, 7) - 1).unwrap();
            assert_ne!(buf, vec![0b_1111_1111]);
            assert_eq!(buf, vec![0b_0100_0000, 0b_0111_1111]);
        }
    }
    // should write 2 byte int min/max values
    {
        {
            let buf = write_vint(i64::pow(2, 7)).unwrap();
            assert_eq!(buf, vec![0b_0100_0000, 0b_1000_0000]);
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual max
            let buf = write_vint(i64::pow(2, 14) - 2).unwrap();
            assert_eq!(buf, vec![0b_0111_1111, 0b_1111_1110]);
            // reserved id
            let buf = write_vint(i64::pow(2, 14) - 1).unwrap();
            assert_ne!(buf, vec![0b_0111_1111, 0b_1111_1111]);
            assert_eq!(buf, vec![0b_0010_0000, 0b_0011_1111, 0b_1111_1111]);
        }
    }
    // should write 3 byte int min/max values
    {
        {
            let buf = write_vint(i64::pow(2, 14)).unwrap();
            assert_eq!(buf, vec![0b_0010_0000, 0b_0100_0000, 0b_0000_0000]);
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual max
            let buf = write_vint(i64::pow(2, 21) - 2).unwrap();
            assert_eq!(buf, vec![0b_0011_1111, 0b_1111_1111, 0b_1111_1110]);
            // reserved id
            let buf = write_vint(i64::pow(2, 21) - 1).unwrap();
            assert_ne!(buf, vec![0b_0011_1111, 0b_1111_1111, 0b_1111_1111]);
            assert_eq!(
                buf,
                vec![0b_0001_0000, 0b_000_11111, 0b_1111_1111, 0b_1111_1111]
            );
        }
    }
    // should write 4 byte int min/max values
    {
        {
            let buf = write_vint(i64::pow(2, 21)).unwrap();
            assert_eq!(
                buf,
                vec![0b_0001_0000, 0b_0010_0000, 0b_0000_0000, 0b_0000_0000]
            );
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual max
            let buf = write_vint(i64::pow(2, 28) - 2).unwrap();
            assert_eq!(
                buf,
                vec![0b_000_11111, 0b_1111_1111, 0b_1111_1111, 0b_1111_1110]
            );
            // reserved id
            let buf = write_vint(i64::pow(2, 28) - 1).unwrap();
            assert_ne!(
                buf,
                vec![0b_000_11111, 0b_1111_1111, 0b_1111_1111, 0b_1111_1111]
            );
            assert_eq!(
                buf,
                vec![
                    0b_0000_1000,
                    0b_0000_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111
                ]
            );
        }
    }
    // should write 5 byte int min/max values
    {
        {
            let buf = write_vint(i64::pow(2, 28)).unwrap();
            assert_eq!(
                buf,
                vec![
                    0b_0000_1000,
                    0b_0001_0000,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000
                ]
            );
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual max
            let buf = write_vint(i64::pow(2, 35) - 2).unwrap();
            assert_eq!(
                buf,
                vec![
                    0b_0000_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1110
                ]
            );
            // reserved id
            let buf = write_vint(i64::pow(2, 35) - 1).unwrap();
            assert_ne!(
                buf,
                vec![
                    0b_0000_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111
                ]
            );
            assert_eq!(
                buf,
                vec![
                    0b_0000_0100,
                    0b_0000_0111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111
                ]
            );
        }
    }
    // should write 6 byte int min/max values
    {
        {
            let buf = write_vint(i64::pow(2, 35)).unwrap();
            assert_eq!(
                buf,
                vec![
                    0b_0000_0100,
                    0b_0000_1000,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000
                ]
            );
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual max
            let buf = write_vint(i64::pow(2, 42) - 2).unwrap();
            assert_eq!(
                buf,
                vec![
                    0b_0000_0111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1110
                ]
            );
            // reserved id
            let buf = write_vint(i64::pow(2, 42) - 1).unwrap();
            assert_ne!(
                buf,
                vec![
                    0b_0000_0111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111
                ]
            );
            assert_eq!(
                buf,
                vec![
                    0b_0000_0010,
                    0b_0000_0011,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111
                ]
            );
        }
    }
    // should write 7 byte int min/max values
    {
        {
            let buf = write_vint(i64::pow(2, 42)).unwrap();
            assert_eq!(
                buf,
                vec![
                    0b_0000_0010,
                    0b_000_00100,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000
                ]
            );
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual max
            let buf = write_vint(i64::pow(2, 49) - 2).unwrap();
            assert_eq!(
                buf,
                vec![
                    0b_0000_0011,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1110
                ]
            );
            // reserved id
            let buf = write_vint(i64::pow(2, 49) - 1).unwrap();
            assert_ne!(
                buf,
                vec![
                    0b_0000_0011,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111
                ]
            );
            assert_eq!(
                buf,
                vec![
                    0b_0000_0001,
                    0b_0000_0001,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111
                ]
            );
        }
    }
    // should write the correct value for 8 byte int min value
    {
        {
            let buf = write_vint(i64::pow(2, 49)).unwrap();
            assert_eq!(
                buf,
                vec![
                    0b_0000_0001,
                    0b_0000_0010,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000,
                    0b_0000_0000
                ]
            );
        }
        {
            // https://github.com/node-ebml/node-ebml/pull/14
            // actual max
            let buf = write_vint(i64::pow(2, 56) - 2).unwrap();
            assert_eq!(
                buf,
                vec![
                    0b_0000_0001,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1111,
                    0b_1111_1110
                ]
            );
            // out of range
            assert!(write_vint(i64::pow(2, 56) - 1).is_err());
        }
    }
}

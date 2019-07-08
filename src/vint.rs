use err_derive::Error;
use log_derive::{logfn, logfn_inputs};

#[derive(Debug, Error)]
pub enum ReadVintError {
    #[error(display = "{}", _0)]
    UnrepresentableLength(#[error(cause)] UnrepresentableLengthError),
    #[error(display = "NeedMoreBuffer")]
    NeedMoreBuffer(#[error(cause)] std::io::Error),
}

#[derive(Debug, Error)]
#[error(display = "unrepresentable length: {}", length)]
pub struct UnrepresentableLengthError {
    length: u8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vint {
    pub length: u8,
    pub value: i64,
}

pub trait ReadVintExt: std::io::Read {
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn read_vint(&mut self) -> Result<Vint, ReadVintError> {
        use byteorder::ReadBytesExt as _;
        use ReadVintError::*;
        let start_byte = self.read_u8().map_err(NeedMoreBuffer)?;
        if start_byte == 0b_0000_0000 {
            // 9+ byte int values
            return Err(UnrepresentableLength(UnrepresentableLengthError {
                length: 9,
            }));
        }
        let length: u8 = 8 - log_2(u32::from(start_byte));
        if length > 8 {
            return Err(UnrepresentableLength(UnrepresentableLengthError { length }));
        }
        let mut buffer = vec![0; length as usize - 1];
        self.read_exact(&mut buffer).map_err(NeedMoreBuffer)?;
        let mut value = i64::from(start_byte & ((1 << (8 - length)) - 1));
        for i in 0..length - 1 {
            value *= i64::pow(2, 8);
            value += i64::from(buffer[i as usize]);
        }
        Ok(Vint { length, value })
    }
}

impl<R: std::io::Read + ?Sized> ReadVintExt for R {}

/// https://www.matroska.org/technical/specs/index.html#EBML_ex
#[logfn_inputs(TRACE)]
#[logfn(ok = "TRACE", err = "ERROR")]
pub fn read_vint(buffer: &[u8], start: usize) -> Result<Option<Vint>, UnrepresentableLengthError> {
    let mut o = std::io::Cursor::new(buffer);
    o.set_position(start as u64);
    match o.read_vint() {
        Ok(o) => Ok(Some(o)),
        Err(ReadVintError::NeedMoreBuffer(_)) => Ok(None),
        Err(ReadVintError::UnrepresentableLength(err)) => Err(err),
    }
}

// https://users.rust-lang.org/t/logarithm-of-integers/8506
const fn num_bits<T>() -> usize {
    std::mem::size_of::<T>() * 8
}
fn log_2(x: u32) -> u8 {
    assert!(x > 0);
    (num_bits::<u32>() as u32 - x.leading_zeros() - 1) as u8
}

#[derive(Debug, Error)]
pub enum WriteVintError {
    #[error(display = "{}", _0)]
    UnrepresentableValue(#[error(cause)] UnrepresentableValueError),
    #[error(display = "Io")]
    Io(#[error(cause)] std::io::Error),
}

#[derive(Debug, Error)]
#[error(display = "unrepresentable value: {}", value)]
pub struct UnrepresentableValueError {
    value: i64,
}

pub trait WriteVintExt: std::io::Write {
    #[logfn(ok = "TRACE", err = "ERROR")]
    fn write_vint(&mut self, value: i64) -> Result<(), WriteVintError> {
        if value < 0 || i64::pow(2, 56) - 2 < value {
            return Err(WriteVintError::UnrepresentableValue(
                UnrepresentableValueError { value },
            ));
        }
        let mut length = 1;
        for i in 1..=8 {
            // https://github.com/node-ebml/node-ebml/pull/14
            if value < i64::pow(2, 7 * i) - 1 {
                length = i;
                break;
            }
        }
        let mut buffer: Vec<u8> = vec![0; length as usize];
        let mut val = value;
        for i in 1..=length {
            #[allow(clippy::identity_op)]
            let b = (val as u8) & 0b_1111_1111;
            buffer[(length as usize) - (i as usize)] = b;
            val -= i64::from(b);
            val /= i64::pow(2, 8);
        }
        buffer[0] |= 1 << (8 - length);
        self.write_all(&buffer).map_err(WriteVintError::Io)?;
        Ok(())
    }
}

impl<R: std::io::Write + ?Sized> WriteVintExt for R {}

#[logfn_inputs(TRACE)]
#[logfn(ok = "TRACE", err = "ERROR")]
pub fn write_vint(value: i64) -> Result<Vec<u8>, UnrepresentableValueError> {
    let mut buf = vec![];
    match buf.write_vint(value) {
        Ok(()) => Ok(buf),
        Err(WriteVintError::UnrepresentableValue(err)) => Err(err),
        res => {
            res.unwrap();
            unreachable!()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_read_vint() {
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
    fn test_log_2() {
        dotenv::dotenv().ok();
        env_logger::try_init().ok();
        assert_eq!(7, log_2(255));
        assert_eq!(7, log_2(128));
        assert_eq!(6, log_2(127));
        assert_eq!(6, log_2(64));
        assert_eq!(5, log_2(63));
        assert_eq!(5, log_2(32));
        assert_eq!(4, log_2(31));
        assert_eq!(4, log_2(16));
        assert_eq!(3, log_2(15));
        assert_eq!(3, log_2(8));
        assert_eq!(2, log_2(7));
        assert_eq!(2, log_2(4));
        assert_eq!(1, log_2(3));
        assert_eq!(1, log_2(2));
        assert_eq!(0, log_2(1));
        // assert_eq!(0, log_2(0)); // assertion error
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
}

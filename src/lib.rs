//! # linux_max6675
//!
//! A library that helps you read from a MAX6675 over Linux SPI.
//!
//! ## Usage
//!
//! To use this library, you'll need to know which SPI device to select.
//! On Linux, you can use `ls /dev -1 | grep spidev` to figure it out!
//!
//! Then, you can use something like this example in your binary...
//!
//! ```no_run
//!
//! use rppal::spi::{ Spi, Bus, SlaveSelect, Mode };
//! use std::time::Duration;
//!
//! let mut tc = Spi::new(
//!     Bus::Spi0,
//!     SlaveSelect::Ss0,
//!     1_000_000,
//!     Mode::Mode1
//! ).unwrap();
//!
//! std::thread::sleep(Duration::from_secs(3));
//!
//! loop {
//!     let celsius = linux_max6675::read_celsius(&mut tc).unwrap();
//!     println!("Read Celsius! Got: {}° C.", celsius);
//!     std::thread::sleep(Duration::from_millis(500));
//! };
//!
//! ```

use rppal::spi::Spi;
use thiserror::Error;

/// An error emitted due to problems with the MAX6675.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Error using the provided SPI. See rppal::spi::Error: {source}")]
    SPI {
        #[from]
        source: rppal::spi::Error,
    },
    #[error("The MAX6675 detected an open circuit (bit D2 was high). Please check the thermocouple connection and try again.")]
    OpenCircuit,
    #[error("The SPI bus received nothing. Please check your SPI bus and CS and try again.")]
    ReceivedNothing,
}

/// Tries to return the thermocouple's raw data for data science. (and other fun little things)
///
/// Only fails if there's something wrong with the SPI connection.
///
/// Refer to page 5 of [Maxim Integrated's MAX6675 specsheet](https://www.analog.com/media/en/technical-documentation/data-sheets/MAX6675.pdf)
/// for info on how to interpret this raw data.
pub fn read(spi: &mut Spi) -> Result<u16, Error> {
    // Create 2 bytes buffer
    let mut buf = [0_u8; 2];
    // Read bytes from SPI
    let len = spi.read(&mut buf)?;
    if len == 2 {
        // Return bytes as u16
        Ok(u16::from_be_bytes(buf))
    } else {
        // No bytes read
        Err(Error::ReceivedNothing)
    }
}

/// Check if MAX6675 terminals are open.
///
/// This only works if -T terminal is grounded.
///
/// Check for Bit D2 being high, indicating that the thermocouple input is open
/// (see MAX6675 datasheet, p. 5)
///
/// ## Example
///
/// ```no_run
///
/// use rppal::spi::{ Spi, Bus, SlaveSelect, Mode };
///
/// let mut tc = Spi::new(
///     Bus::Spi0,
///     SlaveSelect::Ss0,
///     1_000_000,
///     Mode::Mode1
/// ).unwrap();
///
/// let bytes = linux_max6675::read(&mut tc).unwrap();
///
/// if linux_max6675::is_open(bytes) {
///     println("thermocouple is open!")
/// };
///
/// ````
pub fn is_open(bytes: u16) -> bool {
   (bytes & 0x04) != 0
}

/// Parse temperature from bytes
///
/// Extracts 12 bit integer from D14-D3 and multiply it by 1/4 precision factor
/// (see MAX6675 datasheet, p. 5)
pub fn parse_celsius(bytes: u16) -> f64 {
    ((0x1FFF & (bytes >> 3)) as f64) * 0.25
}

/// Tries to read the thermocouple's temperature in Celsius.
///
/// ## Example
///
/// ```no_run
///
/// use rppal::spi::{ Spi, Bus, SlaveSelect, Mode };
///
/// let mut tc = Spi::new(
///     Bus::Spi0,
///     SlaveSelect::Ss0,
///     1_000_000,
///     Mode::Mode1
/// ).unwrap();
///
/// let celsius = linux_max6675::read_celsius(&mut tc).unwrap();
///
/// println!("it's {}° celsius in here!", celsius);
///
/// ```
pub fn read_celsius(spi: &mut Spi) -> Result<f64, Error> {
    // Read bytes from SPI
    let bytes = read(spi)?;
    // Check if MAX6675 terminals are open
    is_open(bytes)
        .then(|| Err(Error::OpenCircuit))
        .map_or(Ok(()), |e| e)?;
    // Parse temperature from bytes
    Ok(parse_celsius(bytes))
}

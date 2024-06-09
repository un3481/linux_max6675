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
//! fn main() -> anyhow::Result<()> {
//!     use linux_max6675::Max6675;
//!     use std::time::Duration;
//!
//!     let mut max = Max6675::new("/dev/spidev0.0")?;
//!
//!     std::thread::sleep(Duration::from_secs(3));
//!
//!     loop {
//!         println!("Read Celsius! Got: {}° C.", max.read_celsius()?);
//!         std::thread::sleep(Duration::from_millis(500));
//!     }
//! }
//! ```

use std::io::Read;
use spidev::{ Spidev, SpidevOptions, SpiModeFlags };
use thiserror::Error;

/// An error emitted due to problems with the MAX6675.
#[derive(Debug, Error)]
pub enum Max6675Error {
    #[error("Couldn't connect to the provided SPI path. See std::io::Error: {source}")]
    IoError {
        #[from]
        source: std::io::Error,
    },
    #[error("The MAX6675 detected an open circuit (bit D2 was high). Please check the thermocouple connection and try again.")]
    OpenCircuitError,
    #[error("The SPI connection to the MAX6675 has not been completed, plese run `connect()` and try again.")] 
    SpiUninitialized,
}

/// SPI options for connecting to MAX6675
pub const SPI_OPTIONS: SpidevOptions = SpidevOptions {
    bits_per_word: Some(8),
    max_speed_hz: Some(1_000_000),
    lsb_first: None,
    spi_mode: Some(SpiModeFlags::SPI_MODE_1),
};

/// Tries to create a new `Max6675` based on the given SPI path.
/// A valid SPI path usually looks like `/dev/spidev0.0`.
///
/// Only fails if there's something wrong with the SPI connection.
///
/// ## Example
///
/// ```no_run
///
/// let mut tc = linux_max6675::open("/dev/spidev0.0").unwrap();
/// let bytes = linux_max6675::read_bytes(&mut tc).unwrap();
/// 
/// if linux_max6675::is_open(bytes) {
///     println("thermocouple is open!")
/// };
///
/// ````
pub fn open(path: impl AsRef<str>) -> Result<Spidev, Max6675Error> {
    // Open SPI connection
    let mut spi = Spidev::open(path.as_ref())?;
    // Configure SPI for MAX6675
    spi.configure(&SPI_OPTIONS)?;
    // Return SPI connection
    Ok(spi)
}

/// Tries to return the thermocouple's raw data for data science. (and other fun little things)
///
/// Refer to page 5 of [Maxim Integrated's MAX6675 specsheet](https://www.analog.com/media/en/technical-documentation/data-sheets/MAX6675.pdf)
/// for info on how to interpret this raw data.
///
/// ## Example
///
/// ```no_run
///
/// let mut tc = linux_max6675::open("/dev/spidev0.0").unwrap();     
/// let bytes = linux_max6675::read_bytes(&mut tc).unwrap();
///
/// println!("oOoo here's my favorite bytes: {}", bytes);
///     
/// ```
pub fn read_bytes(spi: &mut Spidev) -> Result<u16, Max6675Error> {
    // Create 2 bytes buffer
    let mut buf = [0_u8; 2];
    // Read bytes from SPI
    spi.read_exact(&mut buf)?;
    // Return bytes as u16
    Ok(u16::from_be_bytes(buf))
}

/// Check if MAX6675 terminals are open.
///
/// This only works if -T terminal is grounded.
///
/// Check for Bit D2 being high, indicating that the thermocouple input is open
/// (see MAX6675 datasheet, p. 5)
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
/// let mut tc = linux_max6675::open("/dev/spidev0.0").unwrap();     
/// let celsius = linux_max6675::read_celsius(&mut tc).unwrap();
/// 
/// println!("it's {}° celsius in here!", celsius);
///     
/// ```
pub fn read_celsius(spi: &mut Spidev) -> Result<f64, Max6675Error> {
    // Read bytes from SPI
    let bytes = read_bytes(spi)?;
    // Check if MAX6675 terminals are open
    is_open(bytes)
        .then(|| Err(Max6675Error::OpenCircuitError))
        .map_or(Ok(()), |e| e)?;
    // Parse temperature from bytes
    Ok(parse_celsius(bytes))
}

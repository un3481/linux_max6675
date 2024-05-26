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

/// A representation of the MAX6675 thermocouple digitizer.
#[derive(Debug)]
pub struct Max6675 {
    pub spi: Option<Spidev>
}

impl Max6675 {
    /// Tries to create a new `Max6675` based on the given SPI path.
    /// A valid SPI path usually looks like `/dev/spidev0.0`.
    ///
    /// ## Example
    ///
    /// ```
    /// use linux_max6675::Max6675;
    /// use std::sync::Mutex;
    /// use anyhow::Result;
    ///
    /// static TC1: Mutex<Max6675> = Mutex::new(Max6675::new());
    ///
    /// fn main() -> Result<()> {
    ///     let tc = TC1.lock()
    ///         .unwrap();
    ///
    ///     (*tc).connect("/dev/spidev0.0")?;
    ///     if (*tc).is_open()? {
    ///         println("thermocouple is open!")
    ///     };
    ///
    ///     Ok(())
    /// }
    /// ````
    pub const fn new() -> Self {
        Self { spi: None }
    }

    /// Tries to create a new `Spidev` connection.
    /// Only fails if there's something wrong with the SPI connection.
    pub fn connect(&mut self, spi_path: impl AsRef<str>) -> Result<&mut Self, Max6675Error> {
        // Open SPI connection
        let mut spi = Spidev::open(spi_path.as_ref())?;
        let options = SpidevOptions::new()
            .bits_per_word(8)
            .max_speed_hz(1_000_000)
            .mode(SpiModeFlags::SPI_MODE_1)
            .build();
        spi.configure(&options)?;
        // Store the connection
        self.spi = Some(spi);
        Ok(self)
    }

    /// Checks if SPI connection is initialized
    pub fn is_initialized(&mut self) -> bool {
        self.spi.is_some()
    }

    /// Tries to return the thermocouple's raw data for data science. (and other fun little things)
    ///
    /// Refer to page 5 of [Maxim Integrated's MAX6675 specsheet](https://www.analog.com/media/en/technical-documentation/data-sheets/MAX6675.pdf)
    /// for info on how to interpret this raw data.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use linux_max6675::Max6675;
    /// use anyhow::Result;
    ///
    /// fn main() -> Result<()> {
    ///     let tc = Max6675::new()
    ///         .connect("/dev/spidev0.0")?
    ///         .deref();
    ///     
    ///     let bytes = tc.read_bytes()?;
    ///     println!("oOoo here's my favorite bytes: {}", bytes);
    ///     
    ///     Ok(())
    /// }
    /// ````
    pub fn read_bytes(&mut self) -> Result<u16, Max6675Error> {
        // Read MISO bytes into buffer
        let mut buf = [0_u8; 2];
        self.spi
            .ok_or(Max6675Error::SpiUninitialized)?
            .read_exact(&mut buf)?;
        // Convert buffer into word
        Ok(u16::from_be_bytes(buf))
    }

    /// Check if thermocouple input is open (-T must be grounded).
    pub fn is_open(&mut self) -> Result<bool, Max6675Error> {
        // Read MISO bytes
        let bytes = self.read_bytes()?;
        // Check for Bit D2 being high, indicating that the thermocouple input is open
        // (see MAX6675 datasheet, p. 5)
        Ok((bytes & 0x04) != 0)
    }

    /// Tries to read the thermocouple's temperature in Celsius.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// use linux_max6675::Max6675;
    /// use simmer::Temperature;
    /// use anyhow::Result;
    ///
    /// fn main() -> Result<()> {
    ///     let tc = Max6675::new()
    ///         .connect("/dev/spidev0.0")?
    ///         .deref();
    ///     
    ///     let celsius = tc.read_celsius()?;
    ///     println!("it's {}° celsius in here!", celsius);
    ///     
    ///     let kelvin = tc.read_celsius()
    ///         .map(|temp| Temperature::Celsius(temp))?
    ///         .to_kelvin();
    ///     println!("it's {}° kelvin in here!", kelvin);
    ///     
    ///     Ok(())
    /// }
    /// ```
    pub fn read_celsius(&mut self) -> Result<f64, Max6675Error> {
        // Read MISO bytes
        let bytes = self.read_bytes()?;
        // Check for input open
        ((bytes & 0x04) != 0)
            .then(|| Err(Max6675Error::OpenCircuitError))
            .map_or(Ok(()), |e| e)?;
        // Extract 12 bit integer from D14-D3 and multiply it by 1/4 precision factor
        // (see MAX6675 datasheet, p. 5)
        Ok((0x1FFF & (bytes >> 3)) as f64) * 0.25)
    }
}

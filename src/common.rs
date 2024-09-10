use std::{
    fs::File,
    io::{Read, Seek, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpStream},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{anyhow, bail};
use snow::StatelessTransportState;

use crate::{
    message::NoiseHeader,
    types::{BlockIndex, FileChecksums, FileSize},
};

pub static BINCODE_CONFIG: bincode::config::Configuration<
    bincode::config::BigEndian,
    bincode::config::Fixint,
> = bincode::config::standard()
    .with_big_endian()
    .with_fixed_int_encoding();

pub static NOISE_PATTERN: &str = "Noise_XXpsk3_25519_ChaChaPoly_BLAKE2s";

pub static DEFAULT_SECRET: &[u8; 32] = &[
    0xe3, 0x5b, 0x0f, 0x9b, 0x64, 0x15, 0x6b, 0x84, 0xc9, 0xa2, 0x7a, 0x42, 0x74, 0x62, 0xf8, 0xff,
    0x25, 0x48, 0xdb, 0x99, 0xec, 0x04, 0x6e, 0x5d, 0xf7, 0x53, 0x3d, 0xdd, 0x60, 0x1d, 0xa2, 0x79,
];

pub const BLOCK_SIZE: u16 = 1024;

pub fn transcript_warn_error(result: anyhow::Result<()>) {
    if let Err(err) = result {
        println!("Unable to perform transcript: {err}");
    }
}

pub fn load_secret(path: &Option<PathBuf>, dest: &mut [u8; 32]) {
    match path {
        Some(path) => {
            if load_secret_internal(path, dest).is_err() {
                *dest = *DEFAULT_SECRET;
            }
        }
        None => *dest = *DEFAULT_SECRET,
    }
}

fn load_secret_internal(path: &Path, dest: &mut [u8; 32]) -> anyhow::Result<()> {
    let mut file = File::open(path)?;
    file.read_exact(dest)?;
    Ok(())
}

/// Returns the number of microseconds that have passed since the given `Instant`.
///
/// # Panics
/// Panics if the number of microseconds would not fit into a `u64`.
#[must_use]
pub fn get_µs_since(old_time: Instant) -> u64 {
    let now = Instant::now();
    now.duration_since(old_time)
        .as_micros()
        .try_into()
        .expect("microseconds 64 bit overflow")
}

/// Returns the `Duration` since the Unix epoch.
#[must_use]
pub fn epoch() -> Duration {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
}

/// Returns the IPv6 or IPv4 universal bind host (e.g. 0.0.0.0 for IPv4) depending on the given
/// parameter.
#[must_use]
pub fn catch_all_host(ipv6: bool) -> IpAddr {
    if ipv6 {
        IpAddr::V6(Ipv6Addr::UNSPECIFIED)
    } else {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    }
}

#[must_use]
pub fn make_transcript_filename(extension: &str) -> String {
    let seconds = epoch().as_secs();
    format!("{seconds}.{extension}")
}

#[must_use]
pub fn prepare_proof(buffer: &mut [u8], secret: &[u8]) -> md5::Digest {
    for (offset, fresh0) in buffer.iter_mut().enumerate() {
        *fresh0 ^= secret[offset.rem_euclid(secret.len())];
    }
    md5::compute(buffer)
}

/// Sleeps for the given number of microseconds.
pub fn µsleep_that_works(µs: u64) {
    std::thread::sleep(Duration::from_micros(µs));
}

/// Returns the UDP `InErrors` value from `/proc/net/snmp` on Linux, which quantifies the number of
/// UDP packets that were lost at OS level.
///
/// # Errors
/// Returns an error if the value could not be obtained for whatever reason, such as being on an
/// operating system that does not support this method of obtaining the UDP input error count.
pub fn get_udp_in_errors() -> anyhow::Result<u64> {
    let snmp_string = std::fs::read_to_string("/proc/net/snmp")?;

    let mut lines = snmp_string.lines().filter(|line| line.starts_with("Udp: "));

    let first_udp_line = lines
        .next()
        .ok_or_else(|| anyhow!("Could not find UDP line"))?;
    let second_udp_line = lines
        .next()
        .ok_or_else(|| anyhow!("Could not find second UDP line"))?;

    let in_errors_pos = first_udp_line
        .split(' ')
        .position(|element| element == "InErrors")
        .ok_or_else(|| anyhow!("Could not find InErrors in first UDP line"))?;
    let in_errors_value_str = second_udp_line
        .split(' ')
        .nth(in_errors_pos)
        .ok_or_else(|| anyhow!("Second UDP line does not have enough values"))?;
    let in_errors_value: u64 = in_errors_value_str.parse()?;

    Ok(in_errors_value)
}

/// Determine the amount of blocks each chunk of a file with the given size should contain.
///
/// # Panics
/// Panics if the block size is 0.
#[must_use]
pub fn chunk_blocks(file_size: FileSize) -> u64 {
    let chunk_size = file_size.0 >> 8; // use 128-256 chunks per file
    chunk_size
        .checked_div(u64::from(BLOCK_SIZE))
        .expect("block size is 0")
}

/// Calculate the chunk-wise checksum for the data in the given file. Returns one checksum value
/// for each chunk.
///
/// # Errors
/// Returns an error on file I/O failure.
///
/// # Panics
/// Panics on arithmetic overflow.
pub fn calculate_checksums(
    file: &mut File,
    file_size: FileSize,
    block_count: BlockIndex,
    chunk_blocks: u64,
) -> anyhow::Result<FileChecksums> {
    let chunk_size = u64::from(BLOCK_SIZE)
        .checked_mul(chunk_blocks)
        .expect("chunk size overflow #1");
    let chunk_size_usize: usize = chunk_size.try_into().expect("chunk size overflow #2");
    let num_chunks = file_size
        .0
        .checked_div(chunk_size)
        .expect("chunk size is 0");
    let last_chunk_blocks = u64::from(block_count.0)
        .checked_rem(chunk_blocks)
        .expect("chunk_blocks is 0");
    let mut checksums: Vec<u64> = Vec::with_capacity(
        num_chunks
            .checked_add(1)
            .expect("capacity overflow #1")
            .try_into()
            .expect("capacity overflow #2"),
    );
    let mut data_buffer = vec![0_u8; chunk_size_usize];

    for i in 0..=num_chunks {
        let start_pos = i.checked_mul(chunk_size).expect("start pos overflow");
        file.seek(std::io::SeekFrom::Start(start_pos))?;
        let read_count = file.read(&mut data_buffer)?;
        if read_count < chunk_size_usize && i != num_chunks {
            eprintln!("WARNING: Read only {read_count} instead of {chunk_size} bytes for chunk {i} out of {num_chunks}");
        }
        let checksum = xxhash_rust::xxh3::xxh3_64(&data_buffer);
        checksums.push(checksum);
    }

    Ok(FileChecksums {
        chunk_blocks,
        last_chunk_blocks,
        checksums,
    })
}

/// Wraps a `TcpStream` to be able to conveniently read `bincode` de-/encodable objects.
pub struct SocketWrapper {
    pub socket: TcpStream,
    noise: Option<NoiseWrapper>,
    nonce: u64,
}

impl SocketWrapper {
    #[must_use]
    pub fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            noise: None,
            nonce: 0,
        }
    }

    pub fn set_noise_state(&mut self, state: StatelessTransportState) {
        self.noise = Some(NoiseWrapper::new(state));
    }

    /// Increment the stored nonce. Returns the old value.
    ///
    /// # Panics
    /// Panics on overflow.
    pub fn nonce(&mut self) -> u64 {
        let old = self.nonce;
        self.nonce = self.nonce.checked_add(1).expect("nonce overflow");
        old
    }

    /// Try to decrypt the given payload, and decode the result as one instance of type `T`.
    ///
    /// # Errors
    /// Returns an error if decryption or decoding was unsuccessful.
    ///
    /// # Panics
    /// Panics if decryption is not available (noise not initialised)
    pub fn decrypt_decode<T: bincode::Decode>(
        &mut self,
        nonce: u64,
        payload: &[u8],
    ) -> anyhow::Result<T> {
        let noise = self.noise.as_mut().expect("decryption should be available");
        decrypt_decode(&noise.state, &mut noise.write_buffer, nonce, payload)
    }

    /// Try to decrypt the given payload, and borrow-decode the result as one instance of type `T`.
    ///
    /// # Errors
    /// Returns an error if decryption or decoding was unsuccessful.
    ///
    /// # Panics
    /// Panics if decryption is not available (noise not initialised)
    pub fn decrypt_borrow_decode<'a, T: bincode::BorrowDecode<'a>>(
        &mut self,
        nonce: u64,
        payload: &[u8],
        write_buffer: &'a mut [u8],
    ) -> anyhow::Result<T> {
        let noise = self.noise.as_mut().expect("decryption should be available");
        decrypt_borrow_decode(&noise.state, write_buffer, nonce, payload)
    }

    /// Encode the given object using bincode and encrypt the resulting data as a noise message. The
    /// `write_buffer` is used as an intermediate; it must be large enough to hold the noise message
    /// (i.e. at least the encoded data length + 16 bytes). If successful, the slice of the buffer
    /// containing the message is returned.
    ///
    /// # Errors
    /// Returns an error if encoding or encryption was unsuccessful.
    ///
    /// # Panics
    /// Panics if encryption is unavailable.
    pub fn encode_encrypt<'a, T: bincode::Encode>(
        &mut self,
        write_buffer: &'a mut [u8],
        nonce: u64,
        value: T,
    ) -> anyhow::Result<&'a [u8]> {
        let noise = self.noise.as_mut().expect("encryption should be available");
        encode_encrypt(
            &noise.state,
            &mut noise.read_buffer,
            write_buffer,
            nonce,
            value,
        )
    }

    /// Try to read one instance of the given type from the TCP stream. Blocks until one complete
    /// instance is read.
    ///
    /// # Errors
    /// Returns an error if the reading process terminated prematurely (e.g. due to EOF)
    pub fn read<T: bincode::Decode>(&mut self) -> anyhow::Result<T> {
        match &mut self.noise {
            Some(noise) => {
                let NoiseHeader { length, nonce } = read_unencrypted(&mut self.socket)?;
                let payload = &mut noise.read_buffer[..(length as usize)];
                self.socket.read_exact(payload)?;
                decrypt_decode(&noise.state, &mut noise.write_buffer, nonce, payload)
            }
            None => {
                // No encryption is available
                self.read_unencrypted()
            }
        }
    }

    /// Try to read one instance of the given type from the unencrypted TCP stream. Blocks until one
    /// complete instance is read.
    ///
    /// # Errors
    /// Returns an error if the reading process terminated prematurely (e.g. due to EOF)
    pub fn read_unencrypted<T: bincode::Decode>(&mut self) -> anyhow::Result<T> {
        read_unencrypted(&mut self.socket)
    }

    /// Write the given object into the TCP stream.
    ///
    /// # Errors
    /// Returns an error if writing the bytes was unsuccessful.
    ///
    /// # Panics
    /// Panics if the data decoded by noise does not fit into the size limit.
    pub fn write<T: bincode::Encode>(&mut self, value: T) -> anyhow::Result<usize> {
        let nonce = self.nonce();

        match &mut self.noise {
            Some(noise) => {
                let message = encode_encrypt(
                    &noise.state,
                    &mut noise.read_buffer,
                    &mut noise.write_buffer,
                    nonce,
                    value,
                )?;

                write_unencrypted(
                    &mut self.socket,
                    NoiseHeader {
                        length: message
                            .len()
                            .try_into()
                            .expect("noise message length overflow"),
                        nonce,
                    },
                )?;
                self.socket.write_all(message)?;
                Ok(message.len())
            }
            None => {
                // No encryption is available
                self.write_unencrypted(value)
            }
        }
    }

    /// Write the given object into the unencrypted TCP stream.
    ///
    /// # Errors
    /// Returns an error if writing the bytes was unsuccessful.
    pub fn write_unencrypted<T: bincode::Encode>(&mut self, value: T) -> anyhow::Result<usize> {
        write_unencrypted(&mut self.socket, value)
    }

    /// Flushes the TCP stream.
    ///
    /// # Errors
    /// Returns an error on I/O failure.
    pub fn flush(&mut self) -> anyhow::Result<()> {
        self.socket.flush()?;
        Ok(())
    }
}

fn read_unencrypted<T: bincode::Decode>(socket: &mut TcpStream) -> anyhow::Result<T> {
    Ok(bincode::decode_from_std_read(socket, BINCODE_CONFIG)?)
}

fn write_unencrypted<T: bincode::Encode>(
    socket: &mut TcpStream,
    value: T,
) -> anyhow::Result<usize> {
    Ok(bincode::encode_into_std_write(
        value,
        socket,
        BINCODE_CONFIG,
    )?)
}

fn decrypt_decode<T: bincode::Decode>(
    state: &StatelessTransportState,
    write_buffer: &mut [u8],
    nonce: u64,
    payload: &[u8],
) -> anyhow::Result<T> {
    let message_len = state.read_message(nonce, payload, write_buffer)?;
    let message = &write_buffer[..message_len];
    match bincode::decode_from_slice(message, BINCODE_CONFIG) {
        Ok((decoded, _)) => Ok(decoded),
        Err(err) => {
            bail!("Failed to decode data {message:x?}, error: {err}");
        }
    }
}

fn decrypt_borrow_decode<'a, T: bincode::BorrowDecode<'a>>(
    state: &StatelessTransportState,
    write_buffer: &'a mut [u8],
    nonce: u64,
    payload: &[u8],
) -> anyhow::Result<T> {
    let message_len = state.read_message(nonce, payload, write_buffer)?;
    let message = &write_buffer[..message_len];
    match bincode::borrow_decode_from_slice(message, BINCODE_CONFIG) {
        Ok((decoded, _)) => Ok(decoded),
        Err(err) => {
            bail!("Failed to decode data {message:x?}, error: {err}");
        }
    }
}

fn encode_encrypt<'a, T: bincode::Encode>(
    state: &StatelessTransportState,
    read_buffer: &mut [u8],
    write_buffer: &'a mut [u8],
    nonce: u64,
    value: T,
) -> anyhow::Result<&'a [u8]> {
    let encoded_len = bincode::encode_into_slice(value, read_buffer, BINCODE_CONFIG)?;
    let message_len = state.write_message(nonce, &read_buffer[..encoded_len], write_buffer)?;
    let message = &write_buffer[..message_len];
    Ok(message)
}

struct NoiseWrapper {
    pub state: StatelessTransportState,
    pub read_buffer: Vec<u8>,
    pub write_buffer: Vec<u8>,
}

impl NoiseWrapper {
    #[must_use]
    pub fn new(state: StatelessTransportState) -> Self {
        Self {
            state,
            read_buffer: vec![0_u8; 0xffff],
            write_buffer: vec![0_u8; 0xffff],
        }
    }
}

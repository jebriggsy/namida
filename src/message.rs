use std::{path::PathBuf, time::Duration};

use crate::types::{
    BlockIndex, BlockSize, ErrorRate, FileMetadata, FileSize, Fraction, TargetRate,
};

#[derive(Debug, bincode::Encode, bincode::Decode)]
pub enum ClientToServer {
    ProtocolRevision(u32),
    AuthenticationResponse([u8; 16]),
    FileRequest {
        path: PathBuf,
        block_size: BlockSize,
        target_rate: TargetRate,
        error_rate: ErrorRate,
        slowdown: Fraction,
        speedup: Fraction,
    },
    UdpInit(UdpMethod),
    MultiRequest,
    MultiAcknowledgeCount,
    MultiEnd,
    RetransmitMany(Vec<BlockIndex>),
    DirList,
    DirListEnd,
}

#[derive(Debug, bincode::Encode, bincode::Decode)]
pub enum ServerToClient {
    ProtocolRevision(u32),
    AuthenticationChallenge([u8; 64]),
    AuthenticationStatus(bool),
    FileRequestSuccess {
        file_size: FileSize,
        block_size: BlockSize,
        block_count: BlockIndex,
        epoch: Duration,
        udp_port: u16,
    },
    FileRequestError(FileRequestError),
    UdpDone,
    DirListHeader {
        status: DirListStatus,
        num_files: u32,
    },
    DirListFile(FileMetadata),
    MultiFileCount(u32),
    MultiFile(FileMetadata),
}

#[derive(Debug, bincode::Encode, bincode::Decode)]
pub enum TransmissionControl {
    RestartAt(BlockIndex),
    Retransmit(BlockIndex),
    SubmitErrorRate(ErrorRate),

    /// Dummy value to ensure all enum variants have the same length
    EndTransmission(u32),
}

impl TransmissionControl {
    pub const SIZE: usize = 8;
}

#[derive(Debug, Copy, Clone, bincode::Encode, bincode::Decode)]
pub enum UdpMethod {
    StaticPort(u16),
    Discovery,
}

#[derive(Debug, Copy, Clone, bincode::Encode, bincode::Decode)]
pub enum FileRequestError {
    Nonexistent,
}

#[derive(Debug, Copy, Clone, bincode::Encode, bincode::Decode)]
pub enum DirListStatus {
    Ok,
    Unsupported,
}

#[cfg(test)]
mod tests {
    use crate::types::{BlockIndex, ErrorRate};

    use super::TransmissionControl;

    #[test]
    fn transmission_control_sizes() -> anyhow::Result<()> {
        let mut slice = [0_u8; 8];

        assert_eq!(
            bincode::encode_into_slice(
                TransmissionControl::RestartAt(BlockIndex(0)),
                &mut slice,
                crate::common::BINCODE_CONFIG,
            )?,
            TransmissionControl::SIZE
        );
        assert_eq!(
            bincode::encode_into_slice(
                TransmissionControl::Retransmit(BlockIndex(0)),
                &mut slice,
                crate::common::BINCODE_CONFIG,
            )?,
            TransmissionControl::SIZE
        );
        assert_eq!(
            bincode::encode_into_slice(
                TransmissionControl::SubmitErrorRate(ErrorRate(0)),
                &mut slice,
                crate::common::BINCODE_CONFIG,
            )?,
            TransmissionControl::SIZE
        );
        assert_eq!(
            bincode::encode_into_slice(
                TransmissionControl::EndTransmission(0),
                &mut slice,
                crate::common::BINCODE_CONFIG,
            )?,
            TransmissionControl::SIZE
        );

        Ok(())
    }
}

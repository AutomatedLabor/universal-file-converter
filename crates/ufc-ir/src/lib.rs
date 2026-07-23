//! # UFC Intermediate Representations
//!
//! Domain-specific IRs that sit between source decoders and target encoders.
//! Using IRs avoids N×M pairwise converter implementations.
//!
//! Each IR is designed to be:
//! - **Complete**: captures all meaningful information from source formats
//! - **Versioned**: schema version for forward compatibility
//! - **Inspectable**: easy to debug and validate
//! - **Serializable**: can be persisted for caching/debugging

pub mod archive;
pub mod audio;
pub mod document;
pub mod image;
pub mod mesh;
pub mod table;
pub mod traits;
pub mod vector;
pub mod video;

pub use archive::ArchiveIR;
pub use audio::AudioIR;
pub use document::DocumentIR;
pub use image::ImageIR;
pub use mesh::Mesh3DIR;
pub use table::TableIR;
pub use traits::IntermediateRepresentation;
pub use vector::VectorIR;
pub use video::VideoIR;

/// Get the current IR API version.
pub fn api_version() -> semver::Version {
    semver::Version::new(1, 0, 0)
}

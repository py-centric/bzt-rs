#[allow(
    clippy::missing_errors_doc,
    clippy::doc_markdown,
    clippy::default_trait_access,
    clippy::too_many_lines
)]
pub mod bzt_mock {
    tonic::include_proto!("bzt_mock");
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("bzt_mock_descriptor");
}

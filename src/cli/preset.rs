/// Enum representing different preset barcode formats.
#[derive(clap::ValueEnum, Clone)]
pub enum PresetBarcodeFormats {
    /// @BARCODE_UMI format as produced by Flexiplex for 10x3 chemistry
    BcUmi,

    /// `_<UMI>` format as produced by `umi-tools extract`.
    UmiTools,

    /// bcl2fastq format, which has `:<UMI>` at the end of the read ID.
    Illumina,
}

/// Returns a regular expression string for barcode presets.
///
/// # Arguments
///
/// * `preset` - A reference to a `PresetBarcodeFormats` enum variant.
///
/// # Returns
///
/// A `String` containing the regular expression for the specified barcode format.
pub fn get_barcode_regex(preset: &PresetBarcodeFormats) -> String {
    match preset {
        PresetBarcodeFormats::BcUmi => String::from(r"^([ATCG]{16})_([ATCG]{12})"),
        PresetBarcodeFormats::UmiTools => String::from(r"_([ATCG]+)$"),
        PresetBarcodeFormats::Illumina => String::from(r":([ATCG]+)$"),
    }
}

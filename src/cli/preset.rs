use regex::Regex;

/// Enum representing different preset barcode formats.
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum PresetBarcodeFormats {
    /// @BARCODE_UMI format as produced by Flexiplex for 10x3 chemistry
    BcUmi,

    /// `_<UMI>` format as produced by `umi-tools extract`.
    UmiTools,

    /// bcl2fastq format, which has `:<UMI>` at the end of the read ID.
    Illumina,
}

impl PresetBarcodeFormats {
    /// Returns the corresponding regex for the preset
    pub fn to_regex(&self) -> Result<Regex, regex::Error> {
        Regex::new(match self {
            PresetBarcodeFormats::BcUmi => r"^([ATCG]{16})_([ATCG]{12})",
            PresetBarcodeFormats::UmiTools => r"_([ATCG]+)$",
            PresetBarcodeFormats::Illumina => r":([ATCG]+)$",
        })
    }
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum PresetOutputFormats {
    Fastq,
    Fasta,
}

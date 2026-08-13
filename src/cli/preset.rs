// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

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

    /// .sam tag format with barcode and UMI, which uses the :CB:Z:____ and :UB:Z:____ tag format.
    SamTaggedCBUB,

    /// .sam tag format with only barcode, which uses the :CB:Z:___ format.
    SamTaggedCB,
}

impl PresetBarcodeFormats {
    /// Returns the corresponding regex for the preset
    pub fn to_regex(&self) -> Result<Regex, regex::Error> {
        Regex::new(match self {
            PresetBarcodeFormats::BcUmi => r"^(?<CB>[ATCGNX]{16})_(?<UB>[ATCGNX]{12})",
            PresetBarcodeFormats::UmiTools => r"_(?<UB>[ATCGNX]+)$",
            PresetBarcodeFormats::Illumina => r":(?<UB>[ATCGNX]+)$",
            PresetBarcodeFormats::SamTaggedCB => r"\sCB:Z:(?<CB>[ATCGNX]+)",
            PresetBarcodeFormats::SamTaggedCBUB => {
                r"\sCB:Z:(?<CB>[ATCGNX]+).*\sUB:Z:(?<UB>[ATCGNX]+)"
            }
        })
    }
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
pub enum PresetOutputFormats {
    Fastq,
    Fasta,
    Metadata,
}

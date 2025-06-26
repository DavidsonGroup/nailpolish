// Copyright 2025 Oliver Cheng <cheng.o@wehi.edu.au> and the Davidson Lab.
// This program is distributed under the MIT License.
// We also ask that you cite this software in publications
// where you made use of it for any part of the data analysis.

/// A numeric interval with a minimum and maximum bound
#[derive(Copy, Clone, Debug)]
pub struct ArgInterval {
    pub min: f32,
    pub max: f32,
}

/// Error type for parsing an interval string.
#[derive(Debug)]
pub struct ParseIntervalErr(String);

impl std::fmt::Display for ParseIntervalErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid interval format: {}", self.0)
    }
}

impl std::error::Error for ParseIntervalErr {}

/// Parses a string in the format "min,max" into an interval. Supports "inf" and "-inf"
impl<'a> TryFrom<&'a str> for ArgInterval {
    type Error = ParseIntervalErr;

    fn try_from(arg: &'a str) -> Result<ArgInterval, Self::Error> {
        let arg_lc = arg.to_lowercase();
        let parts: Vec<&str> = arg_lc.split(',').collect();

        if parts.len() != 2 {
            return Err(ParseIntervalErr(indoc::formatdoc! {"
            Expected format '<min>,<max>', got '{arg}'. The expected format is \
            `a,b`, as in:
              --len 0,15000
              --len 0,inf
              --len 100,15000
            "}));
        }

        // Try to parse the minimum and maximum, handling unbounded cases.
        let min = match parts[0].trim() {
            "-inf" => f32::NEG_INFINITY,
            s => s.parse::<f32>().map_err(|_| {
                ParseIntervalErr(format!(
                    "Invalid minimum value: '{}' (should be any float or `-inf`)",
                    parts[0].trim()
                ))
            })?,
        };

        let max = match parts[1].trim() {
            "inf" => f32::INFINITY,
            s => s.parse::<f32>().map_err(|_| {
                ParseIntervalErr(format!(
                    "Invalid maximum value: '{}' (should be any float or `inf`)",
                    parts[1].trim()
                ))
            })?,
        };

        Ok(ArgInterval { min, max })
    }
}

impl ArgInterval {
    /// Tests if a value lies within the interval (exclusive bounds)
    pub fn contains(&self, v: f32) -> bool {
        (self.min < v) && (v < self.max)
    }
}

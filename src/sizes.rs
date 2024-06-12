// SPDX-FileCopyrightText: © Matteo Settenvini <matteo.settenvini@montecristosoftware.eu>
// SPDX-License-Identifier: EUPL-1.2

pub fn bytes_to_human(bytes: u64) -> String {
    use human_size::{Any, SpecificSize};

    let size: f64 = bytes as f64;
    let digits = size.log10().floor() as u32;
    let mut order = digits / 3;
    let unit = match order {
        0 => Any::Byte,
        1 => Any::Kilobyte,
        2 => Any::Megabyte,
        _ => {
            order = 3; // Let's stop here.
            Any::Gigabyte
        }
    };

    format!(
        "{:.3}",
        SpecificSize::new(size / 10u64.pow(order * 3) as f64, unit)
            .unwrap_or(SpecificSize::new(0., Any::Byte).unwrap())
    )
}

// -------------------------------------------------------------

#[cfg(test)]
mod tests {
    use rstest::rstest;

    #[rstest]
    #[case(1024, "1.024 kB")]
    #[case(10240, "10.240 kB")]
    #[case(1024*1024, "1.049 MB")]
    #[case(1024*1024*1024, "1.074 GB")]
    #[case(0, "0.000 B")]
    #[case(u64::MAX, format!("{:.3} GB",u64::MAX as f64/(1_000_000_000.0)))]
    #[case(u64::MIN, format!("{:.3} B",u64::MIN as f64))]
    fn bytes_to_human(#[case] bytes: u64, #[case] expected: String) {
        assert_eq!(super::bytes_to_human(bytes), expected);
    }
}

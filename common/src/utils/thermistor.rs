// precomputed
const LUT: [u16; 20] = [
    111, // 0C
    144, // 5C
    186, // ...
    236, 298, 372, 459, 559, 675, 804, 948, 1104, 1271, 1446, 1626, 1809, 1991, 2170,
    2344, // ...
    2511, // 95C
];

pub const fn celsius_to_sample(celsius: u8) -> u16 {
    LUT[(celsius / 5) as usize]
}

#[cfg(not(target_os = "none"))]
#[uniffi::export]
fn sample_to_celsius(sample: u16) -> u8 {
    match LUT.binary_search(&sample) {
        Ok(index) => u8::try_from(index).unwrap() * 5,
        Err(index) => {
            if index == 0 {
                0
            } else {
                let lower = LUT[index - 1];
                let upper = LUT[index];

                u8::try_from(
                    5 * (sample - lower) / (upper - lower)
                        + 5 * (u16::try_from(index).unwrap() - 1),
                )
                .unwrap()
            }
        }
    }
}

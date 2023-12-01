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

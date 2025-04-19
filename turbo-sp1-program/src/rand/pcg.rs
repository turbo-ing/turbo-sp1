/// XSH-RS (64 → 32 bits)
///
/// This function takes a 64-bit unsigned integer `x` and does the following:
/// 1. Computes `count` as the top 3 bits of `x` (i.e. `x >> 61`).
/// 2. Modifies `x` by XOR’ing it with itself shifted right by 22 bits.
/// 3. Returns the lower 32-bit result of `x` shifted right by `(29 - count)`.
pub fn xsh_rs(mut x: u64) -> u32 {
    // Extract a 3-bit count from the most-significant bits.
    let count = (x >> 61) as u32;

    // In-place modification of x.
    x ^= x >> 22;

    // Shift x right by (29 - count) and cast to u32.
    (x >> (29 - count)) as u32
}

/// XSL-RR using two u64 values (high and low) to represent a 128-bit state.
///
/// This function does the following:
/// 1. Extracts the top 6 bits as the rotate count from the high part:
///      count = high >> 58;  // (Because 128-122 = 6, and the top 6 bits are in `high`)
/// 2. Combines the two parts by taking the XOR of the high and low parts.
///    This mimics the operation: (x ^ (x >> 64)) as u64, where x = (high<<64) | low.
///    When computed, this yields (low ^ high).
/// 3. Rotates the resulting 64-bit value to the right by the computed count.
pub fn xsl_rr(high: u64, low: u64) -> u64 {
    // Extract the top 6 bits from the high part.
    let count = (high >> 58) as u32;

    // Combine the two parts of the state.
    // Note: The original operation for a 128-bit x is:
    //    x64 = (uint64_t)(x ^ (x >> 64))
    // When x is formed as (high << 64 | low), then x >> 64 equals high,
    // and the lower 64 bits of (x ^ (x >> 64)) become (low ^ high).
    let x64 = low ^ high;

    // Rotate the result to the right by `count` bits.
    x64.rotate_right(count)
}

/// RXS-M-XS (64 → 64 bits)
///
/// This function takes a 64-bit unsigned integer `x` and performs the PCG RXS-M-XS transform:
/// 1. Extracts a 5-bit count from the most significant bits (x >> 59)
/// 2. XORs x with itself shifted right by (5 + count) bits
/// 3. Multiplies by a constant multiplier
/// 4. XORs with itself shifted right by 43 bits
pub fn rxs_m_xs(mut x: u64) -> u64 {
    // Extract 5-bit count from most significant bits
    let count = (x >> 59) as u32;

    // XOR with variable right shift
    x ^= x >> (5 + count);

    // Multiply by constant
    x = x.wrapping_mul(12605985483714917081);

    // XOR with fixed right shift
    x ^ (x >> 43)
}

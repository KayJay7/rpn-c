use num_traits::Zero;
use ramp::int::Int;
use ramp::rational::Rational;

// Byte iterator for string printing
// with "buffering" to allow better unrolling
// Might cause an excess of up to 7 trailing zeroes
pub struct Stringer {
    num: Int,
    partial: u64,
    iter: usize,
}

impl Stringer {
    // Constructor, consumes the provided Int
    #[inline]
    pub fn from(num: Int) -> Stringer {
        Stringer {
            num: num.abs(),
            partial: 0,
            // We could do withoud iter, and reduce the number of (possibly useless) 0-writes
            // but having it makes the loop more predictable
            // plus, those 0-writes might not be actually useless
            iter: 8,
        }
    }
}

impl Iterator for Stringer {
    type Item = u8;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // If all 8 bufferized bytes have been printed
        // extract new ones
        if self.iter == 8 {
            // Returns None if there are no more bytes to extract
            if self.num.is_zero() {
                return None;
            }

            // Otherwise extract 8 more using integer divisions and modulo
            let (q, r) = self.num.divmod(&Int::from(0x1_00_00_00_00_00_00_00_00u128));
            // Populate buffer with extracted bytes
            self.num = q;
            self.partial = u64::from(&r);
            self.iter = 0;
        }

        // Extract one byte from buffer, increase counter, and returns
        let shift = self.iter * 8;
        let byte = ((self.partial >> shift) & 255) as u8;
        self.iter += 1;
        Some(byte)
    }
}

#[inline]
pub fn from_string(string: &str) -> Rational {
    // Trims quotes and turns into iter
    let iter = string.as_bytes()[1..(string.len() - 1)].into_iter();
    let mut num = Int::zero();
    let mut esc = false;
    let mut esc2 = false;
    let mut prev = 0;
    let mut stack = Vec::new();

    // Parse escape sequences
    for ch in iter {
        // If not escaped, just stack it
        if !esc {
            // 92='\\'
            if *ch != 92 {
                stack.push(*ch);
            } else {
                esc = true;
            }
        } else {
            // Otherwise look at next character
            esc = false;
            if !esc2 {
                // If its a shortcut escape, stack it
                // Note that num was already shifted from the loop before
                match *ch {
                    110 => stack.push(10), // \n
                    114 => stack.push(13), // \r
                    116 => stack.push(09), // \t
                    92 => stack.push(92),  // \\
                    34 => stack.push(34),  // \"
                    // If its an hex escape, save it and read the next one to
                    _ => {
                        esc = true;
                        esc2 = true;
                        prev = from_hex(*ch) << 4;
                    }
                }
            } else {
                // Now that you have the second hex, stack the whole
                // Note that num was already shifted from the loop before
                esc = false;
                esc2 = false;
                stack.push(prev + from_hex(*ch));
            }
        }
    }

    // Puts characters into num
    // accelerated in a similar fashion to stringer
    while !stack.is_empty() {
        let mut partial: u64 = 0;
        let mut count = 0;
        let mut stop = false;

        // Stack at most 8 bytes into partial, and counts them
        // Using a while let would drop one byte every 8
        while count != 8 && !stop {
            if let Some(ch) = stack.pop() {
                partial <<= 8;
                partial += ch as u64;
                count += 1;
            } else {
                stop = true;
            }
        }

        // Make space for partial, and add it
        num <<= 8 * count;
        num += partial;
    }

    Rational::from(num)
}

#[inline]
fn from_hex(hex: u8) -> u8 {
    if hex >= 48 && hex <= 57 {
        hex - 48
    } else if hex >= 65 && hex <= 90 {
        hex - 55
    } else if hex >= 97 && hex <= 122 {
        hex - 87
    } else {
        0
    }
}

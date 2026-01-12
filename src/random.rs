use crate::Context;

pub struct RandomContext {
    seed: u32,
    rand_position: u32,
}

pub struct RandomValue(u32);

impl RandomValue {
    /// random u32 [u32::MIN, u32::MAX]
    #[inline]
    pub fn u32(&mut self) -> u32 {
        self.0
    }

    /// random i32 [i32::MIN, i32::MAX]
    #[inline]
    pub fn i32(&mut self) -> i32 {
        self.u32() as i32
    }

    /// random f32 [f32::MIN, f32::MAX]
    #[inline]
    pub fn f32(&mut self) -> f32 {
        self.u32() as f32
    }

    /// random f32 [0, 1]
    #[inline]
    pub fn unorm(&mut self) -> f32 {
        self.f32() / u32::MAX as f32
    }

    /// random f32 [-1, 1]
    #[inline]
    pub fn snorm(&mut self) -> f32 {
        self.unorm() * 2.0 - 1.0
    }

    /// random u32 [min, max]
    #[inline]
    pub fn range_u32(&mut self, min: u32, max: u32) -> u32 {
        min + ((self.unorm() * ((max - min) as f32)) as u32)
    }

    /// random i32 [min, max]
    #[inline]
    pub fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        min + ((self.unorm() * ((max - min) as f32)) as i32)
    }

    /// random f32 [min, max]
    #[inline]
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.unorm() * (max - min)
    }

    // TODO: add vectors?
}

impl RandomContext {
    pub(crate) fn new() -> Self {
        Self {
            seed: 0x9E3779B9,
            rand_position: 0,
        }
    }

    #[inline]
    fn rand(&mut self) -> RandomValue {
        self.rand_position += 1;
        RandomValue(self.hash(self.rand_position))
    }

    #[inline]
    fn hash(&mut self, v: u32) -> u32 {
        squirrel5(v, self.seed)
    }

    #[inline]
    fn hash_u32(&mut self, v: u32) -> RandomValue {
        RandomValue(self.hash(v))
    }

    #[inline]
    fn hash_i32(&mut self, v: i32) -> RandomValue {
        RandomValue(self.hash(v as u32))
    }

    #[inline]
    fn hash_f32(&mut self, v: f32) -> RandomValue {
        RandomValue(self.hash(v as u32))
    }

    // TODO: add vectors?
}

fn squirrel5(pos: u32, seed: u32) -> u32 {
    const SQ5_BIT_NOISE1: u32 = 0xd2a80a3f; // 11010010101010000000101000111111
    const SQ5_BIT_NOISE2: u32 = 0xa884f197; // 10101000100001001111000110010111
    const SQ5_BIT_NOISE3: u32 = 0x6C736F4B; // 01101100011100110110111101001011
    const SQ5_BIT_NOISE4: u32 = 0xB79F3ABB; // 10110111100111110011101010111011
    const SQ5_BIT_NOISE5: u32 = 0x1b56c4f5; // 00011011010101101100010011110101

    let mut mangled_bits = pos;
    mangled_bits = mangled_bits.wrapping_mul(SQ5_BIT_NOISE1);
    mangled_bits = mangled_bits.wrapping_add(seed);
    mangled_bits ^= mangled_bits.wrapping_shr(9);
    mangled_bits = mangled_bits.wrapping_add(SQ5_BIT_NOISE2);
    mangled_bits ^= mangled_bits.wrapping_shr(11);
    mangled_bits = mangled_bits.wrapping_mul(SQ5_BIT_NOISE3);
    mangled_bits ^= mangled_bits.wrapping_shr(13);
    mangled_bits = mangled_bits.wrapping_add(SQ5_BIT_NOISE4);
    mangled_bits ^= mangled_bits.wrapping_shr(15);
    mangled_bits = mangled_bits.wrapping_mul(SQ5_BIT_NOISE5);
    mangled_bits ^= mangled_bits.wrapping_shr(17);

    mangled_bits
}

//
// Commands
//

/// hash u32
pub fn hash_u32(ctx: &mut Context, value: u32) -> RandomValue {
    ctx.random.hash_u32(value)
}

/// hash i32
pub fn hash_i32(ctx: &mut Context, value: i32) -> RandomValue {
    ctx.random.hash_i32(value)
}

/// hash f32
pub fn hash_f32(ctx: &mut Context, value: f32) -> RandomValue {
    ctx.random.hash_f32(value)
}

/// hash without input
///
/// keeps internal position
/// will always generate same sequence for same seed
pub fn rand(ctx: &mut Context) -> RandomValue {
    ctx.random.rand()
}

/// seed the random state
pub fn seed(ctx: &mut Context, seed: u32) {
    ctx.random.seed = seed;
}

/// seed the random state with time since UNIX_EPOCH
pub fn seed_with_time(ctx: &mut Context) {
    #[cfg(target_arch = "wasm32")]
    let nanos = (web_sys::window()
        .expect("could not get window")
        .performance()
        .expect("could not get performance")
        .now()
        * 1000000.0) as u32;
    #[cfg(not(target_arch = "wasm32"))]
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("could not get duration since UNIX EPOCH")
        .as_nanos() as u32;

    seed(ctx, nanos);
}

/// resets the random state
pub fn reset(ctx: &mut Context) {
    ctx.random.rand_position = 0;
}

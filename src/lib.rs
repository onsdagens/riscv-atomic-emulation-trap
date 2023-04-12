#![doc = include_str!("../README.md")]
#![cfg_attr(not(test), no_std)]

pub const PLATFORM_REGISTER_LEN: usize = 32; // TODO will be less on r32e, handle at somepoint

macro_rules! amo {
    ($frame:ident, $rs1:ident, $rs2:ident, $rd:ident, $operation:expr) => {
        let tmp = $frame[$rs1 as usize];
        let a = *(tmp as *const _);
        let b = $frame[$rs2 as usize];
        $frame[$rd as usize] = a;
        *(tmp as *mut _) = $operation(a, b);
    };
}

/// is_atomic_instruction
/// 
/// Take the instruction and returns whether the instruction at that address is an atomic one
pub unsafe fn is_atomic_instruction(insn: u32) -> bool {
    (insn & 0b1111111) == 0b0101111
}

/// atomic_emulation
/// 
/// Takes the exception program counter and an array of registers at point of exception with [`PLATFORM_REGISTER_LEN`] length.
pub unsafe fn atomic_emulation(pc: usize, frame: &mut [usize; PLATFORM_REGISTER_LEN]) -> bool {
    static mut S_LR_ADDR: usize = 0;
    let insn = if pc % 4 != 0 {
        let prev_aligned = pc & !0x3;
        let offset = (pc - prev_aligned) as usize; 

        let buffer = (*((prev_aligned + 4) as *const u32) as u64) << 32
            | (*(prev_aligned as *const u32) as u64);
        let buffer_bytes = buffer.to_le_bytes();

        u32::from_le_bytes([
            buffer_bytes[offset],
            buffer_bytes[offset + 1],
            buffer_bytes[offset + 2],
            0,
        ])
    } else {
        *(pc as *const u32)
    };

    //not needed since checked in rt
    //if !is_atomic_instruction(insn) {
    //    return false;
    //}

    let reg_mask = 0b11111;
    // destination register
    let rd = ((insn >> 7) & reg_mask) as usize;
    // source 1 register
    let rs1 = ((insn >> 15) & reg_mask) as usize;
    // source 2 register
    let rs2 = ((insn >> 20) & reg_mask) as usize;

    match insn >> 27 {
        0b00010 => {
            /* LR */
            S_LR_ADDR = frame[rs1];
            let tmp: usize = *(S_LR_ADDR as *const _);
            frame[rd] = tmp;
        }
        0b00011 => {
            /* SC */
            let tmp: usize = frame[rs1];
            if tmp != S_LR_ADDR {
                frame[rd] = 1;
            } else {
                *(S_LR_ADDR as *mut _) = frame[rs2];
                frame[rd] = 0;
                S_LR_ADDR = 0;
            }
        }
        0b00001 => {
            /* AMOSWAP */
            amo!(frame, rs1, rs2, rd, |_, b| b);
        }
        0b00000 => {
            /* AMOADD */
            amo!(frame, rs1, rs2, rd, |a, b| a + b);
        }
        0b00100 => {
            /* AMOXOR */
            amo!(frame, rs1, rs2, rd, |a, b| a ^ b);
        }
        0b01100 => {
            /* AMOAND */
            amo!(frame, rs1, rs2, rd, |a, b| a & b);
        }
        0b01000 => {
            /* AMOOR */
            amo!(frame, rs1, rs2, rd, |a, b| a | b);
        }
        0b10000 => {
            /* AMOMIN */
            amo!(frame, rs1, rs2, rd, |a, b| (a as isize).min(b as isize));
        }
        0b10100 => {
            /* AMOMAX */
            amo!(frame, rs1, rs2, rd, |a, b| (a as isize).max(b as isize));
        }
        0b11000 => {
            /* AMOMINU */
            amo!(frame, rs1, rs2, rd, |a: usize, b| a.min(b));
        }
        0b11100 => {
            /* AMOMAXU */
            amo!(frame, rs1, rs2, rd, |a: usize, b| a.max(b));
        }
        _ => return false,
    }

    true
}

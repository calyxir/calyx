use num_integer::Integer;
use num_traits::{One, Zero};
use std::cmp::Ordering;
use std::fmt;
use std::ops::Rem;

use bitvec::prelude::*;
use num_bigint::{BigInt, BigUint};
use std::iter::once;

#[derive(Debug)]
pub struct SharedEnvironment {
    shared_bits: BitVec<usize, Lsb0>, // RI: integers are little-endian
    offsets: Vec<usize>,              // offsets[i] = start of node i
}

impl fmt::Display for SharedEnvironment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\nEnvironment:\n")?;

        for i in 0..self.offsets.len() - 1 {
            if self.offsets[i] == self.offsets[i + 1] {
                writeln!(f, "{} : _", i)?;
            } else if self.offsets[i + 1] - self.offsets[i]
                > (usize::BITS).try_into().unwrap()
            {
                writeln!(f, "{} : too large to display", i)?;
            } else {
                writeln!(
                    f,
                    "{} : {}",
                    i,
                    SharedEnvironment::slice_to_usize(
                        &self.shared_bits[self.offsets[i]..self.offsets[i + 1]]
                    )
                )?;
            }
        }

        Ok(())
    }
}

impl SharedEnvironment {
    pub fn new(node_sorts: Vec<usize>) -> Self {
        let offsets = once(&0usize)
            .chain(once(&0usize))
            .chain(node_sorts.iter())
            .scan(0usize, |state, &x| {
                *state += x;
                Some(*state)
            })
            .collect::<Vec<_>>();
        let shared_bits = BitVec::repeat(false, *offsets.last().unwrap());
        SharedEnvironment {
            shared_bits,
            offsets,
        }
    }

    /// Sets the bitslice corresponding to the node at with node_id `idx`
    pub fn set(&mut self, idx: usize, value: &BitSlice) {
        self.shared_bits[self.offsets[idx]..self.offsets[idx + 1]]
            .copy_from_bitslice(value);
    }

    /// Sets the bitslice corresponding to the node at with node_id `idx`, used for inputs
    pub fn set_vec(&mut self, idx: usize, value: Vec<bool>) {
        for i in self.offsets[idx]..self.offsets[idx + 1] {
            self.shared_bits.set(i, value[i - self.offsets[idx]]);
        }
    }

    /// Returns the bitslice corresponding to the node at with node_id `idx`
    pub fn get(&mut self, idx: usize) -> &BitSlice {
        &self.shared_bits[self.offsets[idx]..self.offsets[idx + 1]]
    }

    pub fn sext(&mut self, i1: usize, i2: usize) {
        let old_start = self.offsets[i1];
        let old_end = self.offsets[i1 + 1];
        let new_start = self.offsets[i2];
        let new_end = self.offsets[i2 + 1];
        let first_bit = self.shared_bits[old_start];
        self.shared_bits.copy_within(old_start..old_end, new_start);
        self.shared_bits[new_start + (old_end - old_start)..new_end]
            .fill(first_bit);
    }

    pub fn uext(&mut self, i1: usize, i2: usize) {
        let old_start = self.offsets[i1];
        let old_end = self.offsets[i1 + 1];
        let new_start = self.offsets[i2];
        let new_end = self.offsets[i2 + 1];
        self.shared_bits.copy_within(old_start..old_end, new_start);
        self.shared_bits[new_start + (old_end - old_start)..new_end]
            .fill(false);
    }

    pub fn slice(&mut self, u: usize, l: usize, i1: usize, i2: usize) {
        let old_start = self.offsets[i1];
        let new_start = self.offsets[i2];
        self.shared_bits
            .copy_within(old_start + l..old_start + u + 1, new_start);
    }

    pub fn not(&mut self, i1: usize, i2: usize) {
        let old_start = self.offsets[i1];
        let old_end = self.offsets[i1 + 1];
        let new_start = self.offsets[i2];
        let new_end = self.offsets[i2 + 1];
        let mut rhs = BitVec::repeat(true, old_end - old_start);
        rhs ^= &self.shared_bits[old_start..old_end];
        self.shared_bits[new_start..new_end]
            .copy_from_bitslice(rhs.as_bitslice());
    }

    pub fn and(&mut self, i1: usize, i2: usize, i3: usize) {
        let mut rhs = BitVec::from_bitslice(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        rhs &= &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .copy_from_bitslice(rhs.as_bitslice());
    }

    pub fn nand(&mut self, i1: usize, i2: usize, i3: usize) {
        let mut rhs = BitVec::from_bitslice(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        rhs &= &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        rhs = !rhs;
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .copy_from_bitslice(rhs.as_bitslice());
    }

    pub fn nor(&mut self, i1: usize, i2: usize, i3: usize) {
        let mut rhs = BitVec::from_bitslice(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        rhs |= &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        rhs = !rhs;
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .copy_from_bitslice(rhs.as_bitslice());
    }

    pub fn or(&mut self, i1: usize, i2: usize, i3: usize) {
        let mut rhs = BitVec::from_bitslice(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        rhs |= &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .copy_from_bitslice(rhs.as_bitslice());
    }

    pub fn xnor(&mut self, i1: usize, i2: usize, i3: usize) {
        let mut rhs = BitVec::from_bitslice(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        rhs ^= &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        rhs = !rhs;
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .copy_from_bitslice(rhs.as_bitslice());
    }

    pub fn xor(&mut self, i1: usize, i2: usize, i3: usize) {
        let mut rhs = BitVec::from_bitslice(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        rhs ^= &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .copy_from_bitslice(rhs.as_bitslice());
    }

    pub fn concat(&mut self, i1: usize, i2: usize, i3: usize) {
        let start1 = self.offsets[i1];
        let end1 = self.offsets[i1 + 1];
        let start2 = self.offsets[i2];
        let end2 = self.offsets[i2 + 1];
        let start3 = self.offsets[i3];
        self.shared_bits.copy_within(start1..end1, start3);
        self.shared_bits
            .copy_within(start2..end2, start3 + end1 - start1);
    }

    fn slice_to_bigint(slice: &BitSlice) -> BigInt {
        if slice.is_empty() {
            Zero::zero()
        } else if slice[slice.len() - 1] {
            let z: BigInt = Zero::zero();
            let o: BigInt = One::one();
            let mut ans = z - o;
            for i in 0..slice.len() {
                ans.set_bit(i.try_into().unwrap(), slice[i])
            }
            ans
        } else {
            let mut ans: BigInt = Zero::zero();
            for i in 0..slice.len() {
                ans.set_bit(i.try_into().unwrap(), slice[i])
            }
            ans
        }
    }

    fn slice_to_biguint(slice: &BitSlice) -> BigUint {
        let mut ans: BigUint = Zero::zero();
        for i in 0..slice.len() {
            ans.set_bit(i.try_into().unwrap(), slice[i])
        }
        ans
    }

    fn slice_to_usize(slice: &BitSlice) -> usize {
        let mut ans: usize = 0;
        for i in 0..slice.len() {
            if slice[i] {
                ans += 1 << i;
            }
        }
        ans
    }

    pub fn inc(&mut self, i1: usize, i2: usize) {
        self.shared_bits.copy_within(
            self.offsets[i1]..self.offsets[i1 + 1],
            self.offsets[i2],
        );
        let dest =
            self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]].as_mut();
        match dest.first_zero() {
            Some(i) => {
                dest[i..i + 1].fill(true);
                dest[..i].fill(false);
            }
            None => {
                dest.fill(false);
            }
        }
    }

    pub fn dec(&mut self, i1: usize, i2: usize) {
        self.shared_bits.copy_within(
            self.offsets[i1]..self.offsets[i1 + 1],
            self.offsets[i2],
        );
        let dest =
            self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]].as_mut();
        match dest.first_one() {
            Some(i) => {
                dest[i..i + 1].fill(false);
                dest[..i].fill(true);
            }
            None => {
                dest.fill(true);
            }
        }
    }

    pub fn neg(&mut self, i1: usize, i2: usize) {
        let bitwise_neg = !BitVec::from_bitslice(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let dest =
            self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]].as_mut();
        dest.copy_from_bitslice(&bitwise_neg);

        match dest.first_zero() {
            Some(i) => {
                dest[i..i + 1].fill(true);
                dest[..i].fill(false);
            }
            None => {
                dest.fill(false);
            }
        }
    }

    pub fn redand(&mut self, i1: usize, i2: usize) {
        let ans =
            self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]].all();
        self.shared_bits[self.offsets[i2]..self.offsets[i2] + 1].fill(ans);
    }

    pub fn redor(&mut self, i1: usize, i2: usize) {
        let ans =
            self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]].any();
        self.shared_bits[self.offsets[i2]..self.offsets[i2] + 1].fill(ans);
    }

    pub fn redxor(&mut self, i1: usize, i2: usize) {
        let ans = self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]]
            .count_ones()
            % 2
            == 1;
        self.shared_bits[self.offsets[i2]..self.offsets[i2] + 1].fill(ans);
    }

    pub fn iff(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = self.shared_bits[self.offsets[i1]]
            == self.shared_bits[self.offsets[i2]];
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn implies(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = !self.shared_bits[self.offsets[i1]]
            || self.shared_bits[self.offsets[i2]];
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn eq(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]]
            == self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn neq(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]]
            != self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    fn compare_signed(&self, i1: usize, i2: usize) -> Ordering {
        let a = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        a.cmp(&b)
    }

    fn compare_unsigned(&self, i1: usize, i2: usize) -> Ordering {
        let a = &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]];
        let b = &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]];
        a.cmp(b)
    }

    pub fn sgt(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = match self.compare_signed(i1, i2) {
            Ordering::Less => false,
            Ordering::Equal => false,
            Ordering::Greater => true,
        };
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn ugt(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = match self.compare_unsigned(i1, i2) {
            Ordering::Less => false,
            Ordering::Equal => false,
            Ordering::Greater => true,
        };
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn sgte(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = match self.compare_signed(i1, i2) {
            Ordering::Less => false,
            Ordering::Equal => true,
            Ordering::Greater => true,
        };
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn ugte(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = match self.compare_unsigned(i1, i2) {
            Ordering::Less => false,
            Ordering::Equal => true,
            Ordering::Greater => true,
        };
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn slt(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = match self.compare_signed(i1, i2) {
            Ordering::Greater => false,
            Ordering::Equal => false,
            Ordering::Less => true,
        };
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn ult(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = match self.compare_unsigned(i1, i2) {
            Ordering::Greater => false,
            Ordering::Equal => false,
            Ordering::Less => true,
        };
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn slte(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = match self.compare_signed(i1, i2) {
            Ordering::Greater => false,
            Ordering::Equal => true,
            Ordering::Less => true,
        };
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn ulte(&mut self, i1: usize, i2: usize, i3: usize) {
        let ans = match self.compare_unsigned(i1, i2) {
            Ordering::Greater => false,
            Ordering::Equal => true,
            Ordering::Less => true,
        };
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(ans);
    }

    pub fn add(&mut self, i1: usize, i2: usize, i3: usize) {
        let a = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        let c = a + b;
        for i in self.offsets[i3]..self.offsets[i3 + 1] {
            self.shared_bits[i..i + 1]
                .fill(c.bit((i - self.offsets[i3]).try_into().unwrap()));
        }
    }

    pub fn mul(&mut self, i1: usize, i2: usize, i3: usize) {
        let a = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        let c = a * b;
        for i in self.offsets[i3]..self.offsets[i3 + 1] {
            self.shared_bits[i..i + 1]
                .fill(c.bit((i - self.offsets[i3]).try_into().unwrap()));
        }
    }

    pub fn sdiv(&mut self, i1: usize, i2: usize, i3: usize) {
        let a = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        let c = a / b;
        for i in self.offsets[i3]..self.offsets[i3 + 1] {
            self.shared_bits[i..i + 1]
                .fill(c.bit((i - self.offsets[i3]).try_into().unwrap()));
        }
    }

    pub fn udiv(&mut self, i1: usize, i2: usize, i3: usize) {
        let a = Self::slice_to_biguint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_biguint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        let c = a / b;
        for i in self.offsets[i3]..self.offsets[i3 + 1] {
            self.shared_bits[i..i + 1]
                .fill(c.bit((i - self.offsets[i3]).try_into().unwrap()));
        }
    }

    pub fn smod(&mut self, i1: usize, i2: usize, i3: usize) {
        let a = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        let c = a.mod_floor(&b);
        for i in self.offsets[i3]..self.offsets[i3 + 1] {
            self.shared_bits[i..i + 1]
                .fill(c.bit((i - self.offsets[i3]).try_into().unwrap()));
        }
    }

    pub fn srem(&mut self, i1: usize, i2: usize, i3: usize) {
        let a = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        let mut c = a.mod_floor(&b);
        if a.sign() != b.sign() && !a.is_zero() && !b.is_zero() {
            c -= b;
        }
        for i in self.offsets[i3]..self.offsets[i3 + 1] {
            self.shared_bits[i..i + 1]
                .fill(c.bit((i - self.offsets[i3]).try_into().unwrap()));
        }
    }

    pub fn urem(&mut self, i1: usize, i2: usize, i3: usize) {
        let a = Self::slice_to_biguint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_biguint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        let c = a.rem(b);
        for i in self.offsets[i3]..self.offsets[i3 + 1] {
            self.shared_bits[i..i + 1]
                .fill(c.bit((i - self.offsets[i3]).try_into().unwrap()));
        }
    }

    pub fn sub(&mut self, i1: usize, i2: usize, i3: usize) {
        let a = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]],
        );
        let b = Self::slice_to_bigint(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        let c = a - b;
        for i in self.offsets[i3]..self.offsets[i3 + 1] {
            self.shared_bits[i..i + 1]
                .fill(c.bit((i - self.offsets[i3]).try_into().unwrap()));
        }
    }

    pub fn saddo(&mut self, _i1: usize, _i2: usize, _i3: usize) {
        todo!()
    }

    pub fn uaddo(&mut self, _i1: usize, _i2: usize, _i3: usize) {
        todo!()
    }

    pub fn sdivo(&mut self, _i1: usize, _i2: usize, _i3: usize) {
        todo!()
    }

    pub fn udivo(&mut self, _i1: usize, _i2: usize, i3: usize) {
        self.shared_bits[self.offsets[i3]..self.offsets[i3] + 1].fill(false);
    }

    pub fn smulo(&mut self, _i1: usize, _i2: usize, _i3: usize) {
        todo!()
    }

    pub fn umulo(&mut self, _i1: usize, _i2: usize, _i3: usize) {
        todo!()
    }

    pub fn ssubo(&mut self, _i1: usize, _i2: usize, _i3: usize) {
        todo!()
    }

    pub fn usubo(&mut self, _i1: usize, _i2: usize, _i3: usize) {
        todo!()
    }

    pub fn ite(&mut self, i1: usize, i2: usize, i3: usize, i4: usize) {
        if self.shared_bits[self.offsets[i1]] {
            self.shared_bits.copy_within(
                self.offsets[i2]..self.offsets[i2 + 1],
                self.offsets[i4],
            );
        } else {
            self.shared_bits.copy_within(
                self.offsets[i3]..self.offsets[i3 + 1],
                self.offsets[i4],
            );
        }
    }

    pub fn rol(&mut self, i1: usize, i2: usize, i3: usize) {
        let shift_amount = Self::slice_to_usize(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        self.shared_bits.copy_within(
            self.offsets[i1]..self.offsets[i1 + 1],
            self.offsets[i3],
        );
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .rotate_right(shift_amount);
    }

    pub fn ror(&mut self, i1: usize, i2: usize, i3: usize) {
        let shift_amount = Self::slice_to_usize(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        self.shared_bits.copy_within(
            self.offsets[i1]..self.offsets[i1 + 1],
            self.offsets[i3],
        );
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .rotate_left(shift_amount);
    }

    pub fn sll(&mut self, i1: usize, i2: usize, i3: usize) {
        let shift_amount = Self::slice_to_usize(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        self.shared_bits.copy_within(
            self.offsets[i1]..self.offsets[i1 + 1],
            self.offsets[i3],
        );
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .shift_right(shift_amount);
    }

    pub fn sra(&mut self, i1: usize, i2: usize, i3: usize) {
        let shift_amount = Self::slice_to_usize(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        self.shared_bits.copy_within(
            self.offsets[i1]..self.offsets[i1 + 1],
            self.offsets[i3],
        );
        let last_bit = self.shared_bits[self.offsets[i1 + 1] - 1];
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .shift_left(shift_amount);
        self.shared_bits
            [self.offsets[i3 + 1] - shift_amount..self.offsets[i3 + 1]]
            .fill(last_bit);
    }

    pub fn srl(&mut self, i1: usize, i2: usize, i3: usize) {
        let shift_amount = Self::slice_to_usize(
            &self.shared_bits[self.offsets[i2]..self.offsets[i2 + 1]],
        );
        self.shared_bits.copy_within(
            self.offsets[i1]..self.offsets[i1 + 1],
            self.offsets[i3],
        );
        self.shared_bits[self.offsets[i3]..self.offsets[i3 + 1]]
            .shift_left(shift_amount);
    }

    pub fn one(&mut self, i1: usize) {
        self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]].fill(false);
        self.shared_bits[self.offsets[i1]..self.offsets[i1] + 1].fill(true); // little endian
    }

    pub fn ones(&mut self, i1: usize) {
        self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]].fill(true);
    }

    pub fn zero(&mut self, i1: usize) {
        self.shared_bits[self.offsets[i1]..self.offsets[i1 + 1]].fill(false);
    }

    pub fn const_(&mut self, i1: usize, value: Vec<bool>) {
        for i in self.offsets[i1]..self.offsets[i1 + 1] {
            self.shared_bits[i..i + 1].fill(value[i - self.offsets[i1]]);
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_get_set() {
        let node_widths = vec![2, 8, 6];
        let mut s_env = SharedEnvironment::new(node_widths);
        assert!(s_env.get(1) == bits![0, 0]);
        assert!(s_env.get(2) == bits![0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(s_env.get(3) == bits![0, 0, 0, 0, 0, 0]);

        s_env.set(1, bits![0, 1]);
        s_env.set(2, bits![0, 1, 0, 1, 1, 1, 1, 1]);
        s_env.set(3, bits![0, 1, 0, 0, 0, 0]);

        assert!(s_env.get(1) == bits![0, 1]);
        assert!(s_env.get(2) == bits![0, 1, 0, 1, 1, 1, 1, 1]);
        assert!(s_env.get(3) == bits![0, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn test_shift_left() {
        let node_widths = vec![2, 8, 6, 8, 8, 8, 8, 8];
        let mut s_env = SharedEnvironment::new(node_widths);
        assert!(s_env.get(1) == bits![0, 0]);
        assert!(s_env.get(2) == bits![0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(s_env.get(3) == bits![0, 0, 0, 0, 0, 0]);
        s_env.set_vec(1, vec![false, true]);
        s_env
            .set_vec(2, vec![false, true, false, true, true, true, true, true]);
        s_env.set_vec(3, vec![false, true, false, false, false, false]);

        s_env.sll(2, 1, 4);
        s_env.srl(2, 1, 5);
        s_env.rol(2, 1, 6);
        s_env.ror(2, 1, 7);
        s_env.sra(2, 1, 8);
        assert!(s_env.get(4) == bits![0, 0, 0, 1, 0, 1, 1, 1]);
        assert!(s_env.get(5) == bits![0, 1, 1, 1, 1, 1, 0, 0]);
        assert!(s_env.get(6) == bits![1, 1, 0, 1, 0, 1, 1, 1]);
        assert!(s_env.get(7) == bits![0, 1, 1, 1, 1, 1, 0, 1]);
        assert!(s_env.get(8) == bits![0, 1, 1, 1, 1, 1, 1, 1]);
    }

    #[test]
    fn test_add_mul() {
        let node_widths = vec![8, 8, 8, 8, 8];
        let mut s_env = SharedEnvironment::new(node_widths);
        s_env.set(1, bits![1, 1, 0, 0, 0, 0, 0, 0]);
        s_env.set(2, bits![1, 1, 1, 0, 0, 0, 0, 0]);
        s_env.set(3, bits![1, 1, 1, 1, 1, 0, 0, 0]);
        s_env.add(1, 3, 4);
        s_env.mul(1, 2, 5);
        assert!(s_env.get(4) == bits![0, 1, 0, 0, 0, 1, 0, 0]);
        assert!(s_env.get(5) == bits![1, 0, 1, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn test_bitwise() {
        let node_widths = vec![4, 4, 4, 4, 4, 4, 4, 4];
        let mut s_env = SharedEnvironment::new(node_widths);
        s_env.set(1, bits![0, 1, 0, 1]);
        s_env.set(2, bits![0, 0, 1, 1]);
        s_env.and(1, 2, 3);
        s_env.nand(1, 2, 4);
        s_env.or(1, 2, 5);
        s_env.nor(1, 2, 6);
        s_env.xor(1, 2, 7);
        s_env.xnor(1, 2, 8);
        assert!(s_env.get(3) == bits![0, 0, 0, 1]);
        assert!(s_env.get(4) == bits![1, 1, 1, 0]);
        assert!(s_env.get(5) == bits![0, 1, 1, 1]);
        assert!(s_env.get(6) == bits![1, 0, 0, 0]);
        assert!(s_env.get(7) == bits![0, 1, 1, 0]);
        assert!(s_env.get(8) == bits![1, 0, 0, 1]);
    }

    #[test]
    fn test_comparisons() {
        let node_widths = vec![4, 4, 1, 1];
        let mut s_env = SharedEnvironment::new(node_widths);
        s_env.set(1, bits![0, 1, 0, 1]);
        s_env.set(2, bits![0, 0, 1, 0]);
        s_env.sgt(1, 2, 3);
        s_env.ugt(1, 2, 4);
        assert!(s_env.get(3) == bits![0]);
        assert!(s_env.get(4) == bits![1]);
    }
}

/**
 *  Btor2Tools: A tool package for the BTOR format.
 *
 *  Copyright (c) 2013-2016 Mathias Preiner.
 *  Copyright (c) 2015-2018 Aina Niemetz.
 *  Copyright (c) 2018 Armin Biere.
 *
 *  All rights reserved.
 *
 *  This file is part of the Btor2Tools package.
 *  See LICENSE.txt for more information on using this software.
 */

#ifndef BTOR2BV_H_INCLUDED
#define BTOR2BV_H_INCLUDED

#include <stdbool.h>
#include <stdint.h>
#include "btorsimrng.h"
#include "util/btor2stack.h"

#define BTORSIM_BV_TYPE uint32_t
#define BTORSIM_BV_TYPE_BW (sizeof (BTORSIM_BV_TYPE) * 8)

struct BtorSimBitVector
{
  uint32_t width; /* length of bit vector */
  uint32_t len;   /* length of 'bits' array */

  /* 'bits' represents the bit vector in 32-bit chunks, first bit of 32-bit bv
   * in bits[0] is MSB, bit vector is 'filled' from LSB, hence spare bits (if
   * any) come in front of the MSB and are zeroed out.
   * E.g., for a bit vector of width 31, representing value 1:
   *
   *    bits[0] = 0 0000....1
   *              ^ ^--- MSB
   *              |--- spare bit
   * */
  BTORSIM_BV_TYPE bits[];
};

typedef struct BtorSimBitVector BtorSimBitVector;

BTOR2_DECLARE_STACK (BtorSimBitVectorPtr, BtorSimBitVector *);

BtorSimBitVector *btorsim_bv_new (uint32_t bw);

BtorSimBitVector *btorsim_bv_new_random (BtorSimRNG *rng, uint32_t bw);

BtorSimBitVector *btorsim_bv_new_random_bit_range (BtorSimRNG *rng,
                                                   uint32_t bw,
                                                   uint32_t up,
                                                   uint32_t lo);

BtorSimBitVector *btorsim_bv_char_to_bv (const char *assignment);

BtorSimBitVector *btorsim_bv_dec_to_bv (const char *decimal_string,
                                        uint32_t bw);

BtorSimBitVector *btorsim_bv_uint64_to_bv (uint64_t value, uint32_t bw);

BtorSimBitVector *btorsim_bv_int64_to_bv (int64_t value, uint32_t bw);

BtorSimBitVector *btorsim_bv_const (const char *str, uint32_t bw);

BtorSimBitVector *btorsim_bv_constd (const char *str, uint32_t bw);

BtorSimBitVector *btorsim_bv_consth (const char *str, uint32_t bw);

BtorSimBitVector *btorsim_bv_copy (const BtorSimBitVector *bv);

/*------------------------------------------------------------------------*/

size_t btorsim_bv_size (const BtorSimBitVector *bv);
void btorsim_bv_free (BtorSimBitVector *bv);
int32_t btorsim_bv_compare (const BtorSimBitVector *a,
                            const BtorSimBitVector *b);
uint32_t btorsim_bv_hash (const BtorSimBitVector *bv);

void btorsim_bv_print (const BtorSimBitVector *bv);
void btorsim_bv_print_all (const BtorSimBitVector *bv);
void btorsim_bv_print_without_new_line (const BtorSimBitVector *bv);

char *btorsim_bv_to_char (const BtorSimBitVector *bv);
char *btorsim_bv_to_hex_char (const BtorSimBitVector *bv);
char *btorsim_bv_to_dec_char (const BtorSimBitVector *bv);

uint64_t btorsim_bv_to_uint64 (const BtorSimBitVector *bv);

/*------------------------------------------------------------------------*/

/* index 0 is LSB, width - 1 is MSB */
uint32_t btorsim_bv_get_bit (const BtorSimBitVector *bv, uint32_t pos);
/* index 0 is LSB, width - 1 is MSB */
void btorsim_bv_set_bit (BtorSimBitVector *bv, uint32_t pos, uint32_t value);

void btorsim_bv_flip_bit (BtorSimBitVector *bv, uint32_t pos);

bool btorsim_bv_is_true (const BtorSimBitVector *bv);
bool btorsim_bv_is_false (const BtorSimBitVector *bv);

bool btorsim_bv_is_zero (const BtorSimBitVector *bv);
bool btorsim_bv_is_ones (const BtorSimBitVector *bv);
bool btorsim_bv_is_one (const BtorSimBitVector *bv);

/* return p for bv = 2^p, and -1 if bv is not a power of 2 */
int64_t btorsim_bv_power_of_two (const BtorSimBitVector *bv);
/* return bv as integer if its value can be converted into a positive
 * integer of bw 32, and -1 otherwise */
int32_t btorsim_bv_small_positive_int (const BtorSimBitVector *bv);

/* count trailing zeros (starting from LSB) */
uint32_t btorsim_bv_get_num_trailing_zeros (const BtorSimBitVector *bv);
/* count leading zeros (starting from MSB) */
uint32_t btorsim_bv_get_num_leading_zeros (const BtorSimBitVector *bv);
/* count leading ones (starting from MSB) */
uint32_t btorsim_bv_get_num_leading_ones (const BtorSimBitVector *bv);

/*------------------------------------------------------------------------*/

#define btorsim_bv_zero(BW) btorsim_bv_new (BW)

BtorSimBitVector *btorsim_bv_one (uint32_t bw);
BtorSimBitVector *btorsim_bv_ones (uint32_t bw);

BtorSimBitVector *btorsim_bv_neg (const BtorSimBitVector *bv);
BtorSimBitVector *btorsim_bv_not (const BtorSimBitVector *bv);
BtorSimBitVector *btorsim_bv_inc (const BtorSimBitVector *bv);
BtorSimBitVector *btorsim_bv_dec (const BtorSimBitVector *bv);

BtorSimBitVector *btorsim_bv_redor (const BtorSimBitVector *bv);
BtorSimBitVector *btorsim_bv_redand (const BtorSimBitVector *bv);

BtorSimBitVector *btorsim_bv_add (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_sub (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_and (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_implies (const BtorSimBitVector *a,
                                      const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_nand (const BtorSimBitVector *a,
                                   const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_nor (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_or (const BtorSimBitVector *a,
                                 const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_xnor (const BtorSimBitVector *a,
                                   const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_xor (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_eq (const BtorSimBitVector *a,
                                 const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_neq (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_ult (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_ulte (const BtorSimBitVector *a,
                                   const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_slt (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_slte (const BtorSimBitVector *a,
                                   const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_sll (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_srl (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_sra (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_mul (const BtorSimBitVector *a,
                                  const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_udiv (const BtorSimBitVector *a,
                                   const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_sdiv (const BtorSimBitVector *a,
                                   const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_urem (const BtorSimBitVector *a,
                                   const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_srem (const BtorSimBitVector *a,
                                   const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_ite (const BtorSimBitVector *c,
                                  const BtorSimBitVector *t,
                                  const BtorSimBitVector *e);

BtorSimBitVector *btorsim_bv_concat (const BtorSimBitVector *a,
                                     const BtorSimBitVector *b);

BtorSimBitVector *btorsim_bv_slice (const BtorSimBitVector *bv,
                                    uint32_t upper,
                                    uint32_t lower);

BtorSimBitVector *btorsim_bv_uext (const BtorSimBitVector *bv0, uint32_t len);

BtorSimBitVector *btorsim_bv_sext (const BtorSimBitVector *bv0, uint32_t len);

BtorSimBitVector *btorsim_bv_flipped_bit (const BtorSimBitVector *bv,
                                          uint32_t pos);

BtorSimBitVector *btorsim_bv_flipped_bit_range (const BtorSimBitVector *bv,
                                                uint32_t up,
                                                uint32_t lo);

/*------------------------------------------------------------------------*/

bool btorsim_bv_is_umulo (const BtorSimBitVector *bv0,
                          const BtorSimBitVector *bv1);

/*------------------------------------------------------------------------*/

#endif

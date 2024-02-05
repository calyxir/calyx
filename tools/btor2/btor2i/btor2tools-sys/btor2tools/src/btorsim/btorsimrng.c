/**
 *  Btor2Tools: A tool package for the BTOR format.
 *
 *  Copyright (c) 2015-2018 Aina Niemetz.
 *
 *  All rights reserved.
 *
 *  This file is part of the Btor2Tools package.
 *  See LICENSE.txt for more information on using this software.
 */

#include "btorsimrng.h"

#include <assert.h>
#include <limits.h>

void
btorsim_rng_init (BtorSimRNG* rng, uint32_t seed)
{
  assert (rng);

  rng->w = seed;
  rng->z = ~rng->w;
  rng->w <<= 1;
  rng->z <<= 1;
  rng->w += 1;
  rng->z += 1;
  rng->w *= 2019164533u;
  rng->z *= 1000632769u;
}

uint32_t
btorsim_rng_rand (BtorSimRNG* rng)
{
  assert (rng);
  rng->z = 36969 * (rng->z & 65535) + (rng->z >> 16);
  rng->w = 18000 * (rng->w & 65535) + (rng->w >> 16);
  return (rng->z << 16) + rng->w; /* 32-bit result */
}

uint32_t
btorsim_rng_pick_rand (BtorSimRNG* rng, uint32_t from, uint32_t to)
{
  assert (rng);
  assert (from <= to);

  uint32_t res;

  from = from == UINT_MAX ? UINT_MAX - 1 : from;
  to   = to == UINT_MAX ? UINT_MAX - 1 : to;
  res  = btorsim_rng_rand (rng);
  res %= to - from + 1;
  res += from;
  return res;
}

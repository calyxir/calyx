/**
 *  Btor2Tools: A tool package for the BTOR format.
 *
 *  Copyright (c) 2013-2019 Mathias Preiner.
 *  Copyright (c) 2015-2018 Aina Niemetz.
 *  Copyright (c) 2018 Armin Biere.
 *
 *  All rights reserved.
 *
 *  This file is part of the Btor2Tools package.
 *  See LICENSE.txt for more information on using this software.
 */

#include "btorsimbv.h"
#include "util/btor2mem.h"

#include <assert.h>
#include <ctype.h>
#include <limits.h>

#define BTOR2_MASK_REM_BITS(bv)                            \
  ((((BTORSIM_BV_TYPE) 1 << (BTORSIM_BV_TYPE_BW - 1)) - 1) \
   >> (BTORSIM_BV_TYPE_BW - 1 - (bv->width % BTORSIM_BV_TYPE_BW)))

/*------------------------------------------------------------------------*/

#ifndef NDEBUG
static bool
rem_bits_zero_dbg (BtorSimBitVector *bv)
{
  return (bv->width % BTORSIM_BV_TYPE_BW == 0
          || (bv->bits[0] >> (bv->width % BTORSIM_BV_TYPE_BW) == 0));
}

static bool
check_bits_sll_dbg (const BtorSimBitVector *bv,
                    const BtorSimBitVector *res,
                    uint32_t shift)
{
  assert (bv);
  assert (res);
  assert (bv->width == res->width);

  uint32_t i;

  if (shift >= bv->width)
  {
    for (i = 0; i < bv->width; i++) assert (btorsim_bv_get_bit (bv, i) == 0);
  }
  else
  {
    for (i = 0; shift + i < bv->width; i++)
      assert (btorsim_bv_get_bit (bv, i)
              == btorsim_bv_get_bit (res, shift + i));
  }

  return true;
}
#endif

static void
set_rem_bits_to_zero (BtorSimBitVector *bv)
{
  if (bv->width != BTORSIM_BV_TYPE_BW * bv->len)
    bv->bits[0] &= BTOR2_MASK_REM_BITS (bv);
}

/*------------------------------------------------------------------------*/

BtorSimBitVector *
btorsim_bv_new (uint32_t bw)
{
  assert (bw > 0);

  uint32_t i;
  BtorSimBitVector *res;

  i = bw / BTORSIM_BV_TYPE_BW;
  if (bw % BTORSIM_BV_TYPE_BW > 0) i += 1;

  assert (i > 0);
  res =
      btorsim_malloc (sizeof (BtorSimBitVector) + sizeof (BTORSIM_BV_TYPE) * i);
  memset (res->bits, 0, i * sizeof *(res->bits));
  res->len = i;
  assert (res->len);
  res->width = bw;
  return res;
}

BtorSimBitVector *
btorsim_bv_new_random_bit_range (BtorSimRNG *rng,
                                 uint32_t bw,
                                 uint32_t up,
                                 uint32_t lo)
{
  assert (rng);
  assert (bw > 0);
  assert (lo <= up);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (bw);
  for (i = 1; i < res->len; i++)
    res->bits[i] = (BTORSIM_BV_TYPE) btorsim_rng_rand (rng);
  res->bits[0] = (BTORSIM_BV_TYPE) btorsim_rng_pick_rand (
      rng, 0, ((~0) >> (BTORSIM_BV_TYPE_BW - bw % BTORSIM_BV_TYPE_BW)) - 1);

  for (i = 0; i < lo; i++) btorsim_bv_set_bit (res, i, 0);
  for (i = up + 1; i < res->width; i++) btorsim_bv_set_bit (res, i, 0);

  set_rem_bits_to_zero (res);

  return res;
}

BtorSimBitVector *
btorsim_bv_new_random (BtorSimRNG *rng, uint32_t bw)
{
  return btorsim_bv_new_random_bit_range (rng, bw, bw - 1, 0);
}

void
btorsim_bv_free (BtorSimBitVector *bv)
{
  assert (bv);
  BTOR2_DELETE (bv);
}

/*------------------------------------------------------------------------*/

BtorSimBitVector *
btorsim_bv_char_to_bv (const char *assignment)
{
  assert (assignment);
  assert (strlen (assignment) > 0);

  return btorsim_bv_const (assignment, strlen (assignment));
}

BtorSimBitVector *
btorsim_bv_uint64_to_bv (uint64_t value, uint32_t bw)
{
  assert (bw > 0);

  BtorSimBitVector *res;

  res = btorsim_bv_new (bw);
  assert (res->len > 0);
  res->bits[res->len - 1] = (BTORSIM_BV_TYPE) value;
  if (res->width > 32)
    res->bits[res->len - 2] = (BTORSIM_BV_TYPE) (value >> BTORSIM_BV_TYPE_BW);

  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_int64_to_bv (int64_t value, uint32_t bw)
{
  assert (bw > 0);

  BtorSimBitVector *res, *tmp;

  res = btorsim_bv_new (bw);
  assert (res->len > 0);

  /* ensure that all bits > 64 are set to 1 in case of negative values */
  if (value < 0 && bw > 64)
  {
    tmp = btorsim_bv_not (res);
    free (res);
    res = tmp;
  }

  res->bits[res->len - 1] = (BTORSIM_BV_TYPE) value;
  if (res->width > 32)
    res->bits[res->len - 2] = (BTORSIM_BV_TYPE) (value >> BTORSIM_BV_TYPE_BW);

  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_const (const char *str, uint32_t bw)
{
  assert (strlen (str) <= bw);

  uint32_t i, j, bit;
  BtorSimBitVector *res;

  res = btorsim_bv_new (bw);
  for (i = 0; i < bw; i++)
  {
    j = bw - 1 - i;
    assert (str[j] == '0' || str[j] == '1');
    bit = str[j] == '0' ? 0 : 1;
    btorsim_bv_set_bit (res, i, bit);
  }
  return res;
}

#ifndef NDEBUG
static int32_t
is_bin_str (const char *c)
{
  const char *p;
  char ch;

  assert (c != NULL);

  for (p = c; (ch = *p); p++)
    if (ch != '0' && ch != '1') return 0;
  return 1;
}
#endif

static const char *
strip_zeroes (const char *a)
{
  assert (a);
  while (*a == '0') a++;
  return a;
}

static char *
mult_unbounded_bin_str (const char *a, const char *b)
{
  assert (a);
  assert (b);
  assert (is_bin_str (a));
  assert (is_bin_str (b));

  char *res, *r, c, x, y, s, m;
  uint32_t alen, blen, rlen, i;
  const char *p;

  a = strip_zeroes (a);
  if (!*a) return btorsim_strdup ("");
  if (a[0] == '1' && !a[1]) return btorsim_strdup (b);

  b = strip_zeroes (b);
  if (!*b) return btorsim_strdup ("");
  if (b[0] == '1' && !b[1]) return btorsim_strdup (a);

  alen = strlen (a);
  blen = strlen (b);
  rlen = alen + blen;
  BTOR2_NEWN (res, rlen + 1);
  res[rlen] = 0;

  for (r = res; r < res + blen; r++) *r = '0';
  for (p = a; p < a + alen; p++) *r++ = *p;
  assert (r == res + rlen);

  for (i = 0; i < alen; i++)
  {
    m = res[rlen - 1];
    c = '0';

    if (m == '1')
    {
      p = b + blen;
      r = res + blen;

      while (res < r && b < p)
      {
        assert (b < p);
        x  = *--p;
        y  = *--r;
        s  = x ^ y ^ c;
        c  = (x & y) | (x & c) | (y & c);
        *r = s;
      }
    }

    memmove (res + 1, res, rlen - 1);
    res[0] = c;
  }

  return res;
}

static char *
add_unbounded_bin_str (const char *a, const char *b)
{
  assert (a);
  assert (b);
  assert (is_bin_str (a));
  assert (is_bin_str (b));

  char *res, *r, c, x, y, s, *tmp;
  uint32_t alen, blen, rlen;
  const char *p, *q;

  a = strip_zeroes (a);
  b = strip_zeroes (b);

  if (!*a) return btorsim_strdup (b);
  if (!*b) return btorsim_strdup (a);

  alen = strlen (a);
  blen = strlen (b);
  rlen = (alen < blen) ? blen : alen;
  rlen++;

  BTOR2_NEWN (res, rlen + 1);

  p = a + alen;
  q = b + blen;

  c = '0';

  r  = res + rlen;
  *r = 0;

  while (res < r)
  {
    x    = (a < p) ? *--p : '0';
    y    = (b < q) ? *--q : '0';
    s    = x ^ y ^ c;
    c    = (x & y) | (x & c) | (y & c);
    *--r = s;
  }

  p = strip_zeroes (res);
  if ((p != res))
  {
    tmp = btorsim_strdup (p);
    if (!tmp)
    {
      free (res);
      return 0;
    }
    free (res);
    res = tmp;
  }

  return res;
}

static const char *digit2const_table[10] = {
    "",
    "1",
    "10",
    "11",
    "100",
    "101",
    "110",
    "111",
    "1000",
    "1001",
};

static const char *
digit2const (char ch)
{
  assert ('0' <= ch);
  assert (ch <= '9');

  return digit2const_table[ch - '0'];
}

static char *
dec_to_bin_str (const char *str, uint32_t len)
{
  assert (str);

  const char *end, *p;
  char *res, *tmp;

  res = btorsim_strdup ("");
  if (!res) return 0;

  end = str + len;
  for (p = str; p < end; p++)
  {
    tmp = mult_unbounded_bin_str (res, "1010"); /* *10 */
    if (!tmp)
    {
      free (res);
      return 0;
    }
    free (res);
    res = tmp;

    tmp = add_unbounded_bin_str (res, digit2const (*p));
    if (!tmp)
    {
      free (res);
      return 0;
    }
    free (res);
    res = tmp;
  }

  assert (strip_zeroes (res) == res);
  if (strlen (res)) return res;
  free (res);
  return btorsim_strdup ("0");
}

#ifndef NDEBUG
static int32_t
check_constd (const char *str, uint32_t width)
{
  assert (str);
  assert (width);

  int32_t is_neg, is_min_val = 0, res;
  char *bits;
  size_t size_bits, len;

  is_neg    = (str[0] == '-');
  len       = is_neg ? strlen (str) - 1 : strlen (str);
  bits      = dec_to_bin_str (is_neg ? str + 1 : str, len);
  size_bits = strlen (bits);
  if (is_neg)
  {
    is_min_val = (bits[0] == '1');
    for (size_t i = 1; is_min_val && i < size_bits; i++)
      is_min_val = (bits[i] == '0');
  }
  res = ((is_neg && !is_min_val) || size_bits <= width)
        && (!is_neg || is_min_val || size_bits + 1 <= width);
  free (bits);
  return res;
}
#endif

BtorSimBitVector *
btorsim_bv_constd (const char *str, uint32_t bw)
{
  assert (check_constd (str, bw));

  bool is_neg, is_min_val;
  ;
  BtorSimBitVector *res, *tmp;
  char *bits;
  uint32_t size_bits, len;

  is_min_val = false;
  is_neg     = (str[0] == '-');
  len        = is_neg ? strlen (str) - 1 : strlen (str);
  bits       = dec_to_bin_str (is_neg ? str + 1 : str, len);
  size_bits  = strlen (bits);
  if (is_neg)
  {
    is_min_val = (bits[0] == '1');
    for (size_t i = 1; is_min_val && i < size_bits; i++)
      is_min_val = (bits[i] == '0');
  }
  assert (((is_neg && !is_min_val) || size_bits <= bw)
          && (!is_neg || is_min_val || size_bits + 1 <= bw));

  res = btorsim_bv_char_to_bv (bits);
  free (bits);
  assert (res->width == size_bits);
  /* zero-extend to bw */
  if (size_bits < bw)
  {
    tmp = btorsim_bv_uext (res, bw - size_bits);
    free (res);
    res = tmp;
  }
  if (is_neg)
  {
    tmp = btorsim_bv_neg (res);
    free (res);
    res = tmp;
  }
  return res;
}

#ifndef NDEBUG
static int32_t
check_consth (const char *consth, uint32_t width)
{
  char c;
  size_t i, len, req_width;

  len       = strlen (consth);
  req_width = len * 4;
  for (i = 0; i < len; i++)
  {
    c = consth[i];
    assert (isxdigit (c));
    if (c >= 'A' && c <= 'F') c = tolower (c);

    if (c == '0')
    {
      req_width -= 4;
      continue;
    }
    c = (c >= '0' && c <= '9') ? c - '0' : c - 'a' + 0xa;
    assert (c > 0 && c <= 15);
    if (c >> 1 == 0)
      req_width -= 3;
    else if (c >> 2 == 0)
      req_width -= 2;
    else if (c >> 3 == 0)
      req_width -= 1;
    break;
  }
  if (req_width <= width) return 1;
  return 0;
}
#endif

static char *
hex_to_bin_str (const char *str)
{
  assert (str);

  const char *p, *end;
  char *tmp, *res, *q;
  uint32_t len, blen;

  len  = strlen (str);
  blen = 4 * len;
  BTOR2_NEWN (tmp, blen + 1);
  q = tmp;

  end = str + len;
  for (p = str; p < end; p++) switch (*p)
    {
      case '0':
        *q++ = '0';
        *q++ = '0';
        *q++ = '0';
        *q++ = '0';
        break;
      case '1':
        *q++ = '0';
        *q++ = '0';
        *q++ = '0';
        *q++ = '1';
        break;
      case '2':
        *q++ = '0';
        *q++ = '0';
        *q++ = '1';
        *q++ = '0';
        break;
      case '3':
        *q++ = '0';
        *q++ = '0';
        *q++ = '1';
        *q++ = '1';
        break;
      case '4':
        *q++ = '0';
        *q++ = '1';
        *q++ = '0';
        *q++ = '0';
        break;
      case '5':
        *q++ = '0';
        *q++ = '1';
        *q++ = '0';
        *q++ = '1';
        break;
      case '6':
        *q++ = '0';
        *q++ = '1';
        *q++ = '1';
        *q++ = '0';
        break;
      case '7':
        *q++ = '0';
        *q++ = '1';
        *q++ = '1';
        *q++ = '1';
        break;
      case '8':
        *q++ = '1';
        *q++ = '0';
        *q++ = '0';
        *q++ = '0';
        break;
      case '9':
        *q++ = '1';
        *q++ = '0';
        *q++ = '0';
        *q++ = '1';
        break;
      case 'A':
      case 'a':
        *q++ = '1';
        *q++ = '0';
        *q++ = '1';
        *q++ = '0';
        break;
      case 'B':
      case 'b':
        *q++ = '1';
        *q++ = '0';
        *q++ = '1';
        *q++ = '1';
        break;
      case 'C':
      case 'c':
        *q++ = '1';
        *q++ = '1';
        *q++ = '0';
        *q++ = '0';
        break;
      case 'D':
      case 'd':
        *q++ = '1';
        *q++ = '1';
        *q++ = '0';
        *q++ = '1';
        break;
      case 'E':
      case 'e':
        *q++ = '1';
        *q++ = '1';
        *q++ = '1';
        *q++ = '0';
        break;
      case 'F':
      case 'f':
      default:
        assert (*p == 'f' || *p == 'F');
        *q++ = '1';
        *q++ = '1';
        *q++ = '1';
        *q++ = '1';
        break;
    }

  assert (tmp + blen == q);
  *q++ = 0;

  res = btorsim_strdup (strip_zeroes (tmp));
  free (tmp);

  if (strlen (res)) return res;
  free (res);
  return btorsim_strdup ("0");
}

BtorSimBitVector *
btorsim_bv_consth (const char *str, uint32_t bw)
{
  assert (check_consth (str, bw));

  BtorSimBitVector *res, *tmp;
  char *bits;
  uint32_t size_bits;

  bits      = hex_to_bin_str (str);
  size_bits = strlen (bits);
  assert (size_bits <= bw);
  res = btorsim_bv_char_to_bv (bits);
  free (bits);
  assert (res->width == size_bits);
  /* zero-extend to bw */
  if (size_bits < bw)
  {
    tmp = btorsim_bv_uext (res, bw - size_bits);
    free (res);
    res = tmp;
  }
  return res;
}

/*------------------------------------------------------------------------*/

BtorSimBitVector *
btorsim_bv_copy (const BtorSimBitVector *bv)
{
  assert (bv);

  BtorSimBitVector *res;

  res = btorsim_bv_new (bv->width);
  assert (res->width == bv->width);
  assert (res->len == bv->len);
  memcpy (res->bits, bv->bits, sizeof (*(bv->bits)) * bv->len);
  assert (btorsim_bv_compare (res, (BtorSimBitVector *) bv) == 0);
  return res;
}

/*------------------------------------------------------------------------*/

size_t
btorsim_bv_size (const BtorSimBitVector *bv)
{
  assert (bv);
  return sizeof (BtorSimBitVector) + bv->len * sizeof (BTORSIM_BV_TYPE);
}

int32_t
btorsim_bv_compare (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);

  uint32_t i;

  if (a->width != b->width) return -1;

  /* find index on which a and b differ */
  for (i = 0; i < a->len && a->bits[i] == b->bits[i]; i++)
    ;

  if (i == a->len) return 0;

  if (a->bits[i] > b->bits[i]) return 1;

  assert (a->bits[i] < b->bits[i]);
  return -1;
}

static uint32_t hash_primes[] = {333444569u, 76891121u, 456790003u};

#define NPRIMES ((uint32_t) (sizeof hash_primes / sizeof *hash_primes))

uint32_t
btorsim_bv_hash (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t res = 0, i, j = 0, x, p0, p1;

  res = bv->width * hash_primes[j++];
  for (i = 0, j = 0; i < bv->len; i++)
  {
    p0 = hash_primes[j++];
    if (j == NPRIMES) j = 0;
    p1 = hash_primes[j++];
    if (j == NPRIMES) j = 0;
    x   = bv->bits[i] ^ res;
    x   = ((x >> 16) ^ x) * p0;
    x   = ((x >> 16) ^ x) * p1;
    res = ((x >> 16) ^ x);
  }
  return res;
}

/*------------------------------------------------------------------------*/

void
btorsim_bv_print_without_new_line (const BtorSimBitVector *bv)
{
  assert (bv);

  int64_t i;

  for (i = bv->width - 1; i >= 0; i--)
    printf ("%d", btorsim_bv_get_bit (bv, i));
}

void
btorsim_bv_print (const BtorSimBitVector *bv)
{
  btorsim_bv_print_without_new_line (bv);
  printf ("\n");
}

void
btorsim_bv_print_all (const BtorSimBitVector *bv)
{
  assert (bv);

  int64_t i;

  for (i = BTORSIM_BV_TYPE_BW * bv->len - 1; i >= 0; i--)
  {
    if ((uint32_t) i == (BTORSIM_BV_TYPE_BW * bv->len + 1 - bv->width))
      printf ("|");
    if (i > 0
        && (BTORSIM_BV_TYPE_BW * bv->len - 1 - i) % BTORSIM_BV_TYPE_BW == 0)
      printf (".");
    printf ("%d", btorsim_bv_get_bit (bv, i));
  }
  printf ("\n");
}

/*------------------------------------------------------------------------*/

char *
btorsim_bv_to_char (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i, bw, bit;
  char *res;

  bw = bv->width;
  BTOR2_NEWN (res, bw + 1);
  for (i = 0; i < bw; i++)
  {
    bit             = btorsim_bv_get_bit (bv, i);
    res[bw - 1 - i] = bit ? '1' : '0';
  }
  res[bw] = '\0';

  return res;
}

char *
btorsim_bv_to_hex_char (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t len, i, j, k, tmp;
  char *res, ch;

  len = (bv->width + 3) / 4;
  BTOR2_CNEWN (res, len + 1);
  for (i = 0, j = len - 1; i < bv->width;)
  {
    tmp = btorsim_bv_get_bit (bv, i++);
    for (k = 1; i < bv->width && k <= 3; i++, k++)
      tmp |= btorsim_bv_get_bit (bv, i) << k;
    ch       = tmp < 10 ? '0' + tmp : 'a' + (tmp - 10);
    res[j--] = ch;
  }

  return res;
}

static uint32_t
get_first_one_bit_idx (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i;

  for (i = bv->width - 1; i < UINT_MAX; i--)
  {
    if (btorsim_bv_get_bit (bv, i)) break;
    if (i == 0) return UINT_MAX;
  }
  return i;
}

char *
btorsim_bv_to_dec_char (const BtorSimBitVector *bv)
{
  assert (bv);

  BtorSimBitVector *tmp, *div, *rem, *ten;
  uint32_t i;
  char *res, ch, *p, *q;
  BtorCharStack stack;

  if (btorsim_bv_is_zero (bv))
  {
    BTOR2_CNEWN (res, 2);
    res[0] = '0';
    return res;
  }

  BTOR2_INIT_STACK (stack);

  if (bv->width < 4)
  {
    ten = btorsim_bv_uint64_to_bv (10, 4);
    tmp = btorsim_bv_uext ((BtorSimBitVector *) bv, 4 - bv->width);
  }
  else
  {
    ten = btorsim_bv_uint64_to_bv (10, bv->width);
    tmp = btorsim_bv_copy (bv);
  }
  while (!btorsim_bv_is_zero (tmp))
  {
    div = btorsim_bv_udiv (tmp, ten);
    rem = btorsim_bv_urem (tmp, ten);
    ch  = 0;
    for (i = get_first_one_bit_idx (rem); i < UINT_MAX; i--)
    {
      ch <<= 1;
      if (btorsim_bv_get_bit (rem, i)) ch += 1;
    }
    assert (ch < 10);
    ch += '0';
    BTOR2_PUSH_STACK (stack, ch);
    free (rem);
    free (tmp);
    tmp = div;
  }
  free (tmp);
  free (ten);
  if (BTOR2_EMPTY_STACK (stack)) BTOR2_PUSH_STACK (stack, '0');
  BTOR2_NEWN (res, BTOR2_COUNT_STACK (stack) + 1);
  q = res;
  p = stack.top;
  while (p > stack.start) *q++ = *--p;
  assert (res + BTOR2_COUNT_STACK (stack) == q);
  *q = 0;
  assert ((uint32_t) BTOR2_COUNT_STACK (stack) == strlen (res));
  BTOR2_RELEASE_STACK (stack);
  return res;
}

/*------------------------------------------------------------------------*/

uint64_t
btorsim_bv_to_uint64 (const BtorSimBitVector *bv)
{
  assert (bv);
  assert (bv->width <= sizeof (uint64_t) * 8);
  assert (bv->len <= 2);

  uint32_t i;
  uint64_t res;

  res = 0;
  for (i = 0; i < bv->len; i++)
    res |= ((uint64_t) bv->bits[i]) << (BTORSIM_BV_TYPE_BW * (bv->len - 1 - i));

  return res;
}

/*------------------------------------------------------------------------*/

uint32_t
btorsim_bv_get_bit (const BtorSimBitVector *bv, uint32_t pos)
{
  assert (bv);

  uint32_t i, j;

  i = pos / BTORSIM_BV_TYPE_BW;
  j = pos % BTORSIM_BV_TYPE_BW;

  return (bv->bits[bv->len - 1 - i] >> j) & 1;
}

void
btorsim_bv_set_bit (BtorSimBitVector *bv, uint32_t pos, uint32_t bit)
{
  assert (bv);
  assert (bv->len > 0);
  assert (bit == 0 || bit == 1);
  assert (pos < bv->width);

  uint32_t i, j;

  i = pos / BTORSIM_BV_TYPE_BW;
  j = pos % BTORSIM_BV_TYPE_BW;
  assert (i < bv->len);

  if (bit)
    bv->bits[bv->len - 1 - i] |= (1u << j);
  else
    bv->bits[bv->len - 1 - i] &= ~(1u << j);
}

void
btorsim_bv_flip_bit (BtorSimBitVector *bv, uint32_t pos)
{
  assert (bv);
  assert (bv->len > 0);
  assert (pos < bv->width);

  btorsim_bv_set_bit (bv, pos, btorsim_bv_get_bit (bv, pos) ? 0 : 1);
}

/*------------------------------------------------------------------------*/

bool
btorsim_bv_is_true (const BtorSimBitVector *bv)
{
  assert (bv);

  if (bv->width != 1) return 0;
  return btorsim_bv_get_bit (bv, 0);
}

bool
btorsim_bv_is_false (const BtorSimBitVector *bv)
{
  assert (bv);

  if (bv->width != 1) return 0;
  return !btorsim_bv_get_bit (bv, 0);
}

bool
btorsim_bv_is_zero (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i;
  for (i = 0; i < bv->len; i++)
    if (bv->bits[i] != 0) return false;
  return true;
}

bool
btorsim_bv_is_ones (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i, n;
  for (i = bv->len - 1; i >= 1; i--)
    if (bv->bits[i] != UINT_MAX) return false;
  if (bv->width == BTORSIM_BV_TYPE_BW)
    return bv->bits[0] == UINT_MAX;
  else
  {
    n = BTORSIM_BV_TYPE_BW - bv->width % BTORSIM_BV_TYPE_BW;
    assert (n > 0);
    if (bv->bits[0] != UINT_MAX >> n) return false;
  }
  return true;
}

bool
btorsim_bv_is_one (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i;

  if (bv->bits[bv->len - 1] != 1) return false;
  for (i = 0; i < bv->len - 1; i++)
    if (bv->bits[i] != 0) return false;
  return true;
}

int64_t
btorsim_bv_power_of_two (const BtorSimBitVector *bv)
{
  assert (bv);

  int64_t i, j;
  uint32_t bit;
  bool iszero;

  for (i = 0, j = 0, iszero = true; i < bv->width; i++)
  {
    bit = btorsim_bv_get_bit (bv, i);
    if (!bit) continue;
    if (bit && !iszero) return -1;
    assert (bit && iszero);
    j      = i;
    iszero = false;
  }
  return j;
}

int32_t
btorsim_bv_small_positive_int (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i;

  for (i = 0; i < bv->len - 1; i++)
    if (bv->bits[i] != 0) return -1;
  if (((int32_t) bv->bits[bv->len - 1]) < 0) return -1;
  return bv->bits[bv->len - 1];
}

uint32_t
btorsim_bv_get_num_trailing_zeros (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i, res;

  for (i = 0, res = 0; i < bv->width; i++)
  {
    if (btorsim_bv_get_bit (bv, i)) break;
    res += 1;
  }

  return res;
}

uint32_t
btorsim_bv_get_num_leading_zeros (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i, res;

  for (i = bv->width - 1, res = 0; i < UINT_MAX; i--)
  {
    if (btorsim_bv_get_bit (bv, i)) break;
    res += 1;
  }

  return res;
}

uint32_t
btorsim_bv_get_num_leading_ones (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i, res;

  for (i = bv->width - 1, res = 0; i < UINT_MAX; i--)
  {
    if (!btorsim_bv_get_bit (bv, i)) break;
    res += 1;
  }

  return res;
}

/*------------------------------------------------------------------------*/

BtorSimBitVector *
btorsim_bv_one (uint32_t bw)
{
  assert (bw);

  BtorSimBitVector *res = btorsim_bv_new (bw);
  btorsim_bv_set_bit (res, 0, 1);
  return res;
}

BtorSimBitVector *
btorsim_bv_ones (uint32_t bw)
{
  assert (bw);

  BtorSimBitVector *res, *tmp;

  tmp = btorsim_bv_new (bw);
  res = btorsim_bv_not (tmp);
  free (tmp);

  return res;
}

BtorSimBitVector *
btorsim_bv_neg (const BtorSimBitVector *bv)
{
  assert (bv);

  BtorSimBitVector *not_bv, *one, *neg_b;

  not_bv = btorsim_bv_not (bv);
  one    = btorsim_bv_one (bv->width);
  neg_b  = btorsim_bv_add (not_bv, one);
  free (not_bv);
  free (one);

  return neg_b;
}

BtorSimBitVector *
btorsim_bv_not (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (bv->width);
  for (i = 0; i < bv->len; i++) res->bits[i] = ~bv->bits[i];

  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_inc (const BtorSimBitVector *bv)
{
  assert (bv);

  BtorSimBitVector *res, *one;

  one = btorsim_bv_one (bv->width);
  res = btorsim_bv_add (bv, one);
  free (one);
  return res;
}

BtorSimBitVector *
btorsim_bv_dec (const BtorSimBitVector *bv)
{
  assert (bv);

  BtorSimBitVector *res, *one, *negone;

  one    = btorsim_bv_one (bv->width);
  negone = btorsim_bv_neg (one);
  res    = btorsim_bv_add (bv, negone);
  free (one);
  free (negone);
  return res;
}

BtorSimBitVector *
btorsim_bv_redand (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i;
  uint32_t bit;
  uint32_t mask0;
  BtorSimBitVector *res;

  res = btorsim_bv_new (1);
  assert (rem_bits_zero_dbg (res));

  if (bv->width == BTORSIM_BV_TYPE_BW * bv->len)
    mask0 = ~(BTORSIM_BV_TYPE) 0;
  else
    mask0 = BTOR2_MASK_REM_BITS (bv);

  bit = (bv->bits[0] == mask0);

  for (i = 1; bit && i < bv->len; i++)
    if (bv->bits[i] != ~(BTORSIM_BV_TYPE) 0) bit = 0;

  btorsim_bv_set_bit (res, 0, bit);

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_redor (const BtorSimBitVector *bv)
{
  assert (bv);

  uint32_t i;
  uint32_t bit;
  BtorSimBitVector *res;

  res = btorsim_bv_new (1);
  assert (rem_bits_zero_dbg (res));
  bit = 0;
  for (i = 0; !bit && i < bv->len; i++)
    if (bv->bits[i]) bit = 1;

  btorsim_bv_set_bit (res, 0, bit);

  assert (rem_bits_zero_dbg (res));
  return res;
}

/*------------------------------------------------------------------------*/

BtorSimBitVector *
btorsim_bv_add (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  int64_t i;
  uint64_t x, y, sum;
  BtorSimBitVector *res;
  BTORSIM_BV_TYPE carry;

  if (a->width <= 64)
  {
    x   = btorsim_bv_to_uint64 (a);
    y   = btorsim_bv_to_uint64 (b);
    res = btorsim_bv_uint64_to_bv (x + y, a->width);
  }
  else
  {
    res   = btorsim_bv_new (a->width);
    carry = 0;
    for (i = a->len - 1; i >= 0; i--)
    {
      sum          = (uint64_t) a->bits[i] + b->bits[i] + carry;
      res->bits[i] = (BTORSIM_BV_TYPE) sum;
      carry        = (BTORSIM_BV_TYPE) (sum >> 32);
    }
  }

  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_sub (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  BtorSimBitVector *negb, *res;

  negb = btorsim_bv_neg (b);
  res  = btorsim_bv_add (a, negb);
  free (negb);
  return res;
}

BtorSimBitVector *
btorsim_bv_and (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (a->width);
  for (i = 0; i < a->len; i++) res->bits[i] = a->bits[i] & b->bits[i];

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_implies (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (a->width);
  for (i = 0; i < a->len; i++) res->bits[i] = ~a->bits[i] | b->bits[i];

  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_or (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (a->width);
  for (i = 0; i < a->len; i++) res->bits[i] = a->bits[i] | b->bits[i];

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_nand (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (a->width);
  for (i = 0; i < a->len; i++) res->bits[i] = ~(a->bits[i] & b->bits[i]);

  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_nor (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (a->width);
  for (i = 0; i < a->len; i++) res->bits[i] = ~(a->bits[i] | b->bits[i]);

  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_xnor (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (a->width);
  for (i = 0; i < a->len; i++) res->bits[i] = a->bits[i] ^ ~b->bits[i];

  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_xor (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_new (a->width);
  for (i = 0; i < a->len; i++) res->bits[i] = a->bits[i] ^ b->bits[i];

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_eq (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i, bit;
  BtorSimBitVector *res;

  res = btorsim_bv_new (1);
  bit = 1;
  for (i = 0; i < a->len; i++)
  {
    if (a->bits[i] != b->bits[i])
    {
      bit = 0;
      break;
    }
  }
  btorsim_bv_set_bit (res, 0, bit);

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_neq (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i, bit;
  BtorSimBitVector *res;

  res = btorsim_bv_new (1);
  bit = 1;
  for (i = 0; i < a->len; i++)
  {
    if (a->bits[i] != b->bits[i])
    {
      bit = 0;
      break;
    }
  }
  btorsim_bv_set_bit (res, 0, !bit);

  assert (rem_bits_zero_dbg (res));
  return res;
}

static uint32_t
find_diff_index (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  uint32_t i = 0;

  /* find index on which a and b differ */
  for (i = 0; i < a->len && a->bits[i] == b->bits[i]; i++)
    ;

  return i;
}

BtorSimBitVector *
btorsim_bv_ult (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i, bit;
  BtorSimBitVector *res;

  res = btorsim_bv_new (1);
  bit = 1;

  i = find_diff_index (a, b);

  /* a == b */
  if (i == a->len || a->bits[i] >= b->bits[i]) bit = 0;

  btorsim_bv_set_bit (res, 0, bit);

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_ulte (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i, bit;
  BtorSimBitVector *res;

  res = btorsim_bv_new (1);
  bit = 1;

  i = find_diff_index (a, b);

  /* a == b */
  if (i < a->len && a->bits[i] > b->bits[i]) bit = 0;

  btorsim_bv_set_bit (res, 0, bit);

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_slt (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i, bit, sign_a, sign_b;
  BtorSimBitVector *res;

  res = btorsim_bv_new (1);
  bit = 1;

  sign_a = btorsim_bv_get_bit (a, a->width - 1);
  sign_b = btorsim_bv_get_bit (b, b->width - 1);

  if (sign_a == sign_b)
  {
    i = find_diff_index (a, b);
    if (i == a->len || a->bits[i] >= b->bits[i]) bit = 0;
  }
  else
  {
    bit = sign_a && !sign_b;
  }

  btorsim_bv_set_bit (res, 0, bit);

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_slte (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i, bit, sign_a, sign_b;
  BtorSimBitVector *res;

  res = btorsim_bv_new (1);
  bit = 1;

  sign_a = btorsim_bv_get_bit (a, a->width - 1);
  sign_b = btorsim_bv_get_bit (b, b->width - 1);

  if (sign_a == sign_b)
  {
    i = find_diff_index (a, b);
    if (i < a->len && a->bits[i] > b->bits[i]) bit = 0;
  }
  else
  {
    bit = sign_a && !sign_b;
  }

  btorsim_bv_set_bit (res, 0, bit);

  assert (rem_bits_zero_dbg (res));
  return res;
}

static BtorSimBitVector *
sll_bv (const BtorSimBitVector *a, uint32_t shift)
{
  assert (a);

  uint32_t skip, i, j, k;
  BtorSimBitVector *res;
  BTORSIM_BV_TYPE v;

  res = btorsim_bv_new (a->width);
  if (shift >= a->width) return res;

  k    = shift % BTORSIM_BV_TYPE_BW;
  skip = shift / BTORSIM_BV_TYPE_BW;

  v = 0;
  for (i = a->len - 1, j = res->len - 1 - skip;; i--, j--)
  {
    v            = (k == 0) ? a->bits[i] : v | (a->bits[i] << k);
    res->bits[j] = v;
    v = (k == 0) ? a->bits[i] : a->bits[i] >> (BTORSIM_BV_TYPE_BW - k);
    if (i == 0 || j == 0) break;
  }
  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  assert (check_bits_sll_dbg (a, res, shift));
  return res;
}

BtorSimBitVector *
btorsim_bv_sll (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint64_t shift;
  BtorSimBitVector *res;
  shift = btorsim_bv_to_uint64 (b);
  res   = sll_bv (a, shift);
  return res;
}

BtorSimBitVector *
btorsim_bv_srl (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t skip, i, j, k;
  uint64_t shift;
  BtorSimBitVector *res;
  BTORSIM_BV_TYPE v;

  res   = btorsim_bv_new (a->width);
  shift = btorsim_bv_to_uint64 (b);
  if (shift >= a->width) return res;

  k    = shift % BTORSIM_BV_TYPE_BW;
  skip = shift / BTORSIM_BV_TYPE_BW;

  v = 0;
  for (i = 0, j = skip; i < a->len && j < a->len; i++, j++)
  {
    v            = (k == 0) ? a->bits[i] : v | (a->bits[i] >> k);
    res->bits[j] = v;
    v = (k == 0) ? a->bits[i] : a->bits[i] << (BTORSIM_BV_TYPE_BW - k);
  }

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_sra (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  BtorSimBitVector *res, *sign_b, *srl1, *srl2, *not_a;

  sign_b = btorsim_bv_slice (b, b->width - 1, b->width - 1);
  srl1   = btorsim_bv_srl (a, b);
  not_a  = btorsim_bv_not (a);
  srl2   = btorsim_bv_srl (not_a, b);
  res    = btorsim_bv_is_true (not_a) ? btorsim_bv_not (srl2)
                                   : btorsim_bv_copy (srl1);
  btorsim_bv_free (sign_b);
  btorsim_bv_free (srl1);
  btorsim_bv_free (srl2);
  btorsim_bv_free (not_a);
  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_mul (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  uint32_t i;
  uint64_t x, y;
  BtorSimBitVector *res, *and, *shift, *add;

  if (a->width <= 64)
  {
    x   = btorsim_bv_to_uint64 (a);
    y   = btorsim_bv_to_uint64 (b);
    res = btorsim_bv_uint64_to_bv (x * y, a->width);
  }
  else
  {
    res = btorsim_bv_new (a->width);
    for (i = 0; i < a->width; i++)
    {
      if (btorsim_bv_get_bit (b, i))
        and = btorsim_bv_copy (a);
      else
        and = btorsim_bv_new (a->width);
      shift = sll_bv (and, i);
      add   = btorsim_bv_add (res, shift);
      free (and);
      free (shift);
      free (res);
      res = add;
    }
  }
  assert (rem_bits_zero_dbg (res));
  return res;
}

static void
udiv_urem_bv (const BtorSimBitVector *a,
              const BtorSimBitVector *b,
              BtorSimBitVector **q,
              BtorSimBitVector **r)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  int64_t i;
  bool is_true;
  uint64_t x, y, z;

  BtorSimBitVector *neg_b, *quot, *rem, *ult, *eq, *tmp;

  if (a->width <= 64)
  {
    x = btorsim_bv_to_uint64 (a);
    y = btorsim_bv_to_uint64 (b);
    if (y == 0)
    {
      y = x;
      x = UINT64_MAX;
    }
    else
    {
      z = x / y;
      y = x % y;
      x = z;
    }
    quot = btorsim_bv_uint64_to_bv (x, a->width);
    rem  = btorsim_bv_uint64_to_bv (y, a->width);
  }
  else
  {
    neg_b = btorsim_bv_neg (b);
    quot  = btorsim_bv_new (a->width);
    rem   = btorsim_bv_new (a->width);

    for (i = a->width - 1; i >= 0; i--)
    {
      tmp = sll_bv (rem, 1);
      free (rem);
      rem = tmp;
      btorsim_bv_set_bit (rem, 0, btorsim_bv_get_bit (a, i));

      ult     = btorsim_bv_ult (b, rem);
      is_true = btorsim_bv_is_true (ult);
      free (ult);

      if (is_true) goto UDIV_UREM_SUBTRACT;

      eq      = btorsim_bv_eq (b, rem);
      is_true = btorsim_bv_is_true (eq);
      free (eq);

      if (is_true)
      {
      UDIV_UREM_SUBTRACT:
        tmp = btorsim_bv_add (rem, neg_b);
        free (rem);
        rem = tmp;
        btorsim_bv_set_bit (quot, i, 1);
      }
    }
    free (neg_b);
  }

  if (q)
    *q = quot;
  else
    free (quot);

  if (r)
    *r = rem;
  else
    free (rem);
}

BtorSimBitVector *
btorsim_bv_udiv (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  BtorSimBitVector *res = 0;
  udiv_urem_bv (a, b, &res, 0);
  assert (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_sdiv (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  BtorSimBitVector *res = 0, *not_a, *not_a_and_b, *sign_a, *sign_b, *xor;
  BtorSimBitVector *neg_a, *neg_b, *cond_a, *cond_b, *udiv, *neg_udiv;

  if (a->width == 1)
  {
    not_a       = btorsim_bv_not (a);
    not_a_and_b = btorsim_bv_and (not_a, b);
    res         = btorsim_bv_not (not_a_and_b);
    btorsim_bv_free (not_a);
    btorsim_bv_free (not_a_and_b);
  }
  else
  {
    sign_a = btorsim_bv_slice (a, a->width - 1, a->width - 1);
    sign_b = btorsim_bv_slice (b, b->width - 1, b->width - 1);
    xor    = btorsim_bv_xor (sign_a, sign_b);
    neg_a  = btorsim_bv_neg (a);
    neg_b  = btorsim_bv_neg (b);
    cond_a = btorsim_bv_is_true (sign_a) ? btorsim_bv_copy (neg_a)
                                         : btorsim_bv_copy (a);
    cond_b = btorsim_bv_is_true (sign_b) ? btorsim_bv_copy (neg_b)
                                         : btorsim_bv_copy (b);
    udiv     = btorsim_bv_udiv (cond_a, cond_b);
    neg_udiv = btorsim_bv_neg (udiv);
    res      = btorsim_bv_is_true (xor) ? btorsim_bv_copy (neg_udiv)
                                   : btorsim_bv_copy (udiv);
    btorsim_bv_free (sign_a);
    btorsim_bv_free (sign_b);
    btorsim_bv_free (xor);
    btorsim_bv_free (neg_a);
    btorsim_bv_free (neg_b);
    btorsim_bv_free (cond_a);
    btorsim_bv_free (cond_b);
    btorsim_bv_free (udiv);
    btorsim_bv_free (neg_udiv);
  }

  assert (res);
  assert (rem_bits_zero_dbg (res));

  return res;
}

BtorSimBitVector *
btorsim_bv_urem (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  BtorSimBitVector *res = 0;
  udiv_urem_bv (a, b, 0, &res);
  assert (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_srem (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  BtorSimBitVector *res = 0, *not_b, *sign_a, *sign_b, *neg_a, *neg_b;
  BtorSimBitVector *cond_a, *cond_b, *urem, *neg_urem;

  if (a->width == 1)
  {
    not_b = btorsim_bv_not (b);
    res   = btorsim_bv_and (a, not_b);
    btorsim_bv_free (not_b);
  }
  else
  {
    sign_a = btorsim_bv_slice (a, a->width - 1, a->width - 1);
    sign_b = btorsim_bv_slice (b, b->width - 1, b->width - 1);
    neg_a  = btorsim_bv_neg (a);
    neg_b  = btorsim_bv_neg (b);
    /* normalize a and b if necessary */
    cond_a = btorsim_bv_is_true (sign_a) ? btorsim_bv_copy (neg_a)
                                         : btorsim_bv_copy (a);
    cond_b = btorsim_bv_is_true (sign_b) ? btorsim_bv_copy (neg_b)
                                         : btorsim_bv_copy (b);
    urem     = btorsim_bv_urem (cond_a, cond_b);
    neg_urem = btorsim_bv_neg (urem);
    res      = btorsim_bv_is_true (sign_a) ? btorsim_bv_copy (neg_urem)
                                      : btorsim_bv_copy (urem);
    btorsim_bv_free (sign_a);
    btorsim_bv_free (sign_b);
    btorsim_bv_free (neg_a);
    btorsim_bv_free (neg_b);
    btorsim_bv_free (cond_a);
    btorsim_bv_free (cond_b);
    btorsim_bv_free (urem);
    btorsim_bv_free (neg_urem);
  }

  assert (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_concat (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);

  int64_t i, j, k;
  BTORSIM_BV_TYPE v;
  BtorSimBitVector *res;

  res = btorsim_bv_new (a->width + b->width);

  j = res->len - 1;

  /* copy bits from bit vector b */
  for (i = b->len - 1; i >= 0; i--) res->bits[j--] = b->bits[i];

  k = b->width % BTORSIM_BV_TYPE_BW;

  /* copy bits from bit vector a */
  if (k == 0)
  {
    assert (j >= 0);
    for (i = a->len - 1; i >= 0; i--) res->bits[j--] = a->bits[i];
  }
  else
  {
    j += 1;
    assert (res->bits[j] >> k == 0);
    v = res->bits[j];
    for (i = a->len - 1; i >= 0; i--)
    {
      v = v | (a->bits[i] << k);
      assert (j >= 0);
      res->bits[j--] = v;
      v              = a->bits[i] >> (BTORSIM_BV_TYPE_BW - k);
    }
    assert (j <= 0);
    if (j == 0) res->bits[j] = v;
  }

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_slice (const BtorSimBitVector *bv, uint32_t upper, uint32_t lower)
{
  assert (bv);

  uint32_t i, j;
  BtorSimBitVector *res;

  res = btorsim_bv_new (upper - lower + 1);
  for (i = lower, j = 0; i <= upper; i++)
    btorsim_bv_set_bit (res, j++, btorsim_bv_get_bit (bv, i));

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_sext (const BtorSimBitVector *bv, uint32_t len)
{
  assert (bv);
  assert (len > 0);

  uint32_t msb;
  BtorSimBitVector *res, *tmp;

  msb = btorsim_bv_get_bit (bv, bv->width - 1);
  tmp = msb ? btorsim_bv_ones (len) : btorsim_bv_zero (len);
  res = btorsim_bv_concat (tmp, bv);
  btorsim_bv_free (tmp);
  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_uext (const BtorSimBitVector *bv, uint32_t len)
{
  assert (bv);
  assert (len > 0);

  BtorSimBitVector *res;

  res = btorsim_bv_new (bv->width + len);
  memcpy (
      res->bits + res->len - bv->len, bv->bits, sizeof (*(bv->bits)) * bv->len);

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_ite (const BtorSimBitVector *c,
                const BtorSimBitVector *t,
                const BtorSimBitVector *e)
{
  assert (c);
  assert (c->len == 1);
  assert (t);
  assert (t->len > 0);
  assert (e);
  assert (t->len == e->len);
  assert (t->width == e->width);

  BtorSimBitVector *res;
  BTORSIM_BV_TYPE cc, nn;
  uint32_t i;

  cc = btorsim_bv_get_bit (c, 0) ? (~(BTORSIM_BV_TYPE) 0) : 0;
  nn = ~cc;

  res = btorsim_bv_new (t->width);
  for (i = 0; i < t->len; i++)
    res->bits[i] = (cc & t->bits[i]) | (nn & e->bits[i]);

  assert (rem_bits_zero_dbg (res));
  return res;
}

BtorSimBitVector *
btorsim_bv_flipped_bit (const BtorSimBitVector *bv, uint32_t pos)
{
  assert (bv);
  assert (bv->len > 0);
  assert (pos < bv->width);

  BtorSimBitVector *res;

  res = btorsim_bv_copy (bv);
  btorsim_bv_set_bit (res, pos, btorsim_bv_get_bit (res, pos) ? 0 : 1);
  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));

  return res;
}

BtorSimBitVector *
btorsim_bv_flipped_bit_range (const BtorSimBitVector *bv,
                              uint32_t upper,
                              uint32_t lower)
{
  assert (lower <= upper);
  assert (upper < bv->width);

  uint32_t i;
  BtorSimBitVector *res;

  res = btorsim_bv_copy (bv);
  for (i = lower; i <= upper; i++)
    btorsim_bv_set_bit (res, i, btorsim_bv_get_bit (res, i) ? 0 : 1);
  set_rem_bits_to_zero (res);
  assert (rem_bits_zero_dbg (res));
  return res;
}

/*------------------------------------------------------------------------*/

bool
btorsim_bv_is_umulo (const BtorSimBitVector *a, const BtorSimBitVector *b)
{
  assert (a);
  assert (b);
  assert (a->len == b->len);
  assert (a->width == b->width);

  bool res;
  BtorSimBitVector *aext, *bext, *mul, *o;

  res = false;

  if (a->width > 1)
  {
    aext = btorsim_bv_uext (a, a->width);
    bext = btorsim_bv_uext (b, b->width);
    mul  = btorsim_bv_mul (aext, bext);
    o    = btorsim_bv_slice (mul, mul->width - 1, a->width);
    if (!btorsim_bv_is_zero (o)) res = true;
    free (aext);
    free (bext);
    free (mul);
    free (o);
  }

  return res;
}

/*------------------------------------------------------------------------*/

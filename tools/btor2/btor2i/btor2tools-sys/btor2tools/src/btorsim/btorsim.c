/**
 *  Btor2Tools: A tool package for the BTOR format.
 *
 *  Copyright (c) 2018 Armin Biere.
 *  Copyright (c) 2018 Aina Niemetz.
 *  Copyright (c) 2019 Mathias Preiner.
 *
 *  All rights reserved.
 *
 *  This file is part of the Btor2Tools package.
 *  See LICENSE.txt for more information on using this software.
 */

#include <assert.h>
#include <ctype.h>
#include <inttypes.h>
#include <limits.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "btor2parser/btor2parser.h"
#include "btorsimbv.h"
#include "btorsimrng.h"
#include "util/btor2mem.h"
#include "util/btor2stack.h"

/*------------------------------------------------------------------------*/

static void
die (char *m, ...)
{
  fflush (stdout);
  fputs ("*** 'btorsim' error: ", stderr);
  va_list ap;
  va_start (ap, m);
  vfprintf (stderr, m, ap);
  va_end (ap);
  fprintf (stderr, "\n");
  exit (1);
}

static int32_t verbosity;
static int32_t print_states;

static void
msg (int32_t level, char *m, ...)
{
  if (level > verbosity) return;
  assert (m);
  printf ("[btorsim] ");
  va_list ap;
  va_start (ap, m);
  vprintf (m, ap);
  va_end (ap);
  printf ("\n");
}

static const char *usage =
    "usage: btorsim [ <option> ... ] [ <btor> [ <witness> ] ]\n"
    "\n"
    "where <option> is one of the following\n"
    "\n"
    "  -h        print this command line option summary\n"
    "  -c        check only <witness> and do not print trace\n"
    "  -v        increase verbosity level (multiple times if necessary)\n"
    "  -r <n>    generate <n> random transitions (default 20)\n"
    "  -s <s>    random seed (default '0')\n"
    "\n"
    "  -b <n>    fake simulation to satisfy bad state property 'b<n>'\n"
    "  -j <n>    fake simulation to satisfy justice property 'j<n>'\n"
    "\n"
    "  --states  print all states\n"
    "\n"
    "and '<btor>' is sequential model in 'BTOR' format\n"
    "and '<witness>' a trace in 'BTOR' witness format.\n"
    "\n"
    "The simulator either checks a given witness (checking mode) or\n"
    "randomly generates inputs (random mode). If no BTOR model path is\n"
    "specified then it is read from '<stdin>'.  The simulator only uses\n"
    "checking mode if both the BTOR model and a witness file are specified.\n";

static const char *model_path;
static const char *witness_path;
static FILE *model_file;
static FILE *witness_file;
static int32_t close_model_file;
static int32_t close_witness_file;

static int32_t
parse_int (const char *str, int32_t *res_ptr)
{
  const char *p = str;
  if (!*p) return 0;
  if (*p == '0' && p[1]) return 0;
  int32_t res = 0;
  while (*p)
  {
    const int32_t ch = *p++;
    if (!isdigit (ch)) return 0;
    if (INT_MAX / 10 < res) return 0;
    res *= 10;
    const int32_t digit = ch - '0';
    if (INT_MAX - digit < res) return 0;
    res += digit;
  }
  *res_ptr = res;
  return 1;
}

static int32_t
parse_long (const char *str, int64_t *res_ptr)
{
  const char *p = str;
  if (!*p) return 0;
  if (*p == '0' && p[1]) return 0;
  int64_t res = 0;
  while (*p)
  {
    const int32_t ch = *p++;
    if (!isdigit (ch)) return 0;
    if (LONG_MAX / 10 < res) return 0;
    res *= 10;
    const int32_t digit = ch - '0';
    if (LONG_MAX - digit < res) return 0;
    res += digit;
  }
  *res_ptr = res;
  return 1;
}

static int32_t checking_mode = 0;
static int32_t random_mode   = 0;

static Btor2Parser *model;

BTOR2_DECLARE_STACK (Btor2LinePtr, Btor2Line *);

static Btor2LinePtrStack inputs;
static Btor2LinePtrStack states;
static Btor2LinePtrStack bads;
static Btor2LinePtrStack constraints;
static Btor2LinePtrStack justices;

BTOR2_DECLARE_STACK (BtorLong, int64_t);

static BtorLongStack reached_bads;

static int64_t constraints_violated = -1;
static int64_t num_unreached_bads;

static int64_t num_format_lines;
static Btor2Line **inits;
static Btor2Line **nexts;

static BtorSimBitVector **current_state;
static BtorSimBitVector **next_state;

static void
parse_model_line (Btor2Line *l)
{
  switch (l->tag)
  {
    case BTOR2_TAG_bad:
    {
      int64_t i = (int64_t) BTOR2_COUNT_STACK (bads);
      msg (2, "bad %" PRId64 " at line %" PRId64, i, l->lineno);
      BTOR2_PUSH_STACK (bads, l);
      BTOR2_PUSH_STACK (reached_bads, -1);
      num_unreached_bads++;
    }
    break;

    case BTOR2_TAG_constraint:
    {
      int64_t i = (int64_t) BTOR2_COUNT_STACK (constraints);
      msg (2, "constraint %" PRId64 " at line %" PRId64, i, l->lineno);
      BTOR2_PUSH_STACK (constraints, l);
    }
    break;

    case BTOR2_TAG_init: inits[l->args[0]] = l; break;

    case BTOR2_TAG_input:
    {
      int64_t i = (int64_t) BTOR2_COUNT_STACK (inputs);
      if (l->symbol)
        msg (2, "input %" PRId64 " '%s' at line %" PRId64, i, l->symbol, l->lineno);
      else
        msg (2, "input %" PRId64 " at line %" PRId64, i, l->lineno);
      BTOR2_PUSH_STACK (inputs, l);
    }
    break;

    case BTOR2_TAG_next: nexts[l->args[0]] = l; break;

    case BTOR2_TAG_sort:
    {
      switch (l->sort.tag)
      {
        case BTOR2_TAG_SORT_bitvec:
          msg (
              2, "sort bitvec %u at line %" PRId64, l->sort.bitvec.width, l->lineno);
          break;
        case BTOR2_TAG_SORT_array:
        default:
          die ("parse error in '%s' at line %" PRId64 ": unsupported sort '%s'",
               model_path,
               l->lineno,
               l->sort.name);
          break;
      }
    }
    break;

    case BTOR2_TAG_state:
    {
      int64_t i = (int64_t) BTOR2_COUNT_STACK (states);
      if (l->symbol)
        msg (2, "state %" PRId64 " '%s' at line %" PRId64, i, l->symbol, l->lineno);
      else
        msg (2, "state %" PRId64 " at line %" PRId64, i, l->lineno);
      BTOR2_PUSH_STACK (states, l);
    }
    break;

    case BTOR2_TAG_add:
    case BTOR2_TAG_and:
    case BTOR2_TAG_concat:
    case BTOR2_TAG_const:
    case BTOR2_TAG_constd:
    case BTOR2_TAG_consth:
    case BTOR2_TAG_dec:
    case BTOR2_TAG_eq:
    case BTOR2_TAG_implies:
    case BTOR2_TAG_inc:
    case BTOR2_TAG_ite:
    case BTOR2_TAG_mul:
    case BTOR2_TAG_nand:
    case BTOR2_TAG_neg:
    case BTOR2_TAG_neq:
    case BTOR2_TAG_nor:
    case BTOR2_TAG_not:
    case BTOR2_TAG_one:
    case BTOR2_TAG_ones:
    case BTOR2_TAG_or:
    case BTOR2_TAG_output:
    case BTOR2_TAG_redand:
    case BTOR2_TAG_redor:
    case BTOR2_TAG_sdiv:
    case BTOR2_TAG_sext:
    case BTOR2_TAG_sgt:
    case BTOR2_TAG_sgte:
    case BTOR2_TAG_slice:
    case BTOR2_TAG_sll:
    case BTOR2_TAG_slt:
    case BTOR2_TAG_slte:
    case BTOR2_TAG_sra:
    case BTOR2_TAG_srem:
    case BTOR2_TAG_srl:
    case BTOR2_TAG_sub:
    case BTOR2_TAG_udiv:
    case BTOR2_TAG_uext:
    case BTOR2_TAG_ugt:
    case BTOR2_TAG_ugte:
    case BTOR2_TAG_ult:
    case BTOR2_TAG_ulte:
    case BTOR2_TAG_urem:
    case BTOR2_TAG_xnor:
    case BTOR2_TAG_xor:
    case BTOR2_TAG_zero: break;

    case BTOR2_TAG_fair:
    case BTOR2_TAG_justice:
    case BTOR2_TAG_read:
    case BTOR2_TAG_redxor:
    case BTOR2_TAG_rol:
    case BTOR2_TAG_ror:
    case BTOR2_TAG_saddo:
    case BTOR2_TAG_sdivo:
    case BTOR2_TAG_smod:
    case BTOR2_TAG_smulo:
    case BTOR2_TAG_ssubo:
    case BTOR2_TAG_uaddo:
    case BTOR2_TAG_umulo:
    case BTOR2_TAG_usubo:
    case BTOR2_TAG_write:
    default:
      die ("parse error in '%s' at line %" PRId64 ": unsupported '%" PRId64 " %s%s'",
           model_path,
           l->lineno,
           l->id,
           l->name,
           l->nargs ? " ..." : "");
      break;
  }
}

static void
parse_model ()
{
  BTOR2_INIT_STACK (inputs);
  BTOR2_INIT_STACK (states);
  BTOR2_INIT_STACK (bads);
  BTOR2_INIT_STACK (justices);
  BTOR2_INIT_STACK (reached_bads);
  BTOR2_INIT_STACK (constraints);
  assert (model_file);
  model = btor2parser_new ();
  if (!btor2parser_read_lines (model, model_file))
    die ("parse error in '%s' at %s", model_path, btor2parser_error (model));
  num_format_lines = btor2parser_max_id (model);
  BTOR2_CNEWN (inits, num_format_lines);
  BTOR2_CNEWN (nexts, num_format_lines);
  Btor2LineIterator it = btor2parser_iter_init (model);
  Btor2Line *line;
  while ((line = btor2parser_iter_next (&it))) parse_model_line (line);

  for (int64_t i = 0; i < BTOR2_COUNT_STACK (states); i++)
  {
    Btor2Line *state = BTOR2_PEEK_STACK (states, i);
    if (!nexts[state->id])
    {
      msg (1, "state %d without next function", state->id);
    }
  }
}

static void
update_current_state (int64_t id, BtorSimBitVector *bv)
{
  assert (0 <= id), assert (id < num_format_lines);
  if (current_state[id]) btorsim_bv_free (current_state[id]);
  current_state[id] = bv;
}

static void
delete_current_state (int64_t id)
{
  assert (0 <= id), assert (id < num_format_lines);
  if (current_state[id]) btorsim_bv_free (current_state[id]);
  current_state[id] = 0;
}

static BtorSimBitVector *
simulate (int64_t id)
{
  int32_t sign = id < 0 ? -1 : 1;
  if (sign < 0) id = -id;
  assert (0 <= id), assert (id < num_format_lines);
  BtorSimBitVector *res = current_state[id];
  if (!res)
  {
    Btor2Line *l = btor2parser_get_line_by_id (model, id);
    if (!l) die ("internal error: unexpected empty ID %" PRId64, id);
    BtorSimBitVector *args[3] = {0, 0, 0};
    for (uint32_t i = 0; i < l->nargs; i++) args[i] = simulate (l->args[i]);
    switch (l->tag)
    {
      case BTOR2_TAG_add:
        assert (l->nargs == 2);
        res = btorsim_bv_add (args[0], args[1]);
        break;
      case BTOR2_TAG_and:
        assert (l->nargs == 2);
        res = btorsim_bv_and (args[0], args[1]);
        break;
      case BTOR2_TAG_concat:
        assert (l->nargs == 2);
        res = btorsim_bv_concat (args[0], args[1]);
        break;
      case BTOR2_TAG_const:
        assert (l->nargs == 0);
        res = btorsim_bv_char_to_bv (l->constant);
        break;
      case BTOR2_TAG_constd:
        assert (l->nargs == 0);
        res = btorsim_bv_constd (l->constant, l->sort.bitvec.width);
        break;
      case BTOR2_TAG_consth:
        assert (l->nargs == 0);
        res = btorsim_bv_consth (l->constant, l->sort.bitvec.width);
        break;
      case BTOR2_TAG_dec:
        assert (l->nargs == 1);
        res = btorsim_bv_dec (args[0]);
        break;
      case BTOR2_TAG_eq:
        assert (l->nargs == 2);
        res = btorsim_bv_eq (args[0], args[1]);
        break;
      case BTOR2_TAG_implies:
        assert (l->nargs == 2);
        res = btorsim_bv_implies (args[0], args[1]);
        break;
      case BTOR2_TAG_inc:
        assert (l->nargs == 1);
        res = btorsim_bv_inc (args[0]);
        break;
      case BTOR2_TAG_ite:
        assert (l->nargs == 3);
        res = btorsim_bv_ite (args[0], args[1], args[2]);
        break;
      case BTOR2_TAG_mul:
        assert (l->nargs == 2);
        res = btorsim_bv_mul (args[0], args[1]);
        break;
      case BTOR2_TAG_nand:
        assert (l->nargs == 2);
        res = btorsim_bv_nand (args[0], args[1]);
        break;
      case BTOR2_TAG_neg:
        assert (l->nargs == 1);
        res = btorsim_bv_neg (args[0]);
        break;
      case BTOR2_TAG_neq:
        assert (l->nargs == 2);
        res = btorsim_bv_neq (args[0], args[1]);
        break;
      case BTOR2_TAG_nor:
        assert (l->nargs == 2);
        res = btorsim_bv_nor (args[0], args[1]);
        break;
      case BTOR2_TAG_not:
        assert (l->nargs == 1);
        res = btorsim_bv_not (args[0]);
        break;
      case BTOR2_TAG_one: res = btorsim_bv_one (l->sort.bitvec.width); break;
      case BTOR2_TAG_ones: res = btorsim_bv_ones (l->sort.bitvec.width); break;
      case BTOR2_TAG_or:
        assert (l->nargs == 2);
        res = btorsim_bv_or (args[0], args[1]);
        break;
      case BTOR2_TAG_redand:
        assert (l->nargs == 1);
        res = btorsim_bv_redand (args[0]);
        break;
      case BTOR2_TAG_redor:
        assert (l->nargs == 1);
        res = btorsim_bv_redor (args[0]);
        break;
      case BTOR2_TAG_slice:
        assert (l->nargs == 1);
        res = btorsim_bv_slice (args[0], l->args[1], l->args[2]);
        break;
      case BTOR2_TAG_sub:
        assert (l->nargs == 2);
        res = btorsim_bv_sub (args[0], args[1]);
        break;
      case BTOR2_TAG_uext:
        assert (l->nargs == 1);
        {
          uint32_t width = args[0]->width;
          assert (width <= l->sort.bitvec.width);
          uint32_t padding = l->sort.bitvec.width - width;
          if (padding)
            res = btorsim_bv_uext (args[0], padding);
          else
            res = btorsim_bv_copy (args[0]);
        }
        break;
      case BTOR2_TAG_udiv:
        assert (l->nargs == 2);
        res = btorsim_bv_udiv (args[0], args[1]);
        break;
      case BTOR2_TAG_sdiv:
        assert (l->nargs == 2);
        res = btorsim_bv_sdiv (args[0], args[1]);
        break;
      case BTOR2_TAG_sext:
        assert (l->nargs == 1);
        {
          uint32_t width = args[0]->width;
          assert (width <= l->sort.bitvec.width);
          uint32_t padding = l->sort.bitvec.width - width;
          if (padding)
            res = btorsim_bv_sext (args[0], padding);
          else
            res = btorsim_bv_copy (args[0]);
        }
        break;
      case BTOR2_TAG_sll:
        assert (l->nargs == 2);
        res = btorsim_bv_sll (args[0], args[1]);
        break;
      case BTOR2_TAG_srl:
        assert (l->nargs == 2);
        res = btorsim_bv_srl (args[0], args[1]);
        break;
      case BTOR2_TAG_sra:
        assert (l->nargs == 2);
        res = btorsim_bv_sra (args[0], args[1]);
        break;
      case BTOR2_TAG_srem:
        assert (l->nargs == 2);
        res = btorsim_bv_srem (args[0], args[1]);
        break;
      case BTOR2_TAG_ugt:
        assert (l->nargs == 2);
        res = btorsim_bv_ult (args[1], args[0]);
        break;
      case BTOR2_TAG_ugte:
        assert (l->nargs == 2);
        res = btorsim_bv_ulte (args[1], args[0]);
        break;
      case BTOR2_TAG_ult:
        assert (l->nargs == 2);
        res = btorsim_bv_ult (args[0], args[1]);
        break;
      case BTOR2_TAG_ulte:
        assert (l->nargs == 2);
        res = btorsim_bv_ulte (args[0], args[1]);
        break;
      case BTOR2_TAG_urem:
        assert (l->nargs == 2);
        res = btorsim_bv_urem (args[0], args[1]);
        break;
      case BTOR2_TAG_sgt:
        assert (l->nargs == 2);
        res = btorsim_bv_slt (args[1], args[0]);
        break;
      case BTOR2_TAG_sgte:
        assert (l->nargs == 2);
        res = btorsim_bv_slte (args[1], args[0]);
        break;
      case BTOR2_TAG_slt:
        assert (l->nargs == 2);
        res = btorsim_bv_slt (args[0], args[1]);
        break;
      case BTOR2_TAG_slte:
        assert (l->nargs == 2);
        res = btorsim_bv_slte (args[0], args[1]);
        break;
      case BTOR2_TAG_iff:
      case BTOR2_TAG_xnor:
        assert (l->nargs == 2);
        res = btorsim_bv_xnor (args[0], args[1]);
        break;
      case BTOR2_TAG_xor:
        assert (l->nargs == 2);
        res = btorsim_bv_xor (args[0], args[1]);
        break;
      case BTOR2_TAG_zero: res = btorsim_bv_zero (l->sort.bitvec.width); break;
      default:
        die ("can not randomly simulate operator '%s' at line %" PRId64,
             l->name,
             l->lineno);
        break;
    }
    for (uint32_t i = 0; i < l->nargs; i++) btorsim_bv_free (args[i]);
    update_current_state (id, res);
  }
  res = btorsim_bv_copy (res);
  if (sign < 0)
  {
    BtorSimBitVector *tmp = btorsim_bv_not (res);
    btorsim_bv_free (res);
    res = tmp;
  }
  return res;
}

static int32_t print_trace = 1;
static BtorSimRNG rng;

static void
initialize_inputs (int64_t k, int32_t randomize)
{
  msg (1, "initializing inputs @%" PRId64, k);
  if (print_trace) printf ("@%" PRId64 "\n", k);
  for (int64_t i = 0; i < BTOR2_COUNT_STACK (inputs); i++)
  {
    Btor2Line *input = BTOR2_PEEK_STACK (inputs, i);
    uint32_t width   = input->sort.bitvec.width;
    if (current_state[input->id]) continue;
    BtorSimBitVector *update;
    if (randomize)
      update = btorsim_bv_new_random (&rng, width);
    else
      update = btorsim_bv_new (width);
    update_current_state (input->id, update);
    if (print_trace)
    {
      printf ("%" PRId64 " ", i);
      btorsim_bv_print_without_new_line (update);
      if (input->symbol) printf (" %s@%" PRId64, input->symbol, k);
      fputc ('\n', stdout);
    }
  }
}

static void
initialize_states (int32_t randomly)
{
  msg (1, "initializing states at #0");
  if (print_trace) printf ("#0\n");
  for (int64_t i = 0; i < BTOR2_COUNT_STACK (states); i++)
  {
    Btor2Line *state = BTOR2_PEEK_STACK (states, i);
    assert (0 <= state->id), assert (state->id < num_format_lines);
    if (current_state[state->id]) continue;
    Btor2Line *init = inits[state->id];
    BtorSimBitVector *update;
    if (init)
    {
      assert (init->nargs == 2);
      assert (init->args[0] == state->id);
      update = simulate (init->args[1]);
    }
    else
    {
      assert (state->sort.tag == BTOR2_TAG_SORT_bitvec);
      uint32_t width = state->sort.bitvec.width;
      if (randomly)
        update = btorsim_bv_new_random (&rng, width);
      else
        update = btorsim_bv_new (width);
    }
    update_current_state (state->id, update);
    if (print_trace && !init)
    {
      printf ("%" PRId64 " ", i);
      btorsim_bv_print_without_new_line (update);
      if (state->symbol) printf (" %s#0", state->symbol);
      fputc ('\n', stdout);
    }
  }
}

static void
simulate_step (int64_t k, int32_t randomize_states_that_are_inputs)
{
  msg (1, "simulating step %" PRId64, k);
  for (int64_t i = 0; i < num_format_lines; i++)
  {
    Btor2Line *l = btor2parser_get_line_by_id (model, i);
    if (!l) continue;
    if (l->tag == BTOR2_TAG_sort || l->tag == BTOR2_TAG_init
        || l->tag == BTOR2_TAG_next || l->tag == BTOR2_TAG_bad
        || l->tag == BTOR2_TAG_constraint || l->tag == BTOR2_TAG_fair
        || l->tag == BTOR2_TAG_justice || l->tag == BTOR2_TAG_output)
      continue;

    BtorSimBitVector *bv = simulate (i);
#if 0
    printf ("[btorim] %" PRId64 " %s ", l->id, l->name);
    btorsim_bv_print (bv);
    fflush (stdout);
#endif
    btorsim_bv_free (bv);
  }
  for (int64_t i = 0; i < BTOR2_COUNT_STACK (states); i++)
  {
    Btor2Line *state = BTOR2_PEEK_STACK (states, i);
    assert (0 <= state->id), assert (state->id < num_format_lines);
    Btor2Line *next = nexts[state->id];
    BtorSimBitVector *update;
    if (next)
    {
      assert (next->nargs == 2);
      assert (next->args[0] == state->id);
      update = simulate (next->args[1]);
    }
    else
    {
      assert (state->sort.tag == BTOR2_TAG_SORT_bitvec);
      uint32_t width = state->sort.bitvec.width;
      if (randomize_states_that_are_inputs)
        update = btorsim_bv_new_random (&rng, width);
      else
        update = btorsim_bv_new (width);
    }
    assert (!next_state[state->id]);
    next_state[state->id] = update;
  }

  if (constraints_violated < 0)
  {
    for (int64_t i = 0; i < BTOR2_COUNT_STACK (constraints); i++)
    {
      Btor2Line *constraint = BTOR2_PEEK_STACK (constraints, i);
      BtorSimBitVector *bv  = current_state[constraint->args[0]];
      if (!btorsim_bv_is_zero (bv)) continue;
      msg (1,
           "constraint(%" PRId64 ") '%" PRId64 " constraint %" PRId64 "' violated at time %" PRId64,
           i,
           constraint->id,
           constraint->args[0],
           k);
      constraints_violated = k;
    }
  }

  if (constraints_violated < 0)
  {
    for (int64_t i = 0; i < BTOR2_COUNT_STACK (bads); i++)
    {
      int64_t r = BTOR2_PEEK_STACK (reached_bads, i);
      if (r >= 0) continue;
      Btor2Line *bad       = BTOR2_PEEK_STACK (bads, i);
      BtorSimBitVector *bv = current_state[bad->args[0]];
      if (btorsim_bv_is_zero (bv)) continue;
      int64_t bound = BTOR2_PEEK_STACK (reached_bads, i);
      if (bound >= 0) continue;
      BTOR2_POKE_STACK (reached_bads, i, k);
      assert (num_unreached_bads > 0);
      if (!--num_unreached_bads)
        msg (1,
             "all %" PRId64 " bad state properties reached",
             (int64_t) BTOR2_COUNT_STACK (bads));
    }
  }
}

static void
transition (int64_t k)
{
  msg (1, "transition %" PRId64, k);
  for (int64_t i = 0; i < num_format_lines; i++) delete_current_state (i);
  if (print_trace && print_states) printf ("#%" PRId64 "\n", k);
  for (int64_t i = 0; i < BTOR2_COUNT_STACK (states); i++)
  {
    Btor2Line *state = BTOR2_PEEK_STACK (states, i);
    assert (0 <= state->id), assert (state->id < num_format_lines);
    BtorSimBitVector *update = next_state[state->id];
    assert (update);
    update_current_state (state->id, update);
    next_state[state->id] = 0;
    if (print_trace && print_states)
    {
      printf ("%" PRId64 " ", i);
      btorsim_bv_print_without_new_line (update);
      if (state->symbol) printf (" %s#%" PRId64, state->symbol, k);
      fputc ('\n', stdout);
    }
  }
}

static void
report ()
{
  if (verbosity && num_unreached_bads < BTOR2_COUNT_STACK (bads))
  {
    printf ("[btorsim] reached bad state properties {");
    for (int64_t i = 0; i < BTOR2_COUNT_STACK (bads); i++)
    {
      int64_t r = BTOR2_PEEK_STACK (reached_bads, i);
      if (r >= 0) printf (" b%" PRId64 "@%" PRId64, i, r);
    }
    printf (" }\n");
  }
  else if (!BTOR2_EMPTY_STACK (bads))
    msg (1, "no bad state property reached");

  if (constraints_violated >= 0)
    msg (1, "constraints violated at time %" PRId64, constraints_violated);
  else if (!BTOR2_EMPTY_STACK (constraints))
    msg (1, "constraints always satisfied");
}

static void
random_simulation (int64_t k)
{
  msg (1, "starting random simulation up to bound %" PRId64, k);
  assert (k >= 0);

  const int32_t randomize = 1;

  initialize_states (randomize);
  initialize_inputs (0, randomize);
  simulate_step (0, randomize);

  for (int64_t i = 1; i <= k; i++)
  {
    if (constraints_violated >= 0) break;
    if (!num_unreached_bads) break;
    transition (i);
    initialize_inputs (i, randomize);
    simulate_step (i, randomize);
  }

  if (print_trace) printf (".\n"), fflush (stdout);
  report ();
}

static int64_t charno;
static int64_t columno;
static int64_t lineno = 1;
static int32_t saved_char;
static int32_t char_saved;
static uint64_t last_line_length;

static BtorCharStack constant;
static BtorCharStack symbol;

static int32_t
next_char ()
{
  int32_t res;
  if (char_saved)
  {
    res        = saved_char;
    char_saved = 0;
  }
  else
  {
    res = getc (witness_file);
  }
  if (res == '\n')
  {
    last_line_length = columno;
    columno          = 0;
    lineno++;
  }
  else if (res != EOF)
  {
    columno++;
  }
  if (res != EOF) charno++;
  return res;
}

static void
prev_char (int32_t ch)
{
  assert (!char_saved);
  if (ch == '\n')
  {
    columno = last_line_length;
    assert (lineno > 0);
    lineno--;
  }
  else if (ch != EOF)
  {
    assert (charno > 0);
    charno--;
    assert (columno > 0);
    columno--;
  }
  saved_char = ch;
  char_saved = 1;
}

static void
parse_error (const char *msg, ...)
{
  fflush (stdout);
  assert (witness_path);
  fprintf (stderr,
           "*** 'btorsim' parse error in '%s' at line %" PRId64 " column %" PRId64 ": ",
           witness_path,
           lineno,
           columno);
  va_list ap;
  va_start (ap, msg);
  vfprintf (stderr, msg, ap);
  va_end (ap);
  fprintf (stderr, "\n");
  exit (1);
}

static int64_t count_sat_witnesses;
static int64_t count_unsat_witnesses;
static int64_t count_unknown_witnesses;
static int64_t count_witnesses;

static BtorLongStack claimed_bad_witnesses;
static BtorLongStack claimed_justice_witnesses;

static int64_t
parse_unsigned_number (int32_t *ch_ptr)
{
  int32_t ch  = next_char ();
  int64_t res = 0;
  if (ch == '0')
  {
    ch = next_char ();
    if (isdigit (ch)) parse_error ("unexpected digit '%c' after '0'", ch);
  }
  else if (!isdigit (ch))
    parse_error ("expected digit");
  else
  {
    res = ch - '0';
    while (isdigit (ch = next_char ()))
    {
      if (LONG_MAX / 10 < res)
        parse_error ("number too large (too many digits)");
      res *= 10;
      const int32_t digit = ch - '0';
      if (LONG_MAX - digit < res) parse_error ("number too large");
      res += digit;
    }
  }
  *ch_ptr = ch;
  return res;
}

static int64_t constant_columno;
static int32_t found_end_of_witness;
static int32_t found_initial_frame;

static int64_t
parse_assignment ()
{
  int32_t ch = next_char ();
  if (ch == EOF) parse_error ("unexpected end-of-file (without '.')");
  if (ch == '.')
  {
    while ((ch = next_char ()) == ' ')
      ;
    if (ch == EOF) parse_error ("end-of-file after '.' instead of new-line");
    if (ch != '\n')
    {
      if (isprint (ch))
        parse_error ("unexpected character '%c' after '.'", ch);
      else
        parse_error ("unexpected character code 0x%02x after '.'", ch);
    }
    msg (4, "read terminating '.'");
    found_end_of_witness = 1;
    return -1;
  }
  if (ch == '@' || ch == '#')
  {
    prev_char (ch);
    return -1;
  }
  prev_char (ch);
  int64_t res = parse_unsigned_number (&ch);
  if (ch != ' ') parse_error ("space missing after '%" PRId64 "'", res);
  BTOR2_RESET_STACK (constant);
  constant_columno = columno + 1;
  while ((ch = next_char ()) == '0' || ch == '1')
    BTOR2_PUSH_STACK (constant, ch);
  if (ch == '[') parse_error ("can not handle array assignments yet");
  if (BTOR2_EMPTY_STACK (constant)) parse_error ("empty constant");
  if (BTOR2_EMPTY_STACK (constant))
    if (ch != ' ' && ch != '\n')
      parse_error ("expected space or new-line after assignment");
  BTOR2_PUSH_STACK (constant, 0);
  BTOR2_RESET_STACK (symbol);
  while (ch != '\n')
    if ((ch = next_char ()) == EOF)
      parse_error ("unexpected end-of-file in assignment");
    else if (ch != '\n')
      BTOR2_PUSH_STACK (symbol, ch);
  if (!BTOR2_EMPTY_STACK (symbol)) BTOR2_PUSH_STACK (symbol, 0);
  return res;
}

static void
parse_state_part (int64_t k)
{
  int32_t ch = next_char ();
  if (k == 0)
  {
    if (ch != '#' || parse_unsigned_number (&ch) != k || ch != '\n')
      parse_error (
          "missing '#%" PRId64 "' state part header of frame %" PRId64, k, k);
  }
  else
  {
    if (ch != '#')
    {
      prev_char (ch);
      return;
    }
    if (parse_unsigned_number (&ch) != k || ch != '\n')
      parse_error (
          "missing '#%" PRId64 "' state part header of frame %" PRId64, k, k);
  }
  int64_t state_pos;
  while ((state_pos = parse_assignment ()) >= 0)
  {
    int64_t saved_charno = charno;
    charno            = 1;
    assert (lineno > 1);
    lineno--;
    if (state_pos >= BTOR2_COUNT_STACK (states))
      parse_error ("less than %" PRId64 " states defined", state_pos);
    if (BTOR2_EMPTY_STACK (symbol))
      msg (4,
           "state assignment '%" PRId64 " %s' at time frame %" PRId64,
           state_pos,
           constant.start,
           k);
    else
      msg (4,
           "state assignment '%" PRId64 " %s %s' at time frame %" PRId64,
           state_pos,
           constant.start,
           symbol.start,
           k);
    Btor2Line *state = BTOR2_PEEK_STACK (states, state_pos);
    assert (state);
    if (strlen (constant.start) != state->sort.bitvec.width)
      charno = constant_columno,
      parse_error ("expected constant of width '%u'", state->sort.bitvec.width);
    assert (0 <= state->id), assert (state->id < num_format_lines);
    if (current_state[state->id] && nexts[state->id])
      parse_error ("state %" PRId64 " id %" PRId64 " assigned twice in frame %" PRId64,
                   state_pos,
                   state->id,
                   k);
    BtorSimBitVector *val = btorsim_bv_char_to_bv (constant.start);
    Btor2Line *init       = inits[state->id];
    if (init && nexts[state->id])
    {
      assert (init->nargs == 2);
      assert (init->args[0] == state->id);
      BtorSimBitVector *tmp = simulate (init->args[1]);
      if (btorsim_bv_compare (val, tmp))
        parse_error (
            "incompatible initialized state %" PRId64 " id %" PRId64, state_pos, state->id);
      btorsim_bv_free (tmp);
    }
    lineno++;
    charno = saved_charno;
    if (k > 0 && nexts[state->id]
        && btorsim_bv_compare (val, current_state[state->id]))
    {
      parse_error ("incompatible assignment for state %" PRId64 " id %" PRId64
                   " in time frame %" PRId64,
                   state_pos,
                   state->id,
                   k);
    }
    update_current_state (state->id, val);
  }
  if (!k) found_initial_frame = 1;
}

static void
parse_input_part (int64_t k)
{
  int32_t ch = next_char ();
  if (ch != '@' || parse_unsigned_number (&ch) != k || ch != '\n')
    parse_assignment ();
  int64_t input_pos;
  while ((input_pos = parse_assignment ()) >= 0)
  {
    int64_t saved_charno = charno;
    charno            = 1;
    assert (lineno > 1);
    lineno--;
    if (input_pos >= BTOR2_COUNT_STACK (inputs))
      parse_error ("less than %" PRId64 " defined", input_pos);
    if (BTOR2_EMPTY_STACK (symbol))
      msg (4,
           "input assignment '%" PRId64 " %s' at time frame %" PRId64,
           input_pos,
           constant.start,
           k);
    else
      msg (4,
           "input assignment '%" PRId64 " %s %s' at time frame %" PRId64,
           input_pos,
           constant.start,
           symbol.start,
           k);
    Btor2Line *input = BTOR2_PEEK_STACK (inputs, input_pos);
    assert (input);
    if (strlen (constant.start) != input->sort.bitvec.width)
      charno = constant_columno,
      parse_error ("expected constant of width '%u'", input->sort.bitvec.width);
    assert (0 <= input->id), assert (input->id < num_format_lines);
    if (current_state[input->id])
      parse_error ("input %" PRId64 " id %" PRId64 " assigned twice in frame %" PRId64,
                   input_pos,
                   input->id,
                   k);
    BtorSimBitVector *val = btorsim_bv_char_to_bv (constant.start);
    lineno++;
    charno = saved_charno;
    update_current_state (input->id, val);
  }
}

static int32_t
parse_frame (int64_t k)
{
  if (k > 0) transition (k);
  msg (2, "parsing frame %" PRId64, k);
  parse_state_part (k);
  parse_input_part (k);
  const int32_t randomize = 0;
  if (!k) initialize_states (randomize);
  initialize_inputs (k, randomize);
  simulate_step (k, randomize);
  return !found_end_of_witness;
}

static void
parse_sat_witness ()
{
  assert (count_witnesses == 1);

  msg (1, "parsing 'sat' witness %" PRId64, count_sat_witnesses);

  BTOR2_INIT_STACK (claimed_bad_witnesses);
  BTOR2_INIT_STACK (claimed_justice_witnesses);

  for (;;)
  {
    int32_t type = next_char ();
    if (type == ' ') continue;
    if (type == '\n') break;
    ;
    if (type != 'b' && type != 'j') parse_error ("expected 'b' or 'j'");
    int32_t ch;
    int64_t bad = parse_unsigned_number (&ch);
    if (ch != ' ' && ch != '\n')
    {
      if (isprint (ch))
        parse_error (
            "unexpected '%c' after number (expected space or new-line)", ch);
      else
        parse_error (
            "unexpected character 0x%02x after number"
            " (expected space or new-line)",
            ch);
    }
    if (type == 'b')
    {
      if (bad >= BTOR2_COUNT_STACK (bads))
        parse_error ("invalid bad state property number %" PRId64, bad);
      msg (3,
           "... claims to be witness of bad state property number 'b%" PRId64 "'",
           bad);
      BTOR2_PUSH_STACK (claimed_bad_witnesses, bad);
    }
    else
      parse_error ("can not handle justice properties yet");
    if (ch == '\n') break;
  }

  int64_t k = 0;
  while (parse_frame (k)) k++;

  if (!found_initial_frame) parse_error ("initial frame missing");
  msg (1, "finished parsing k = %" PRId64 " frames", k);

  report ();
  if (print_trace) printf (".\n"), fflush (stdout);

  for (int64_t i = 0; i < BTOR2_COUNT_STACK (claimed_bad_witnesses); i++)
  {
    int64_t bad_pos = BTOR2_PEEK_STACK (claimed_bad_witnesses, i);
    int64_t bound   = BTOR2_PEEK_STACK (reached_bads, bad_pos);
    Btor2Line *l = BTOR2_PEEK_STACK (bads, bad_pos);
    if (bound < 0)
      die ("claimed bad state property 'b%" PRId64 "' id %" PRId64 " not reached",
           bad_pos,
           l->id);
  }

  BTOR2_RELEASE_STACK (claimed_bad_witnesses);
  BTOR2_RELEASE_STACK (claimed_justice_witnesses);
}

static void
parse_unknown_witness ()
{
  msg (1, "parsing unknown witness %" PRId64, count_unknown_witnesses);
  int64_t k = 0;

  while (parse_frame (k)) k++;

  if (!found_initial_frame) parse_error ("initial frame missing");

  report ();
  if (print_trace) printf (".\n"), fflush (stdout);

  msg (1, "finished parsing k = %" PRId64 " frames", k);
}

static void
parse_unsat_witness ()
{
  msg (1, "parsing 'unsat' witness %" PRId64, count_unsat_witnesses);
  die ("'unsat' witnesses not supported yet");
}

static int32_t
parse_and_check_witness ()
{
  int32_t ch = next_char ();
  if (ch == EOF) return 0;

  found_end_of_witness = 0;
  found_initial_frame  = 0;

  if (ch == '#')
  {
    count_witnesses++;
    count_unknown_witnesses++;
    if (count_sat_witnesses + count_unknown_witnesses > 1)
      die ("more than one actual witness not supported yet");
    prev_char (ch);
    parse_unknown_witness ();
    return 1;
  }

  if (ch == 's')
  {
    if ((ch = next_char ()) == 'a' && (ch = next_char ()) == 't'
        && (ch = next_char ()) == '\n')
    {
      count_witnesses++;
      count_sat_witnesses++;
      msg (1,
           "found witness %" PRId64 " header 'sat' in '%s' at line %" PRId64,
           count_sat_witnesses,
           witness_path,
           lineno - 1);
      if (count_witnesses > 1)
        die ("more than one actual witness not supported yet");
      parse_sat_witness ();
      return 1;
    }
  }

  if (ch == 'u')
  {
    if ((ch = next_char ()) == 'n' && (ch = next_char ()) == 's'
        && (ch = next_char ()) == 'a' && (ch = next_char ()) == 't'
        && (ch = next_char ()) == '\n')
    {
      count_witnesses++;
      count_unsat_witnesses++;
      msg (1,
           "found witness %" PRId64 " header 'unsat' in '%s' at line %" PRId64,
           witness_path,
           count_unsat_witnesses,
           lineno - 1);
      parse_unsat_witness ();
      return 1;
    }
  }

  while (ch != '\n')
  {
    ch = next_char ();
    if (ch == EOF) parse_error ("unexpected end-of-file before new-line");
  }

  return 1;
}

static void
parse_and_check_all_witnesses ()
{
  BTOR2_INIT_STACK (constant);
  BTOR2_INIT_STACK (symbol);
  assert (witness_file);
  while (parse_and_check_witness ())
    ;
  BTOR2_RELEASE_STACK (constant);
  BTOR2_RELEASE_STACK (symbol);
  msg (1,
       "finished parsing %" PRId64 " witnesses after reading %" PRId64 " bytes (%.1f MB)",
       count_witnesses,
       charno,
       charno / (double) (1l << 20));
}

int32_t
main (int32_t argc, char **argv)
{
  int64_t fake_bad = -1, fake_justice = -1;
  int32_t r = -1, s = -1;
  for (int32_t i = 1; i < argc; i++)
  {
    if (!strcmp (argv[i], "-h"))
      fputs (usage, stdout), exit (0);
    else if (!strcmp (argv[i], "-c"))
      print_trace = 0;
    else if (!strcmp (argv[i], "-v"))
      verbosity++;
    else if (!strcmp (argv[i], "-r"))
    {
      if (++i == argc) die ("argument to '-r' missing");
      if (!parse_int (argv[i], &r)) die ("invalid number in '-r %s'", argv[i]);
    }
    else if (!strcmp (argv[i], "-s"))
    {
      if (++i == argc) die ("argument to '-s' missing");
      if (!parse_int (argv[i], &s)) die ("invalid number in '-s %s'", argv[i]);
    }
    else if (!strcmp (argv[i], "-b"))
    {
      if (++i == argc) die ("argument to '-b' missing");
      if (!parse_long (argv[i], &fake_bad))
        die ("invalid number in '-b %s'", argv[i]);
    }
    else if (!strcmp (argv[i], "-j"))
    {
      if (++i == argc) die ("argument to '-j' missing");
      if (!parse_long (argv[i], &fake_justice))
        die ("invalid number in '-j %s'", argv[i]);
    }
    else if (!strcmp (argv[i], "--states"))
      print_states = 1;
    else if (argv[i][0] == '-')
      die ("invalid command line option '%s' (try '-h')", argv[i]);
    else if (witness_path)
      die ("too many file arguments '%s', '%s', and '%s'",
           model_path,
           witness_path,
           argv[i]);
    else if (model_path)
      witness_path = argv[i];
    else
      model_path = argv[i];
  }
  if (model_path)
  {
    if (!(model_file = fopen (model_path, "r")))
      die ("failed to open BTOR model file '%s' for reading", model_path);
    close_model_file = 1;
  }
  else
  {
    model_path = "<stdin>";
    model_file = stdin;
  }
  if (witness_path)
  {
    if (!(witness_file = fopen (witness_path, "r")))
      die ("failed to open witness file '%s' for reading", witness_path);
    close_witness_file = 1;
  }
  if (model_path && witness_path)
  {
    msg (1, "checking mode: both model and witness specified");
    checking_mode = 1;
    random_mode   = 0;
  }
  else
  {
    msg (1, "random mode: witness not specified");
    checking_mode = 0;
    random_mode   = 1;
  }
  if (checking_mode)
  {
    if (r >= 0)
      die ("number of random test vectors specified in checking mode");
    if (s >= 0) die ("random seed specified in checking mode");
    if (fake_bad >= 0) die ("can not fake bad state property in checking mode");
    if (fake_justice >= 0)
      die ("can not fake justice property in checking mode");
  }
  assert (model_path);
  msg (1, "reading BTOR model from '%s'", model_path);
  parse_model ();
  if (fake_bad >= BTOR2_COUNT_STACK (bads))
    die ("invalid faked bad state property number %" PRId64, fake_bad);
  if (fake_justice >= BTOR2_COUNT_STACK (justices))
    die ("invalid faked justice property number %" PRId64, fake_justice);
  if (close_model_file && fclose (model_file))
    die ("can not close model file '%s'", model_path);
  BTOR2_CNEWN (current_state, num_format_lines);
  BTOR2_CNEWN (next_state, num_format_lines);
  if (random_mode)
  {
    if (r < 0) r = 20;
    if (s < 0) s = 0;
    msg (1, "using random seed %d", s);
    btorsim_rng_init (&rng, (uint32_t) s);
    if (print_trace)
    {
      if (fake_bad >= 0 && fake_justice >= 0)
        printf ("sat\nb%" PRId64 " j%" PRId64 "\n", fake_bad, fake_justice);
      else if (fake_bad >= 0)
        printf ("sat\nb%" PRId64 "\n", fake_bad);
      else if (fake_justice >= 0)
        printf ("sat\nj%" PRId64 "\n", fake_justice);
    }
    random_simulation (r);
  }
  else
  {
    assert (witness_path);
    msg (1, "reading BTOR witness from '%s'", witness_path);
    parse_and_check_all_witnesses ();
    if (close_witness_file && fclose (witness_file))
      die ("can not close witness file '%s'", witness_path);
  }
  BTOR2_RELEASE_STACK (inputs);
  BTOR2_RELEASE_STACK (states);
  BTOR2_RELEASE_STACK (bads);
  BTOR2_RELEASE_STACK (justices);
  BTOR2_RELEASE_STACK (reached_bads);
  BTOR2_RELEASE_STACK (constraints);
  btor2parser_delete (model);
  BTOR2_DELETE (inits);
  BTOR2_DELETE (nexts);
  for (int64_t i = 0; i < num_format_lines; i++)
    if (current_state[i]) btorsim_bv_free (current_state[i]);
  for (int64_t i = 0; i < num_format_lines; i++)
    if (next_state[i]) btorsim_bv_free (next_state[i]);
  BTOR2_DELETE (current_state);
  BTOR2_DELETE (next_state);
  return 0;
}

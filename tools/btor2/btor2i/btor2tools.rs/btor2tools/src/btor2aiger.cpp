/**
 *  Btor2Tools: A tool package for the BTOR format.
 *
 *  Copyright (c) 2019 Mathias Preiner.
 *
 *  All rights reserved.
 *
 *  This file is part of the Btor2Tools package.
 *  See LICENSE.txt for more information on using this software.
 */

#include <cassert>
#include <cstdarg>
#include <cstring>
#include <iostream>
#include <unordered_map>
#include <unordered_set>
#include <vector>

#include "boolector/boolector.h"
#include "btor2parser/btor2parser.h"
extern "C" {
#include "aiger.h"
}

/*--------------------------------------------------------------------------*/

static void
print_usage ()
{
  std::cout << "Usage:" << std::endl;
  std::cout << "  btor2aiger [options] BTOR2_FILE\n" << std::endl;
  std::cout << "Options:" << std::endl;
  std::cout << "  -h,--help   Print this help and exit." << std::endl;
  std::cout << "  -a          Print in AIGER ascii format." << std::endl;
  std::cout << "  -i          Ignore AIGER errors." << std::endl;
  std::cout << std::endl;
}

static void
die (const char *fmt, ...)
{
  va_list ap;
  va_start (ap, fmt);
  fprintf (stderr, "error: ");
  vfprintf (stderr, fmt, ap);
  va_end (ap);
  fprintf (stderr, "\n");
  exit (EXIT_FAILURE);
}

/*--------------------------------------------------------------------------*/

class Btor2Model
{
 public:
  Btor *btor;

  std::vector<BoolectorNode *> inputs;
  std::unordered_map<int64_t, BoolectorNode *> states;
  std::unordered_map<int64_t, BoolectorNode *> init;
  std::unordered_map<int64_t, BoolectorNode *> next;
  std::vector<BoolectorNode *> bad;
  std::vector<BoolectorNode *> constraints;

  std::unordered_map<int64_t, BoolectorNode *> nodes;
  std::unordered_map<int64_t, BoolectorSort> sorts;

  Btor2Model () : btor (boolector_new ()){};

  ~Btor2Model ()
  {
    for (auto kv : nodes)
    {
      boolector_release (btor, kv.second);
    }
    for (auto kv : sorts)
    {
      boolector_release_sort (btor, kv.second);
    }
    boolector_delete (btor);
  }

  BoolectorSort get_sort (int64_t id)
  {
    auto it = sorts.find (id);
    assert (it != sorts.end ());
    return it->second;
  }

  void add_sort (int64_t id, BoolectorSort sort)
  {
    assert (sorts.find(id)  == sorts.end ());
    sorts[id] = sort;
  }

  BoolectorNode *get_node (int64_t id)
  {
    auto it = nodes.find (id);
    if (it == nodes.end ())
    {
      it = nodes.find (-id);
      assert (it != nodes.end ());
      add_node (id, boolector_not (btor, it->second));
      return nodes[id];
    }
    return it->second;
  }

  void add_node (int64_t id, BoolectorNode *node)
  {
    assert (nodes.find(id) == nodes.end ());
    nodes[id] = node;
  }

  BoolectorNode *get_init (int64_t id)
  {
    auto it = init.find (id);
    return it != init.end () ? it->second : nullptr;
  }

  BoolectorNode *get_next (int64_t id)
  {
    auto it = next.find (id);
    return it != next.end () ? it->second : nullptr;
  }
};

using BtorUnaryFun   = BoolectorNode *(*) (Btor *, BoolectorNode *);
using BtorBinaryFun  = BoolectorNode *(*) (Btor *,
                                          BoolectorNode *,
                                          BoolectorNode *);
using BtorTernaryFun = BoolectorNode *(*) (Btor *,
                                           BoolectorNode *,
                                           BoolectorNode *,
                                           BoolectorNode *);

static std::unordered_map<Btor2Tag, BtorUnaryFun> s_tag2unfun ({
    {BTOR2_TAG_dec, boolector_dec},
    {BTOR2_TAG_inc, boolector_inc},
    {BTOR2_TAG_neg, boolector_neg},
    {BTOR2_TAG_not, boolector_not},
    {BTOR2_TAG_redand, boolector_redand},
    {BTOR2_TAG_redor, boolector_redor},
    {BTOR2_TAG_redxor, boolector_redxor},
});

static std::unordered_map<Btor2Tag, BtorBinaryFun> s_tag2binfun ({
    {BTOR2_TAG_add, boolector_add},
    {BTOR2_TAG_and, boolector_and},
    {BTOR2_TAG_concat, boolector_concat},
    {BTOR2_TAG_eq, boolector_eq},
    {BTOR2_TAG_iff, boolector_iff},
    {BTOR2_TAG_implies, boolector_implies},
    {BTOR2_TAG_mul, boolector_mul},
    {BTOR2_TAG_nand, boolector_nand},
    {BTOR2_TAG_neq, boolector_ne},
    {BTOR2_TAG_nor, boolector_nor},
    {BTOR2_TAG_or, boolector_or},
    //  {BTOR2_TAG_read, boolector_read},
    {BTOR2_TAG_rol, boolector_rol},
    {BTOR2_TAG_ror, boolector_ror},
    {BTOR2_TAG_saddo, boolector_saddo},
    {BTOR2_TAG_sdiv, boolector_sdiv},
    {BTOR2_TAG_sdivo, boolector_sdivo},
    {BTOR2_TAG_sgt, boolector_sgt},
    {BTOR2_TAG_sgte, boolector_sgte},
    {BTOR2_TAG_sll, boolector_sll},
    {BTOR2_TAG_slt, boolector_slt},
    {BTOR2_TAG_slte, boolector_slte},
    {BTOR2_TAG_smod, boolector_smod},
    {BTOR2_TAG_smulo, boolector_smulo},
    {BTOR2_TAG_sra, boolector_sra},
    {BTOR2_TAG_srem, boolector_srem},
    {BTOR2_TAG_srl, boolector_srl},
    {BTOR2_TAG_ssubo, boolector_ssubo},
    {BTOR2_TAG_sub, boolector_sub},
    {BTOR2_TAG_uaddo, boolector_uaddo},
    {BTOR2_TAG_udiv, boolector_udiv},
    {BTOR2_TAG_ugt, boolector_ugt},
    {BTOR2_TAG_ugte, boolector_ugte},
    {BTOR2_TAG_ult, boolector_ult},
    {BTOR2_TAG_ulte, boolector_ulte},
    {BTOR2_TAG_umulo, boolector_umulo},
    {BTOR2_TAG_urem, boolector_urem},
    {BTOR2_TAG_usubo, boolector_usubo},
    {BTOR2_TAG_xnor, boolector_xnor},
    {BTOR2_TAG_xor, boolector_xor},
});

static std::unordered_map<Btor2Tag, BtorTernaryFun> s_tag2terfun ({
    {BTOR2_TAG_ite, boolector_cond},
    //  {BTOR2_TAG_write, boolector_write},
});

static void
parse_btor2 (FILE *infile, Btor2Model &model)
{
  Btor2Parser *parser;
  Btor2LineIterator it;
  Btor2Line *l;
  BoolectorSort sort;
  BoolectorNode *node, *args[3];

  Btor *btor = model.btor;

  parser = btor2parser_new ();
  if (!btor2parser_read_lines (parser, infile))
  {
    die (btor2parser_error (parser));
  }

  it = btor2parser_iter_init (parser);
  while ((l = btor2parser_iter_next (&it)))
  {
    for (uint32_t i = 0; i < l->nargs; ++i)
    {
      args[i] = model.get_node (l->args[i]);
    }

    switch (l->tag)
    {
      case BTOR2_TAG_bad: model.bad.push_back (args[0]); break;

      case BTOR2_TAG_const:
        model.add_node (l->id, boolector_const (btor, l->constant));
        break;

      case BTOR2_TAG_constd:
        sort = model.get_sort (l->sort.id);
        model.add_node (l->id, boolector_constd (btor, sort, l->constant));
        break;

      case BTOR2_TAG_consth:
        sort = model.get_sort (l->sort.id);
        model.add_node (l->id, boolector_consth (btor, sort, l->constant));
        break;

      case BTOR2_TAG_constraint: model.constraints.push_back (args[0]); break;

      case BTOR2_TAG_init:
        assert (!model.get_init (l->args[0]));
        model.init[l->args[0]] = args[1];
        break;

      case BTOR2_TAG_input:
      case BTOR2_TAG_state:
        sort = model.get_sort (l->sort.id);
        node = boolector_var (btor, sort, l->symbol);
        model.add_node (l->id, node);

        if (l->tag == BTOR2_TAG_input)
          model.inputs.push_back (node);
        else
          model.states[l->id] = node;
        break;

      case BTOR2_TAG_next:
        assert (!model.get_next (l->args[0]));
        model.next[l->args[0]] = args[1];
        break;

      case BTOR2_TAG_slice:
        model.add_node (
            l->id, boolector_slice (btor, args[0], l->args[1], l->args[2]));
        break;

      case BTOR2_TAG_one:
        sort = model.get_sort (l->sort.id);
        model.add_node (l->id, boolector_one (btor, sort));
        break;

      case BTOR2_TAG_ones:
        sort = model.get_sort (l->sort.id);
        model.add_node (l->id, boolector_ones (btor, sort));
        break;

      case BTOR2_TAG_zero:
        sort = model.get_sort (l->sort.id);
        model.add_node (l->id, boolector_zero (btor, sort));
        break;

      case BTOR2_TAG_sort:
        if (l->sort.tag == BTOR2_TAG_SORT_bitvec)
        {
          assert (l->sort.bitvec.width);
          model.add_sort (l->id,
                          boolector_bitvec_sort (btor, l->sort.bitvec.width));
        }
        else
        {
          die ("arrays not supported yet");
        }
        break;

      case BTOR2_TAG_uext:
        model.add_node (l->id, boolector_uext (btor, args[0], l->args[1]));
        break;

      case BTOR2_TAG_sext:
        model.add_node (l->id, boolector_sext (btor, args[0], l->args[1]));
        break;

      case BTOR2_TAG_fair:
      case BTOR2_TAG_justice: die ("unsupported tag: %s", l->name); break;

      /* BTOR2 outputs can be ignored. */
      case BTOR2_TAG_output: break;

      default:
        node = nullptr;
        if (l->nargs == 1)
        {
          auto it = s_tag2unfun.find (l->tag);
          assert (it != s_tag2unfun.end ());
          node  = it->second (btor, args[0]);
        }
        else if (l->nargs == 2)
        {
          auto it = s_tag2binfun.find (l->tag);
          assert (it != s_tag2binfun.end ());
          node   = it->second (btor, args[0], args[1]);
        }
        else if (l->nargs == 3)
        {
          auto it = s_tag2terfun.find (l->tag);
          assert (it != s_tag2terfun.end ());
          node   = it->second (btor, args[0], args[1], args[2]);
        }
        else
        {
          die ("unsupported tag: %s", l->name);
        }
        assert (node);
        model.add_node (l->id, node);
    }
  }
  btor2parser_delete (parser);
}

struct AIGVisitorState
{
  aiger *aig;
  std::unordered_set<uint64_t> cache;

  AIGVisitorState (aiger *aig) : aig (aig) {}
};

static void
aig_visitor (void *state,
             bool is_post,
             uint64_t node_id,
             const char *symbol,
             uint64_t child0_id,
             uint64_t child1_id)
{
  (void) symbol;
  if (!is_post || !child0_id) return;
  AIGVisitorState *vstate = static_cast<AIGVisitorState *> (state);
  if (vstate->cache.find (node_id) != vstate->cache.end ()) return;
  aiger_add_and (vstate->aig, node_id, child0_id, child1_id);
  vstate->cache.insert (node_id);
}

static void
add_input_to_aiger (Btor *btor,
                    BoolectorAIGMgr *amgr,
                    aiger *aig,
                    BoolectorNode *input)
{
  size_t nbits;
  uint64_t *bits;

  nbits = boolector_get_width (btor, input);
  bits  = boolector_aig_get_bits (amgr, input);

  for (size_t i = 0; i < nbits; ++i)
  {
    aiger_add_input (aig, bits[i], boolector_aig_get_symbol (amgr, bits[i]));
  }
  boolector_aig_free_bits (amgr, bits, nbits);
}

static void
add_state_to_aiger (Btor *btor,
                    BoolectorAIGMgr *amgr,
                    aiger *aig,
                    BoolectorNode *state,
                    BoolectorNode *next,
                    BoolectorNode *init)
{
  size_t nbits;
  const char *sym;
  uint64_t *state_bits, *next_bits, *init_bits, reset_val;

  state_bits = next_bits = init_bits = nullptr;

  nbits = boolector_get_width (btor, state);
  assert (!init || nbits == boolector_get_width (btor, init));
  assert (!next || nbits == boolector_get_width (btor, next));

  if (init && !next)
  {
    /* Note: BTOR2 allows states without next function to be initialized,
     * which are essentially inputs with an initial value in the first time
     * frame. In AIGER we would need to add more logic to have the same
     * behavior, which we omit for now and skip the benchmark. */
    die ("Found initialized state without next function");
  }

  state_bits = boolector_aig_get_bits (amgr, state);
  if (next) next_bits = boolector_aig_get_bits (amgr, next);
  if (init) init_bits = boolector_aig_get_bits (amgr, init);

  for (size_t i = 0; i < nbits; ++i)
  {
    if (init_bits && init_bits[i] != 0 && init_bits[i] != 1)
    {
      /* Note: BTOR2 supports arbitrary initialization functions, but AIGER
       * only supports 0/1/undefined. */
      die ("Found non-constant initialization");
    }
    sym = boolector_aig_get_symbol (amgr, state_bits[i]);
    if (next_bits)
    {
      reset_val = init_bits ? init_bits[i] : state_bits[i];
      aiger_add_latch (aig, state_bits[i], next_bits[i], sym);
      aiger_add_reset (aig, state_bits[i], reset_val);
    }
    else
    {
      /* Note: BTOR2 handles states without next function as input. Thus,
       * we have to create an input in AIGER. */
      aiger_add_input (aig, state_bits[i], sym);
    }
  }

  boolector_aig_free_bits (amgr, state_bits, nbits);
  if (next) boolector_aig_free_bits (amgr, next_bits, nbits);
  if (init) boolector_aig_free_bits (amgr, init_bits, nbits);
}

static void
add_constraint_to_aiger (Btor *btor,
                         BoolectorAIGMgr *amgr,
                         aiger *aig,
                         BoolectorNode *constraint)
{
  size_t nbits;
  uint64_t *bits;

  nbits = boolector_get_width (btor, constraint);
  assert (nbits == 1);
  bits = boolector_aig_get_bits (amgr, constraint);
  aiger_add_constraint (aig, bits[0], boolector_aig_get_symbol (amgr, bits[0]));
  boolector_aig_free_bits (amgr, bits, nbits);
}

static void
add_bad_to_aiger (Btor *btor,
                  BoolectorAIGMgr *amgr,
                  aiger *aig,
                  BoolectorNode *constraint)
{
  size_t nbits;
  uint64_t *bits;

  nbits = boolector_get_width (btor, constraint);
  assert (nbits == 1);
  bits = boolector_aig_get_bits (amgr, constraint);
  aiger_add_bad (aig, bits[0], boolector_aig_get_symbol (amgr, bits[0]));
  boolector_aig_free_bits (amgr, bits, nbits);
}

static void
generate_aiger (Btor2Model &model, bool ascii_mode, bool ignore_error)
{
  BoolectorAIGMgr *amgr;
  aiger *aig;

  amgr = boolector_aig_new (model.btor);

  aig = aiger_init ();
  AIGVisitorState aig_visitor_state (aig);

  for (BoolectorNode *n : model.inputs)
  {
    boolector_aig_bitblast (amgr, n);
    add_input_to_aiger (model.btor, amgr, aig, n);
  }

  for (auto kv : model.states)
  {
    boolector_aig_bitblast (amgr, kv.second);
  }

  for (auto kv : model.init)
  {
    boolector_aig_bitblast (amgr, kv.second);
  }

  for (auto kv : model.next)
  {
    boolector_aig_bitblast (amgr, kv.second);
    boolector_aig_visit (amgr, kv.second, aig_visitor, &aig_visitor_state);
  }

  for (auto kv : model.states)
  {
    add_state_to_aiger (model.btor,
                        amgr,
                        aig,
                        kv.second,
                        model.get_next (kv.first),
                        model.get_init (kv.first));
  }

  for (BoolectorNode *n : model.constraints)
  {
    boolector_aig_bitblast (amgr, n);
    boolector_aig_visit (amgr, n, aig_visitor, &aig_visitor_state);
    add_constraint_to_aiger (model.btor, amgr, aig, n);
  }

  for (BoolectorNode *n : model.bad)
  {
    boolector_aig_bitblast (amgr, n);
    boolector_aig_visit (amgr, n, aig_visitor, &aig_visitor_state);
    add_bad_to_aiger (model.btor, amgr, aig, n);
  }

  const char *err = aiger_check (aig);
  if (err && !ignore_error)
  {
    die (err);
  }

  aiger_write_to_file (
      aig, ascii_mode ? aiger_ascii_mode : aiger_binary_mode, stdout);

  aiger_reset (aig);

  boolector_aig_delete (amgr);
}

int
main (int argc, char *argv[])
{
  FILE *infile     = 0;
  char *infilename = 0;
  bool ascii_mode  = false;
  bool ignore_error = false;

  for (int i = 1; i < argc; ++i)
  {
    if (!strcmp (argv[i], "-a"))
    {
      ascii_mode = true;
    }
    else if (!strcmp (argv[i], "-i"))
    {
      ignore_error = true;
    }
    else if (!strcmp (argv[i], "-h") || !strcmp (argv[i], "--help"))
    {
      print_usage ();
      return EXIT_SUCCESS;
    }
    else
    {
      if (infilename)
      {
        die ("Multiple input files specified.");
      }
      infilename = argv[i];
    }
  }

  if (!infilename)
  {
    die ("No BTOR2 input file specified.");
  }

  infile = fopen (infilename, "r");
  if (!infile)
  {
    die ("Cannot open BTOR2 input file.");
  }
  Btor2Model model;
  parse_btor2 (infile, model);
  fclose (infile);
  generate_aiger (model, ascii_mode, ignore_error);

  return EXIT_SUCCESS;
}

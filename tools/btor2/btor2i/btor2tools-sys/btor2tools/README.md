Btor2Tools
===============================================================================

The Btor2Tools package provides a generic parser and tools for the BTOR2 format.

For a more detailed description of the BTOR2 format, refer to  
*BTOR2, BtorMC and Boolector 3.0.* Aina Niemetz, Mathias Preiner, Clifford Wolf,
and Armin Biere. CAV 2018.

Download
-------------------------------------------------------------------------------

  The latest version of Btor2Tools can be found on GitHub:
  https://github.com/boolector/btor2tools

Build
-------------------------------------------------------------------------------

From the Btor2Tools root directory configure and build as follows:
```
./configure.sh
cd build
make
```
For more build configuration options of Btor2Tools, see `configure.sh -h`.

All binaries (btorsim, catbtor) are generated into directory `build/bin`,
and all libraries (libbtor2parser.a, libbtor2parser.so) are generated into
directory `build/lib`.


Usage
-------------------------------------------------------------------------------

### BTOR2 Parser

Btor2Parser is a generic parser for the BTOR2 format.

```
Btor2Parser* parser;
Btor2LineIterator it;
Btor2Line* line;

parser = btor2parser_new ();
if (!btor2parser_read_lines (reader, input_file))
{
  // parse error
  const char *err = btor2parser_error (parser);
  // error handling
}
// iterate over parsed lines
it = btor2parser_iter_init (parser);
while ((line = btor2parser_iter_next (&it)))
{
  // process line
}
btor2parser_delete (parser);

```

For a simple example on how to use the BTOR2 parser, refer to `src/catbtor.c`.  
For a more comprehensive example, refer to function `parse_model()` in
`src/btorsim/btorsim.c`.


### BtorSim

BtorSim is a witness simulator and checker for BTOR2 witnesses.

For a list of command line options, refer to `btorsim -h`.  
For examples and instructions on how to use BtorSim, refer to
`examples/btorsim`.

### Catbtor

Catbtor is a simple tool to parse and print BTOR2 files. It is mainly used for
debugging purposes.

For a list of command line options, refer to `catbtor -h`.

The BTOR2 Format
------------------------------------------------------------------------------- 
For a detailed description, please refer to
[BTOR2, BtorMC and Boolector 3.0](https://link.springer.com/chapter/10.1007/978-3-319-96145-3_32)
at [CAV 2018](http://cavconference.org/2018/).

### Input Format

```
<num>      ::=  positive unsigned integer (greater than zero)
<uint>     ::=  unsigned integer (including zero)
<string>   ::=  sequence of whitespace and printable characters without '\n'
<symbol>   ::=  sequence of printable characters without '\n'
<comment>  ::=  ';' <string>
<nid>      ::=  <num>
<sid>      ::=  <num>
<const>    ::=  'const' <sid> [0-1]+
<constd>   ::=  'constd' <sid> ['-']<uint>
<consth>   ::=  'consth' <sid> [0-9a-fA-F]+
<input>    ::=  ('input' | 'one' | 'ones' | 'zero') <sid>
              | <const>
              | <constd>
              | <consth>
<state>    ::=  'state' <sid>
<bitvec>   ::=  'bitvec' <num>
<array>    ::=  'array' <sid> <sid>
<node>     ::=  <sid> 'sort' (<array> | <bitvec>)
              | <nid> (<input> | <state>)
              | <nid> <opidx> <sid> <nid> <uint> [<uint>]
              | <nid> <op> <sid> <nid> [<nid> [<nid>]]
              | <nid> ('init' | 'next') <sid> <nid> <nid>
              | <nid> ('bad' | 'constraint' | 'fair' | 'output') <nid>
              | <nid> 'justice' <num> (<nid>)+
<line>     ::=  <comment>
              | <node> [<symbol>] [<comment>]
<btor>     ::=  (<line>'\n')+

```

Non-terminals `<opidx>` and `<op>` are indexed and non-indexed operaters
as defined below (`B_[n]` represents a bit-vector sort of size n, and
`A_[I -> E]` represents an array sort with index sort `I` and element sort `E`).

#### Indexed Operators

| Operator            | Description               | Signature                 |
| ------------------- | ------------------------- | ------------------------- |
| `[su]ext w`         | (un)signed extension      | `B_[n] -> B_[n+w]`        |
| `slice u l`         | extraction, `n > u >= l`  | `B_[n] -> B_[u-l+1]`      |

#### Unary Operators

| Operator                    | Description       | Signature                 |
| --------------------------- | ----------------- | ------------------------- |
| `not`                       | bit-wise          | `B_[n] -> B_[n]`          |
| `inc`, `dec`, `neg`         | arithmetic        | `B_[n] -> B_[n]`          |
| `redand`, `redor`, `redxor` | reduction         | `B_[n] -> B_[1]`          |

#### Binary Operators

| Operator                                          | Description           | Signature                  |
| ------------------------------------------------- | --------------------- | -------------------------- |
| `iff`, `implies`                                  | Boolean               | `B_[1] x B_[1] -> B_[1]`   |
| `eq`, `neq`                                       | (dis)equality         | `S x S -> B_[1]`           |
| `[su]gt`, `[su]gte`, `[su]lt`, `[su]lte`          | (un)signed inequality | `B_[n] x B_[n] -> B_[1]`   |
| `and`, `nand`, `nor`, `or`, `xnor`, `xor`         | bit-wise              | `B_[n] x B_[n] -> B_[n]`   |
| `rol`, `ror`, `sll`, `sra`, `srl`                 | rotate, shift         | `B_[n] x B_[n] -> B_[n]`   |
| `add`, `mul`, `[su]div`, `smod`, `[su]rem`, `sub` | arithmetic            | `B_[n] x B_[n] -> B_[n]`   |
| `[su]addo`, `sdivo`, `[su]mulo`, `[su]subo`       | overflow              | `B_[n] x B_[n] -> B_[1]`   |
| `concat`                                          | concatenation         | `B_[n] x B_[m] -> B_[n+m]` |
| `read`                                            | array read            | `A_[I -> E] x I -> E`      |

#### Ternary Operators

| Operator       | Description           | Signature                          |
| -------------- | --------------------- | ---------------------------------- |
| `ite`          | conditional           | `B_[1] x B_[n] x B_[n] -> B_[n]`   |
| `write`        | array write           | `A_[I -> E] x I x E -> A_[I -> E]` |


### Witness Format

```
<binary-string>     ::=  [0-1]+
<bv-assignment>     ::=  <binary-string>
<array-assignment>  ::=  '['<binary-string>']' <binary-string>
<assignement>       ::=  <uint> (<bv-assignment>
                       | <array-assignment>) [<symbol>]
<model>             ::=  (<comment>'\n'
                       | <assignment>'\n')+
<state part>        ::=  '#'<uint>'\n' <model>
<input part>        ::=  '@'<uint>'\n' <model>
<frame>             ::=  [<state part>] <input part>
<prop>              ::=  ('b' | 'j')<uint>
<header>            ::=  'sat\n' (<prop>)+ '\n'
<witness>           ::=  (<comment>'\n')+
                       | <header> (<frame>)+ '.'
```


- [Objectives](#objectives)
- [Placement](#placement)
- [Tables](#tables)
  - [File Table](#file-table)
  - [Position Table](#position-table)
  - [Memory Locations](#memory-locations)
  - [Variable Assignments](#variable-assignments)
  - [Position-State Map](#position-state-map)

Notes on the metadata/source info table construction.

# Objectives
The goal of the metadata format is to associate information from the source
program with the Calyx program it ultimately generates. The generated program
has attached tags called `@pos{...}` attributes applied during generation. The
metadata table provides information that when used in conjunction with these
position tags allows:
1. Mapping the current execution point of a Calyx program to lines of the source program
2. Mapping source variables to the Calyx state which realizes them

# Placement
The source info table may optionally be included at the end of a calyx file,
after all component definitions. The table is demarcated as follow:
```
sourceinfo #{
  // content goes here
}#
```

# Tables
The source info format supports the following sub-tables in this order:
- File Table (Mandatory)
- Position Table (Mandatory)
- Memory Locations
- Variable Assignments
- Position-State Map

The file and position table are mandatory. The remaining three tables are
optional but must be either included or omitted as a group, i.e., if any of them
is included all must be.

The File & Position tables are use for source line attribution.

Memory Locations, Variable Assignments, and the Position-State Map are used to
map source variable names into Calyx state queries

## File Table
The file table records all source files which may be referred to by positions
attached to the calyx program. Each file may appear only once and is assigned a
unique identifying non-negative number that is used to refer to it in other
tables. By convention, we start the file id number at zero and count upwards,
but that is not required. A sample table is:
```
    FILES // marks the start of the table
        0: test.yx
        1: test2.yx
        2: test3.yx
```
## Position Table
The position table associates the position annotations (`@pos{...}` attributes)
found in the Calyx program with the source file and lines that they correspond
to. An entry in the position table looks like:
```
0: 0 5:8 // position zero, corresponds to file 0 lines 5 through 8
```
If the span the position refers to is only one line, the second line number may
be omitted
```
1: 0 4 // position one, file 0, line 4
```

A sample table looks like
```
    POSITIONS. // marks the start of the table
      0: 0 5:8
      1: 0 4
      15: 1 12:15
```
## Memory Locations
The Memory Location table defines the set of state elements in the Calyx program
which may be used to instantiate source variables. In this context, the state
elements may either be a register, memory, or entry within a memory. Much like
the File Table each state element is assigned a unique number used to reference
it in other tables. Within the table state elements are referred to via their
fully qualified name.

An example of this table is:
```
    MEMORY_LOCATIONS        // marks the start of the table
        0: main.reg1        // location 0 is the register
        1: main.mem0        // location 1 is the entire memory region
        2: main.mem1 [1,4]  // location 2 is the [1,4] entry of memory main.mem1
```

## Variable Assignments
A variable assignment is a collection of mappings between source variable names
and the memory locations which realize them. As with the files, each such
collection is identified with a unique numeric index.

A sample collection is:
```
        0: {      // collection zero
            x: 0  // variable "x" is stored in location 0
            y: 1  // variable "y" is stored in location 1
            z: 2  // variable "z" is stored in location 2
        }
```
Each collection may assign each surface variable at most once.

The variable position table aggregates multiple such collections.
```
    VARIABLE_ASSIGNMENTS
        0: {       // collection zero
            x: 0
            y: 1
            z: 2
        }
        1: {      // collection one
            q: 0  // note collections are independent in their assignments
        }
```

## Position-State Map
The Position-State table is responsible for mapping from positions to the
variable collection containing all the in-scope variables for the given
position. At the moment, such collections are assumed not to nest and so should
contain a mapping for all active variables. This table is a simple mapping:
```
    POSITION_STATE_MAP  // marks the start of the table
        0: 0            // position zero uses collection zero
        2: 1            // position two uses collection one
```

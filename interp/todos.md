# TODOs

What it means to "interpret" a program
- Groups vs. components vs. control
How to deal with undefined behavior

## Group
- Issues with clock cycles and updating data correctly prevent multi-clock cycle groups from being interpreted correctly
- Lift environment out of group level
- Signature: (Group, component, initial environment) -> Environment
- For now, ignoring if 2 groups dependent on each other in parallel

## Control
- Use the group interpreter (without it necessarily being completely correct)
- Par and seq
- If and while sequences

## Component

# Stuff to do
1. Change the group interpreter interface (Environment "decoupling", signature)
2. Implement control interpreter skeleton
  - Initialization (of the environment and the interpreter) and "validation" (of both)
  - How to handle sequences (par, seq, if, while)
3. Stubbing interpreter code for the Component -> Control -> Group hierarchy
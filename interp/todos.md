# TODOs

What it means to "interpret" a program
- Groups vs. components vs. control
How to deal with undefined behavior

## Group
- Issues with clock cycles and updating data correctly prevent multi-clock cycle groups from being interpreted correctly
- Lift environment out of group level
- Signature: (Group, component, initial environment) -> Environment
- For now, ignoring if 2 groups dependent on each other in parallel
- We need to be able to call a component interpreter if we come across a component while interpreting a group

## Control
- Use the group interpreter (without it necessarily being completely correct)
- Par and seq
- If and while sequences

## Component

# Stuff to do
1. Remove update queue from environment, eventually figure out scheduling mechanism when interpreting a control
2. Go over/meet about how to handle interpreting a control

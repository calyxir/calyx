# Notes regarding Fuse-IR

## FIRRTL
- types on wires representing bit width
- partial connects? i.e connecting an x bit wire with a y bit wire
- latency of memories in num of cycles (also clocks)
- flip memories ???

## Time part

The control part is a way of expressing constraints. There are 3 kinds of constraints:
- time constraints: if x happens at time t, and (seq x y), then y happens as time t + 1
- equalities, if x = y, then x is equal to y. (i.e. anywhere that you see x, you can replace it with y)
- conditional equalities, if [x = y (when c1)], then x is equal to y only when c1 is equal to 1.
    [x = y (unless c1)], then x is equal to y only when c1 is equal to 0.

## New Syntax
Be able to define groups of submodules as control points (but connections are normal)
actions in the control section are of the form, activate submodule, conditionally activate submodule, loops

you can imagine the wires as pipes where values flow. an unactivated module means that all the values
flowing into a module are blocked by gates. activating the module corresponds to letting the values
flow through. 

each component has it's own notion of time. in this world, a component takes 1 step to complete, no matter
how many steps it takes for the component to internally complete.

time is entirely imposed by the control structure, at this stage it does not live in the structure.
there are 3 inferfaces for specifying the flow of time, loops, sequential combination, and parallel combination.
these are simply ways for imposing constraints specifying, a happens at the same time as b, or a happens before b.

## implementation things
I need to fix what happens when a port is over specified. i.e [out = a (when c)] [out = b].

implementing memories in the simulation so that side effects are possible.
implementing time in the simulation
how does the new control thing interact with the current way of simulating? do I need to change it to a feed forward thing?
improving the visualization

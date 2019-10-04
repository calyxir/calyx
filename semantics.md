# Semantics for Futil
This document details the semantics for the Futil intermediate hardware description language.

## Basic Constructs
The basic building block in Futil is the `component`. Each component has the following properties:
* `inputs`: A set of input ports
* `outputs`: A set of output ports
* `structure`: Instances of subcomponents and wires between them
* `control`: High-level logical control-flow statements that determine the behavior of the subcomponents

## Simulation Semantics
In Futil, the semantics describe how a Futil program wlil be "simulated." Each simulation has a number of simulation steps, during which the structure and control of a component will be used to determine the values on each wire. During simulation, there are a few notions of metadata:

* The "state" is a set of wires
* Each wire has a timestamp, a value, and an enabled/disabled flag
* Each "step" in simulation will have an associated timestep as well, discussed below

Each component can be "run" and outputs the state of its output wires upon completion. We will describe what it means to run a toplevel component, which will also require running subcomponents. The base case of this recursive "run" will be the evaluation of Futil's primitive components, which will be described later.

## Simulation
Each simulation consists of a number of steps, as determined by the control statement in a component. Each "step" takes in the following:
* A control expression to evaluate
* A timestamp that represents the current logical timestep
* A state (set of wires, where each wire has timestamp, value, and an enabled/disabled flag)

Each step will produce a new state.

### Step
```
timestamp = T;
step(state, worklist, enabled) -> state
  if worklist empty
    return state
    
  // Run each component and collect their new states
  foreach component in worklist:
    if component is enabled:
      new_st = run(component, state)
    else
      new_st = set component outputs to disabled
  merged_st = merge all new_st
  
  // Compute new worklist
  worklist' = []
  foreach component
    foreach output of component
      if output.timestamp = T and componnt is enabled
        worklist'.push(output.dest)
  
  return step(merged_st, worklist')
```

### Simulate
```
simulate(st, c, timestamp) -> state

simulate(st, (seq c1 c2 ... cn), _timestamp):
  st1 = simulate(st, c1, 0)
  st2 = simulate(st1, c2, 1)
  ...
  stn = simulate(st_n-1, cn, n-1)
  return stn
  
simulate(st, (par c1 c2 ... cn), timestamp):
  st1 = simulate(st, c1, timestamp)
  st2 = simulate(st1, c2, timestamp)
  ...
  stn = simulate(st_n-1, cn, timestamp)
  return merge(st1, st2, ..., stn)
  
  .
  .
  .
  
simulate(st, (enable x1 x2 ... xn), timestamp):
  return step(st, all components, [x1, x2, ..., xn])
  
simulate(st, (disable x1 x2 ... xn), timestamp):
  return step(st, all components, all components - [x1, x2, ..., xn])
  
simulate(st, (empty), timestamp):
  return st
```

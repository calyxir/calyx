# Semantics for Futil
This document details the semantics for the Futil intermediate hardware description language.

## Basic Constructs
The basic building block in Futil is the `component`. Each component has the following properties:
* `inputs`: A set of input ports
* `outputs`: A set of output ports
* `structure`: Instances of subcomponents and wires between them
* `control`: High-level logical control-flow statements that determine the behavior of the subcomponents

## Simulation Semantics
We describe the semantics of Futil by describing how a component is executed. There are two parts to a component: it's structure and it's control. The structure defines all the subcomponents and the connections between them. The control defines "logical timesteps" and defines the order in which subcomponents are activated, or "run". The activation of a subcomponent means running it's control to completion and then collecting it's outputs. It is important to note that all subcomponents appear to run in a single logical timestep, regardless of how many logical timesteps the subcomponent actually executes.

Before we go further in depth about how the control works, we lay out what the state of a component looks like during execution. The state consists of:
* A set of wires connecting subcomponents
  * A wire is a set of "stamped values"
  * a stamped value is the triple (value, timestamp, enabled), where value is the data carried on the wire, a timestamp is a stamp from the logical time step that this value was produced, and the enabled flag determines whether the value is ready to be read by other subcomponents
* Each subcomponent can have associated state
  * This is used by things like memories and registers which can store values across logical timesteps

### Simulating a logical timestep
We now describe what it means for a single logical timestep to execute. This takes as input the state of the component and a list of modules that are "active" this logical timestep. All defined submodules that are not "active" are "inactive". All inactive subcomponents have their outputs set to disabled. 

The high level overview of the simulation is that we need to "run" all the subcomponents that are active using the initial state, then rerun any of successors of components that might have needed the values that were just produced. We continue to rerun any subcomponents until it becomes impossible for anything else to change. Note that this means that simulation may never terminate. We deal with loops by

It is important that the order we run components doesn't matter so that executing subcomponents has the semantics of being run in parallel. The result of the simulation of a logical timestep is a new component state.

Here is pseudocode that presents this idea more formally:
```
// input parameters that don't change between iterations
timestamp = T;
active = list of active subcomponents
step(state, worklist) -> state
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

The step function takes in an initial state, a list of all the components that may need to be updated, and a list of subcomponents that are active for this logical timestep, and a unique timestamp for this logical timestep.

If the worklist is not empty, then we loop through the worklist. For each component, we run each subcomponent with the current state. This gives us a set of new states, one from each component on the work list. We merge these into a single state. Then, for each subcomponent on the worklist, we look at it's output wires and add the successor of the wire to the worklist if the value on that output wire has a timestamp matching the current timestamp, regardless of whether the wire is enabled or not. We then recursively call step with the updated state and the new worklist.

If the worklist is empty, we are done and we return the state.

<!-- Each simulation consists of executing a sequence of logical steps. Each control statement takes a single logical step, and the `(seq ...)` -->
<!-- statement lets you define nested logical steps. For example, a whole `(seq (enable a b) (enable b c))` statement takes a single logical -->
<!-- step. However, the children of `seq`, namely `(enable a b)` `(enable b c)`, also each take a single logical step. This works because `seq` -->
<!-- creates a new "time scope", and then `(enable a b)` takes a step, then `(enable b c)` takes a step, and then we leave the scope. -->

<!-- The `simulate` function, described below, describes how the state is passed between different control statements and the `step` function -->
<!-- describes the semantics of a single step. -->

<!-- Each simulation consists of a number of steps, as determined by the control statement in a component. Each "step" takes in the following: -->
<!-- * A control expression to evaluate -->
<!-- * A timestamp that represents the current logical timestep -->
<!-- * A state (set of wires, where each wire has timestamp, value, and an enabled/disabled flag) -->

<!-- Each step will produce a new state. -->

### Simulating the entire control
Here we describe how each control statement works. Most of these simply recurse on their children and look very similar to a normal big step interpreter. The interesting things to note is that `(seq ...)` calls each of it's children with increasing timestamps. This lets the `seq` statement create "nested logical timesteps". The `enable` and `disable` statements are the statements that actually trigger the exeuction of the current components.
```
simulate(st, c, timestamp) -> state

simulate(st, (seq c1 c2 ... cn), _timestamp):
  st1 = simulate(st, c1, 0)
  st2 = simulate(st1, c2, 1)
  ...
  stn = simulate(st_n-1, cn, n-1)
  return stn
  
simulate(st, (par c1 c2 ... cn), _timestamp):
  st1 = simulate(st, c1, timestamp)
  st2 = simulate(st1, c2, timestamp)
  ...
  stn = simulate(st_n-1, cn, timestamp)
  return merge(st1, st2, ..., stn)
  
simulate(st, (if cond tbranch fbranch), timestamp):
  if cond is disabled:
    // if cond is disabled, (if ...) is a nop
    return st

  if cond != 0:
    return simulate(st, tbranch, timestamp)
  else cond = 0:
    return simulate(st, fbranch, timestamp)

simulate(st, (ifen cond tbranch fbranch), timestamp):
  if cond is enabled:
    return simulate(st, tbranch, timestamp)
  else cond is disabled:
    return simulate(st, fbranch, timestamp)

simulate(st, (while cond body), timestamp):
  if cond is disabled:
    // if cond is disabled, (while ...) is a nop
    return st

  if cond != 0:
    st1 = simulate(st, body, timestamp)
    return simulate(st1, (while cond body), timestamp + 1)
  else cond = 0:
    return st

simulate(st, (print x), timestamp):
  display the value of x
  return st
  
simulate(st, (enable x1 x2 ... xn), timestamp):
  return step(st, all components, [x1, x2, ..., xn])
  
simulate(st, (disable x1 x2 ... xn), timestamp):
  return step(st, all components, all components - [x1, x2, ..., xn])
  
simulate(st, (empty), timestamp):
  return st
```

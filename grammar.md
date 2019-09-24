```
namespace ::= ( define/namespace <name: string> <component>* )
component ::= ( 
                define/component
                <signature: comp-sig>   ; module signature (name and parameters)
                <inputs: port-def>*     ; input port definitions
                <outputs: port-def>*    ; output port definitions
                <structure>*            ; list of structure
                <control>               ; single control module
              )
              
comp-sig ::= ( <name: string> <param: number>* )
              
port-def ::= ( port <name: string> <port-width: number> )

structure ::= ( new <inst-name: string> <comp-inst> )
            | ( -> <src: port> <dest: port> )
            
port ::= ( @ <component: name> <port: name> )
       | ( @ this <port: name> )    ; used when referring to own component
       
comp-inst ::= ( <comp-name: string> <param: number>* )    ; component instancing expressions

control ::= ( seq <control>+ )
          | ( par <control>+ )
          | ( if <cond: port> <true_branch: control> <false_branch: control> )
          | ( ifen <cond: port> <true_branch: control> <false_branch: control> )
          | ( while <cond: port> <body: control> )
          | ( print <id: string> )    ; id is the id of a component instance  in structure
          | ( enable <id: string>+ )  ; enables just components with id <id>+
          | ( disable <id: string>+ ) ; deactivates just components with id <id>+
          | ( )                       ; empty control

```


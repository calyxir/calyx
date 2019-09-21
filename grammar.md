```
namespace ::= ( <name: string> <component>* )
component ::= ( 
                <name: string>          ;Name of module
                <inputs: port-def>*     ;Input Port definitions
                <outputs: port-def>*    ;Output Port definitions
                <structure>*            ;List of structure
                <control>               ;Single control module
              )
              
port-def ::= ( <name: string> <port-width: number> )

structure ::= ( new <name: string> <comp-inst> )
            | ( -> <src: port> <dest: port> )
            
port ::= ( @ <component: name> <port: name> )
       | ( @ this <port: name> )    ;Used when referring to own component
       
comp-inst ::= ( <name: string> <param: number>* )    ; Component Instancing Expressions

control ::= ( seq <control>+ )
          | ( par <control>+ )
          | ( if <cond: port> <true_branch: control> <false_branch: control> )
          | ( ifen <cond: port> <true_branch: control> <false_branch: control> )
          | ( while <cond: port> <body: control> )
          | ( print <id: string> )    ;id is the id of a component instance  in structure
          | ( )                       ; Empty control


```


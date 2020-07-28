if exists("b:current_syntax")
  finish
endif

" Numbers
syn match futilConstant "\v<[0-9]+('d[0-9]+)?>"
hi link futilConstant  Constant

syn region futilString start=/\v"/ skip=/\v\\./ end=/\v("|$)/
hi link futilString String

" Control statements
syn keyword futilControl while if with seq par
hi link futilControl Special

" Other keywords
syn keyword futilKeyword import cells wires control group prim
hi link futilKeyword Keyword

" Names of components and groups
syn keyword futilKeyword component group nextgroup=futilBoundName skipwhite
syn match futilBoundName '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained
hi link futilBoundName Include

" Highlight holes
syn keyword futilHole go done
hi link futilHole Type

" Delimiters
syn match futilOperator '!'
syn match futilOperator '!='
syn match futilOperator '='
syn match futilOperator '?'
syn match futilOperator '&'
syn match futilOperator '|'
hi link futilOperator Operator

" Comments
syntax match futilComment "\v//.*$"
hi link futilComment  Comment

let b:current_syntax = "futil"

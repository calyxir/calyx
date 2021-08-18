if exists("b:current_syntax")
  finish
endif

" Numbers
syn match futilConstant "\v<[0-9]+('d[0-9]+)?>"
hi link futilConstant  Constant

" String literals for attributes
syn region futilString start=/\v"/ skip=/\v\\./ end=/\v("|$)/
hi link futilString String

" @ style attributes
syn region futilAttr start=/\v\@[a-zA-Z_]+\(/ end=/\v\)/ contains=futilConstant
hi link futilAttr String

" Control statements
syn keyword futilControl while if with seq par invoke else
hi link futilControl Special

" Other keywords
syn keyword futilKeyword import cells wires control group extern
hi link futilKeyword Keyword

" Names of components and groups
syn keyword futilKeyword component group primitive nextgroup=futilBoundName skipwhite
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
syn region futilComment start=/\v\/\*/ skip=/\v\\./ end=/\v\*\//
hi link futilComment  Comment

let b:current_syntax = "futil"

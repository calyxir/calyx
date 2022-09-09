if exists("b:current_syntax")
  finish
endif

" Numbers
syn match futilConstant "\v<[0-9]+('(d|b|x|o)[0-9]+)?>"
hi link futilConstant  Constant

" String literals for attributes
syn region futilString start=/\v"/ skip=/\v\\./ end=/\v("|$)/
hi link futilString String

" @ style attributes
syn match futilAttr '\v\@[a-zA-Z_]+' nextgroup=futilAttrVal
syn region futilAttrVal start=/\v\(/ end=/\v\)/ contains=futilConstant,futilComment
hi link futilAttr String

" Control statements
syn keyword futilControl while if with seq par invoke else
hi link futilControl Special

" Other keywords
syn keyword futilKeyword import cells wires control group extern
hi link futilKeyword Keyword

" Primitive, component, and groups
syn keyword futilKeyword component group primitive nextgroup=futilBoundName skipwhite
syn match futilBoundName '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained nextgroup=futilAttrs,futilParams,futilPorts
hi link futilBoundName Include

" Parameters attached to primitives
syn region futilParams start=/\v\[/  end=/\v\]/ contains=futilParam,futilComment nextgroup=futilPorts skipwhite skipnl
syn match futilParam '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained
hi link futilParam Type

" Port definitions
syn region futilPorts start=/\v\(/  end=/\v\)/ contains=futilPortDef,futilAttr,futilComment contained
syn match futilPortDef '\v[_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*' contained nextgroup=futilDefColon skipwhite
syn match futilDefColon ':' contained nextgroup=futilPortParam skipwhite
syn match futilPortParam '\v([_a-zA-Z]((\-+)?[_a-zA-Z0-9]+)*)|([1-9][0-9]*)' contained
hi link futilPortParam Type

" Output ports come after the arrow
syn match futilArrow '->' nextgroup=futilPorts skipwhite skipnl
hi link futilArrow futilOperator

" Modifiers for components, groups, primitives
syn keyword futilModifier comb ref
hi link futilModifier Operator

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

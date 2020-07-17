from tvm.relay import Var
from typing import Any, Optional, List, Tuple
import attr


class futilNode:
    pass


@attr.s(auto_attribs=True)
class Decl(futilNode):
    bindings: List[Tuple[Var, futilNode]]


@attr.s(auto_attribs=True)
class FutilFunc(futilNode):
    params: List[Var]
    body: Any
    ret_type: Any
    name: Optional[str] = None

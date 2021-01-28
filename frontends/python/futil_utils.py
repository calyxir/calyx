import textwrap


def block(decl, contents, indent=2, sep='\n'):
    """Format a block like this:
        decl {
          contents[0] <sep> contents[1] ...
        }
    where `decl` is one line and contents is a list that is separated with `sep`.
    """
    return ''.join((decl, ' {\n', textwrap.indent(sep.join(contents), indent * ' '), '\n}'))

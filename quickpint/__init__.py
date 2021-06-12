from . import quickpint as fast_quickpint


def build_eval_tree(tokens):
    if not isinstance(tokens, list):
        tokens = list(tokens)
    
    return fast_quickpint.build_eval_tree(tokens)


__all__ = [
    "build_eval_tree"
]
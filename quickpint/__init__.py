import pint.pint_eval
import pint.registry
import pint.util

from . import quickpint as fast_quickpint

_original_build_eval_tree = pint.pint_eval.build_eval_tree

def build_eval_tree(tokens, *args):
    if not isinstance(tokens, list):
        tokens = list(tokens)
    return fast_quickpint.build_eval_tree(tokens)


def patch():
    pint.pint_eval.build_eval_tree = build_eval_tree
    pint.registry.build_eval_tree = build_eval_tree
    pint.util.build_eval_tree = build_eval_tree


def unpatch():
    pint.pint_eval.build_eval_tree = _original_build_eval_tree
    pint.registry.build_eval_tree = _original_build_eval_tree
    pint.util.build_eval_tree = _original_build_eval_tree

__all__ = [
    "build_eval_tree"
]
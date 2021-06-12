from tokenize import tokenize
from io import BytesIO
from quickpint import build_eval_tree


def test_quickpint():
    input = "gram / meter ** 2 / second"
    tokens = tokenize(BytesIO(input.encode("utf-8")).readline)
    assert build_eval_tree(tokens)

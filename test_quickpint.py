from hypothesis.strategies._internal.core import composite, recursive
from hypothesis.strategies._internal.numbers import integers
import quickpint
import pint
from math import exp, isnan
from pint.compat import tokenizer
import pint.pint_eval
import pytest
import importlib
from io import BytesIO
from hypothesis import given
from hypothesis.strategies import just, one_of, floats, lists

import tokenize


@pytest.fixture
def quick():
    quickpint.patch()
    yield
    quickpint.unpatch()
    importlib.reload(pint)


def test_quickpint_fast(quick, benchmark):
    registry = pint.UnitRegistry()

    units = ("meter", "kilometer", "second", "minute", "angstrom")
    expressions = [f"1.0 {unit} * 1 * 2 * 3 * 4 * 5" for unit in units]

    def _test():
        for expression in expressions:
            assert registry.parse_expression(expression)

    benchmark(_test)


def test_quickpint_slow(benchmark):
    registry = pint.UnitRegistry()

    units = ("meter", "kilometer", "second", "minute", "angstrom")
    expressions = [f"1.0 {unit} * 1 * 2 * 3 * 4 * 5" for unit in units]

    def _test():
        for expression in expressions:
            assert registry.parse_expression(expression)

    benchmark(_test)


@composite
def _expression(draw):
    operator = one_of(just("-") | just("+") | just("*") | just("/") | just("**"))

    values = draw(lists(integers(min_value=-8, max_value=8), min_size=1, max_size=16))

    new_values = []

    for value in values:
        new_values.append(str(value))
        new_values.append(draw(operator))

    return " ".join(new_values[:-1])


@given(_expression())
def test_random_expressions(expression):
    print("expression=", expression)
    assert len(expression) < 100
    tokens = list(tokenizer(expression))

    def eval(token):
        value = float(token.string)

        if value > 0:
            return min(2**32, value)
        else:
            return max(-2*32, value)

    try:
        slow_exc = None
        slow_parse = pint.pint_eval.build_eval_tree(tokens).evaluate(eval)
    except Exception as exc:
        slow_parse = None
        slow_exc = str(exc)

    try:
        fast_exc = None
        fast_parse = quickpint.build_eval_tree(tokens).evaluate(eval)
    except Exception as exc:
        fast_parse = None
        fast_exc = str(exc)

    assert slow_exc == fast_exc and (slow_parse == fast_parse or (isnan(slow_parse) and isnan(fast_parse)))

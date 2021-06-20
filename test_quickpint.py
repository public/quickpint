import quickpint
import pint
from math import isnan
from pint.compat import tokenizer
import pint.pint_eval
import pytest
import importlib
from hypothesis import given
from hypothesis.strategies import just, one_of, floats, lists, composite


@pytest.fixture
def quick():
    quickpint.patch()
    yield
    quickpint.unpatch()
    importlib.reload(pint)


def test_quickpint_fast_benchmark(quick, benchmark):
    registry = pint.UnitRegistry()

    units = ("meter", "kilometer", "second", "minute", "angstrom")
    expressions = [f"1.0 {unit} * 1 * 2 * 3 * 4 * 5" for unit in units]

    def _test():
        for expression in expressions:
            assert registry.parse_expression(expression)

    benchmark(_test)


def test_quickpint_slow_benchmark(benchmark):
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

    values = draw(lists(floats(), min_size=1, max_size=16))

    new_values = []

    for value in values:
        new_values.append(str(value))
        new_values.append(draw(operator))

    return " ".join(new_values[:-1])


@given(_expression())
def test_random_expressions(expression):
    tokens = list(tokenizer(expression))

    def eval(token):
        value = float(token.string)

        if value > 0:
            return min(2 ** 64, value)
        else:
            return max(-2 * 64, value)

    try:
        slow_exc = None
        slow_root = pint.pint_eval.build_eval_tree(tokens)
        slow_eval = slow_root.evaluate(eval)
    except Exception as exc:
        slow_eval = None
        slow_exc = str(exc)

    try:
        fast_exc = None
        fast_root = quickpint.build_eval_tree(tokens)
        fast_eval = fast_root.evaluate(eval)
    except Exception as exc:
        fast_eval = None
        fast_exc = str(exc)

    if slow_exc or fast_exc:
        assert slow_exc == fast_exc
    else:
        assert slow_eval == fast_eval or (isnan(slow_eval) and isnan(fast_eval))
        assert slow_root.to_string() == fast_root.to_string()


@given(_expression())
def test_tokenizer(expression):
    slow_tokens = list(tokenizer(expression))
    fast_tokens = quickpint.tokenizer(expression)

    slow_str = pint.pint_eval.build_eval_tree(slow_tokens).to_string()
    fast_str = pint.pint_eval.build_eval_tree(fast_tokens).to_string()

    assert slow_str == fast_str

import quickpint
import pint
import pytest
import importlib

from pint.compat import tokenizer

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
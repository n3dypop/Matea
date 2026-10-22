"""Evidence checks must run even when Python's assert statements are optimized out."""


def require(condition, message=None):
    if not condition:
        if message is None:
            raise AssertionError
        raise AssertionError(message() if callable(message) else message)

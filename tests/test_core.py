import json
import pytest
from pytrex.core import event, execute_python_event, REGISTERED_EVENTS, PyTreXApp, ElixirClient, BLOCKCHAIN_CACHE


def test_event_decorator():
    @event("test_event")
    def handler(data):
        return json.dumps({"status": "ok"})

    assert "test_event" in REGISTERED_EVENTS
    result = execute_python_event("test_event", "{}")
    assert json.loads(result)["status"] == "ok"


def test_unknown_event():
    result = execute_python_event("nonexistent_event", "{}")
    parsed = json.loads(result)
    assert parsed["status"] == "error"


def test_elixir_client_init():
    client = ElixirClient()
    assert client.host == "localhost"
    assert client.port == 42351
    assert client._connected is False


def test_blockchain_cache():
    assert isinstance(BLOCKCHAIN_CACHE, list)


def test_pytrex_app_init():
    app = PyTreXApp(name="Test App")
    assert app.name == "Test App"
    assert app.network is not None

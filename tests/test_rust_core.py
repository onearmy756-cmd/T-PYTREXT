import json
import pytest


def test_blockchain_creation():
    try:
        import my_framework
        block_json = my_framework.fanya_block_ya_blockchain("Test Transaction")
        block = json.loads(block_json)
        assert block["index"] >= 1
        assert len(block["hash"]) == 64
        assert block["data"] == "Test Transaction"
        assert block["previous_hash"] is not None
    except ImportError:
        pytest.skip("my_framework not built (run maturin develop first)")


def test_blockchain_audit():
    try:
        import my_framework
        block1 = json.loads(my_framework.fanya_block_ya_blockchain("TX1"))
        block2 = json.loads(my_framework.fanya_block_ya_blockchain("TX2"))
        chain = json.dumps([block1, block2])
        is_safe = my_framework.hakiki_blockchain(chain)
        assert is_safe is True
    except ImportError:
        pytest.skip("my_framework not built (run maturin develop first)")


def test_blockchain_tamper_detection():
    try:
        import my_framework
        block1 = json.loads(my_framework.fanya_block_ya_blockchain("TX1"))
        block2 = json.loads(my_framework.fanya_block_ya_blockchain("TX2"))
        block2["data"] = "TAMPERED"
        chain = json.dumps([block1, block2])
        is_safe = my_framework.hakiki_blockchain(chain)
        assert is_safe is False
    except ImportError:
        pytest.skip("my_framework not built (run maturin develop first)")


def test_database_creation():
    try:
        import my_framework
        import os
        db_path = "test_database.db"
        if os.path.exists(db_path):
            os.remove(db_path)
        my_framework.kuandaa_database_salama(db_path, "test_key_123")
        assert os.path.exists(db_path)
        # Note: Can't remove db file because SQLx pool holds it open in static variable.
        # This is expected behavior — the pool persists for the lifetime of the process.
    except ImportError:
        pytest.skip("my_framework not built (run maturin develop first)")

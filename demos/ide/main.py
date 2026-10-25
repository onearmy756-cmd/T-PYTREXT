# main.py — MBILINYI IDE (OpenHands Agent Canvas + zana zote za IDE)
#
# Demo hii inaonyesha jinsi ya kutumia `pytrex.ide` — moduli ya IDE iliyoingizwa
# kutoka MTECHOS kwa architecture ya T-PYTREXT.
#
#   python demos/ide/main.py            # hali + plan (hakuna kubadilisha kitu)
#   python demos/ide/main.py --start    # build canvas + anzisha backend
#   python demos/ide/main.py --tools    # orodhesha zana zote za IDE
#
import json
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", ".."))

from pytrex import MBILINYIIDE, event, execute_python_event  # noqa: E402


# `@event` hurekodi function moja (bila `self`), hivyo handlers za demo ni
# module-level — kama inavyofanya `execute_python_event` ya framework.
@event("demo_open_workspace")
def open_workspace(data):
    """Mfano wa @event unaotumia engine ya IDE inayoshirikiwa."""
    payload = json.loads(data or "{}")
    status = MBILINYIIDE().engine.status()
    return json.dumps({
        "status": "ok",
        "data": {
            "workspace": payload.get("path", "."),
            "ide_available": status["available"],
            "components": list(status["components"].keys()),
        },
    })


def main():
    args = sys.argv[1:]
    app = MBILINYIIDE(name="MBILINYI IDE Demo")

    if "--tools" in args:
        print(json.dumps(app.ide_tools.catalog(), indent=2))
        return

    if "--start" in args:
        print(json.dumps(app.start(), indent=2))
        return

    # Default: hali + plan (hakuna kubadilisha kitu)
    print("== IDE status ==")
    print(json.dumps(app.status(), indent=2))
    print("\n== IDE plan ==")
    print(json.dumps(app.engine.plan(), indent=2))
    print("\n== @event: demo_open_workspace ==")
    print(execute_python_event("demo_open_workspace", json.dumps({"path": "."})))
    print("\n== @event: ide_tools ==")
    print(execute_python_event("ide_tools", "{}")[:200], "...")


if __name__ == "__main__":
    main()

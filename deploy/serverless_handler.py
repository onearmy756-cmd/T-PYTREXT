"""
T-PYTREXT SERVERLESS HANDLER
=============================
Generic serverless handler for AWS Lambda, GCP Cloud Run, Azure Functions.
"""
import json, os, sys

# Auto-import pytrex
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from pytrex.core import execute_python_event, PyTreXApp

# Single shared app instance (Lambda reuses warm containers)
_app = None

def get_app():
    global _app
    if _app is None:
        _app = PyTreXApp(name=os.environ.get("PYTREX_APP_NAME", "serverless"))
    return _app

def handler(event, context=None):
    """
    Universal serverless handler.
    Works with: AWS Lambda, GCP Cloud Functions, Azure Functions
    
    Event format: {"event": "event_name", "data": {...}}
    """
    try:
        # Parse input
        if isinstance(event, str):
            event = json.loads(event)
        
        body = event.get("body", event)
        if isinstance(body, str):
            body = json.loads(body)
        
        event_name = body.get("event", "ping")
        event_data = body.get("data", {})
        
        if isinstance(event_data, dict):
            event_data = json.dumps(event_data)
        
        # Execute
        result = execute_python_event(event_name, event_data)
        
        return {
            "statusCode": 200,
            "headers": {
                "Content-Type": "application/json",
                "X-Powered-By": "T-PYTREXT"
            },
            "body": result
        }
    except Exception as e:
        return {
            "statusCode": 500,
            "body": json.dumps({"error": str(e)})
        }

# For local testing
if __name__ == "__main__":
    print(handler({"event": "ping", "data": {"test": True}}))

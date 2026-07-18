import os, sys

PASSENGER = '''import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from pytrex.core import PyTreXApp
application = PyTreXApp(name="PyTreX cPanel")
'''

HTACCESS = """PassengerAppType wsgi
PassengerStartupFile passenger_wsgi.py
PassengerPython /usr/bin/python3
"""

def main():
    with open("passenger_wsgi.py", "w") as f: f.write(PASSENGER)
    with open(".htaccess", "w") as f: f.write(HTACCESS)
    os.system(f"{sys.executable} -m pip install pytrex-framework")
    print("[PyTreX cPanel] Ready! Setup in cPanel > Python App")

if __name__ == "__main__": main()

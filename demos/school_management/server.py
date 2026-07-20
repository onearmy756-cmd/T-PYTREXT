"""
T-PYTREXT School Management — Web Server
Run: python server.py → Open http://localhost:8080
"""
import http.server, json, sys, os
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from main import SchoolManagementSystem
school = SchoolManagementSystem(school_name="Smart Academy")

class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/':
            self.send_response(200)
            self.send_header('Content-Type','text/html; charset=utf-8')
            self.send_header('Access-Control-Allow-Origin','*')
            self.end_headers()
            with open(os.path.join(os.path.dirname(__file__),'dashboard.html'),'r',encoding='utf-8') as f:
                self.wfile.write(f.read().encode())
            return

        if self.path.startswith('/api/'):
            self.send_response(200)
            self.send_header('Content-Type','application/json')
            self.send_header('Access-Control-Allow-Origin','*')
            self.end_headers()

            if self.path == '/api/dashboard':
                self.wfile.write(school.dashboard("{}").encode())
            elif self.path == '/api/students':
                self.wfile.write(school.list_students('{}').encode())
            elif self.path == '/api/teachers':
                self.wfile.write(school.list_teachers('{}').encode())
            elif self.path == '/api/analytics':
                self.wfile.write(school.school_analytics('{}').encode())
            elif self.path == '/api/fees':
                self.wfile.write(school.fee_report('{}').encode())
            elif self.path == '/api/classes':
                classes = [{"id": c["id"], "name": c["name"], "students": c["students_count"]}
                          for c in school.classes.values()]
                self.wfile.write(json.dumps(classes).encode())
            else:
                self.wfile.write(json.dumps({"status":"ok"}).encode())
            return

        super().do_GET()

port = 8080
print(f'🏫 Smart Academy running on http://localhost:{port}')
http.server.HTTPServer(('127.0.0.1',port), Handler).serve_forever()

"""
T-PYTREX School Management — Full API Server with POST support
Run: python server.py → Open http://localhost:8080
"""
import http.server, json, sys, os, io, urllib.parse
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from main import SchoolManagementSystem
school = SchoolManagementSystem(school_name="Smart Academy")

class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        self._cors()
        if self.path == '/' or self.path == '/dashboard':
            self._html('dashboard.html')
            return
        if self.path.startswith('/api/'):
            self._json(self._handle_api(self.path))
            return
        super().do_GET()

    def do_POST(self):
        self._cors()
        data = self._parse_body()

        if self.path.startswith('/api/'):
            result = self._handle_api_post(self.path, data)
            self._json(result)
            return
        self.send_response(404); self.end_headers()

    def _cors(self):
        self.send_response(200)
        self.send_header('Access-Control-Allow-Origin','*')
        self.send_header('Access-Control-Allow-Methods','GET,POST,OPTIONS')
        self.send_header('Access-Control-Allow-Headers','Content-Type')

    def _html(self, filename):
        self.send_header('Content-Type','text/html; charset=utf-8')
        self.end_headers()
        path = os.path.join(os.path.dirname(__file__), filename)
        with open(path,'r',encoding='utf-8') as f:
            self.wfile.write(f.read().encode())

    def _json(self, data):
        self.send_header('Content-Type','application/json')
        self.end_headers()
        self.wfile.write(json.dumps(data).encode())

    def _parse_body(self):
        length = int(self.headers.get('Content-Length', 0))
        if length > 0:
            body = self.rfile.read(length)
            return json.loads(body)
        return {}

    def _handle_api(self, path):
        if path == '/api/dashboard':
            return json.loads(school.dashboard("{}"))
        elif path == '/api/students':
            return json.loads(school.list_students('{}'))
        elif path == '/api/teachers':
            return json.loads(school.list_teachers('{}'))
        elif path == '/api/analytics':
            return json.loads(school.school_analytics('{}'))
        elif path == '/api/fees':
            return json.loads(school.fee_report('{}'))
        elif path == '/api/classes':
            return [{"id":c["id"],"name":c["name"]} for c in school.classes.values()]
        elif path.startswith('/api/class_results'):
            import urllib.parse
            qs = urllib.parse.urlparse(self.path).query
            params = dict(urllib.parse.parse_qsl(qs))
            return json.loads(school.class_results(json.dumps({
                "class_id": params.get("class_id","FORM-1"),
                "exam_name": params.get("exam_name","Mid-Term")
            })))
        return {"status":"ok"}

    def _handle_api_post(self, path, data):
        if path == '/api/register_student':
            return json.loads(school.register_student(json.dumps(data)))
        elif path == '/api/record_fee_payment':
            return json.loads(school.record_fee_payment(json.dumps(data)))
        elif path == '/api/enter_exam_results':
            return json.loads(school.enter_exam_results(json.dumps(data)))
        elif path == '/api/issue_certificate':
            return json.loads(school.issue_certificate(json.dumps(data)))
        elif path == '/api/mark_attendance':
            return json.loads(school.mark_attendance(json.dumps(data)))
        return {"error":"Unknown endpoint"}

port = 8080
print(f'🏫 Smart Academy running on http://localhost:{port}')
print('📡 API: GET /api/dashboard, POST /api/register_student, etc')
http.server.HTTPServer(('127.0.0.1',port), Handler).serve_forever()

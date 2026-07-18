"""
PyTreX School Management System
================================
A complete school management system built with PyTreX framework.

Features:
  - Multi-school (multi-tenant)
  - Authentication & RBAC (admin, teacher, student, parent)
  - Student/Teacher/Course management (ORM)
  - Attendance tracking (TimeSeriesDB)
  - Grades & Exams
  - Fee management (Payments)
  - Timetable/Scheduling
  - Notifications (Email, SMS)
  - Audit trail (compliance)
  - Analytics & reports
  - Library management
  - Real-time notifications
  - API server (REST)
  - Version control (data versioning)
  - A/B testing (UI experiments)
  - Content moderation (messages)
  - AI assistant (RAG chatbot for students)
  - Security scanning
  - Feature analytics

Usage:
  python school_management.py
  Then open: http://localhost:8080
"""

from pytrex import PyTreXApp
import json
import time

# ═══════════════════════════════════════════════════════════
# 1. INITIALIZE THE SYSTEM
# ═══════════════════════════════════════════════════════════

app = PyTreXApp("PyTreX School Management System")
SCHOOL_NAME = "Bright Future Academy"

# ═══════════════════════════════════════════════════════════
# 2. MULTI-TENANT SETUP (Multiple Schools)
# ═══════════════════════════════════════════════════════════

app.tenants.create("bright_future", SCHOOL_NAME, "enterprise")
app.tenants.create("st_marys", "St. Mary's Secondary", "pro")
app.tenants.set_config("bright_future", "academic_year", "2026")
app.tenants.set_config("bright_future", "term", "Term 1")
app.tenants.set_limit("bright_future", "students", 500)
app.tenants.set_limit("bright_future", "teachers", 50)

# ═══════════════════════════════════════════════════════════
# 3. PERMISSIONS (RBAC)
# ═══════════════════════════════════════════════════════════

app.permissions.define_role("admin")
app.permissions.define_role("teacher")
app.permissions.define_role("student")
app.permissions.define_role("parent")

# Admin: full access
app.permissions.grant("admin", "*", "*")

# Teacher: manage students, grades, attendance
app.permissions.grant("teacher", "read", "students")
app.permissions.grant("teacher", "write", "grades")
app.permissions.grant("teacher", "write", "attendance")
app.permissions.grant("teacher", "read", "courses")
app.permissions.grant("teacher", "write", "assignments")
app.permissions.grant("teacher", "send", "messages")

# Student: read courses, submit assignments
app.permissions.grant("student", "read", "courses")
app.permissions.grant("student", "read", "grades")
app.permissions.grant("student", "write", "assignments")
app.permissions.grant("student", "read", "attendance")
app.permissions.grant("student", "send", "messages")

# Parent: view child's grades, attendance, fees
app.permissions.grant("parent", "read", "grades")
app.permissions.grant("parent", "read", "attendance")
app.permissions.grant("parent", "read", "fees")
app.permissions.grant("parent", "pay", "fees")

# ═══════════════════════════════════════════════════════════
# 4. ORM MODELS (Database Tables)
# ═══════════════════════════════════════════════════════════

app.orm.define("Student",
    student_id=str, name=str, email=str,
    class_id=str, parent_email=str,
    enrolled_date=str, status=str,
)

app.orm.define("Teacher",
    teacher_id=str, name=str, email=str,
    subject=str, hired_date=str, status=str,
)

app.orm.define("Course",
    course_id=str, name=str, teacher_id=str,
    class_id=str, credits=int,
)

app.orm.define("Grade",
    student_id=str, course_id=str, exam_type=str,
    score=float, max_score=float, term=str,
    recorded_by=str, recorded_date=str,
)

app.orm.define("Assignment",
    assignment_id=str, course_id=str, title=str,
    description=str, due_date=str, max_marks=float,
)

app.orm.define("Submission",
    submission_id=str, assignment_id=str, student_id=str,
    content=str, submitted_date=str, marks=float, graded=bool,
)

app.orm.define("Fee",
    fee_id=str, student_id=str, fee_type=str,
    amount=float, due_date=str, status=str,
    paid_date=str, method=str,
)

app.orm.define("LibraryItem",
    item_id=str, title=str, author=str,
    isbn=str, category=str, available=bool,
    borrowed_by=str, due_date=str,
)

app.orm.define("Message",
    message_id=str, sender=str, recipient=str,
    subject=str, body=str, read=bool, sent_date=str,
)

# ═══════════════════════════════════════════════════════════
# 5. SEED DATA (Initial Records)
# ═══════════════════════════════════════════════════════════

# ── Users ──
app.auth.register_user("admin", "admin123", role="admin")
app.auth.register_user("teacher1", "teach123", role="teacher")
app.auth.register_user("teacher2", "teach123", role="teacher")
app.auth.register_user("student1", "stud123", role="student")
app.auth.register_user("student2", "stud123", role="student")
app.auth.register_user("parent1", "parent123", role="parent")

app.permissions.assign_role("admin", "admin")
app.permissions.assign_role("teacher1", "teacher")
app.permissions.assign_role("teacher2", "teacher")
app.permissions.assign_role("student1", "student")
app.permissions.assign_role("student2", "student")
app.permissions.assign_role("parent1", "parent")

# ── Teachers ──
app.orm.create("Teacher",
    teacher_id="T001", name="Mr. Mangi", email="mangi@school.ac.tz",
    subject="Mathematics", hired_date="2025-01-15", status="active",
)
app.orm.create("Teacher",
    teacher_id="T002", name="Ms. Amina", email="amina@school.ac.tz",
    subject="Science", hired_date="2025-02-01", status="active",
)

# ── Students ──
app.orm.create("Student",
    student_id="S001", name="Juma Ali", email="juma@school.ac.tz",
    class_id="C1A", parent_email="parent1@family.com",
    enrolled_date="2026-01-08", status="active",
)
app.orm.create("Student",
    student_id="S002", name="Fatma Hassan", email="fatma@school.ac.tz",
    class_id="C1A", parent_email="parent2@family.com",
    enrolled_date="2026-01-08", status="active",
)

# ── Courses ──
app.orm.create("Course",
    course_id="MATH101", name="Mathematics Form 1",
    teacher_id="T001", class_id="C1A", credits=4,
)
app.orm.create("Course",
    course_id="SCI101", name="Science Form 1",
    teacher_id="T002", class_id="C1A", credits=4,
)

# ── Library ──
app.orm.create("LibraryItem",
    item_id="B001", title="Advanced Mathematics", author="Dr. Mwakyusa",
    isbn="978-9987-123-456", category="Mathematics",
    available=True, borrowed_by="", due_date="",
)
app.orm.create("LibraryItem",
    item_id="B002", title="Physics for Beginners", author="Dr. Shayo",
    isbn="978-9987-654-321", category="Science",
    available=True, borrowed_by="", due_date="",
)

# ── Fees ──
app.orm.create("Fee",
    fee_id="F001", student_id="S001", fee_type="Tuition Term 1",
    amount=500000.0, due_date="2026-02-01", status="pending",
    paid_date="", method="",
)
app.orm.create("Fee",
    fee_id="F002", student_id="S002", fee_type="Tuition Term 1",
    amount=500000.0, due_date="2026-02-01", status="pending",
    paid_date="", method="",
)

# ── Version control initial state ──
app.versions.commit("school_data", {"students": 2, "teachers": 2, "courses": 2},
                    "Initial school data", "admin")

# ═══════════════════════════════════════════════════════════
# 6. TIMETABLE
# ═══════════════════════════════════════════════════════════

timetable = {
    "Monday":    [["08:00", "MATH101", "T001"], ["10:00", "SCI101", "T002"]],
    "Tuesday":   [["08:00", "SCI101", "T002"], ["10:00", "MATH101", "T001"]],
    "Wednesday": [["08:00", "MATH101", "T001"], ["10:00", "SCI101", "T002"]],
    "Thursday":  [["08:00", "SCI101", "T002"], ["10:00", "MATH101", "T001"]],
    "Friday":    [["08:00", "MATH101", "T001"], ["10:00", "SCI101", "T002"]],
}
app.tenants.set_data("bright_future", "timetable", timetable)

# ═══════════════════════════════════════════════════════════
# 7. A/B TESTING
# ═══════════════════════════════════════════════════════════

app.ab.create("dashboard_layout", ["classic", "modern"], "Test dashboard UI")
app.ab.create("fee_reminder", ["email", "sms"], "Test fee reminder channel")

# ═══════════════════════════════════════════════════════════
# 8. AI ASSISTANT (RAG Chatbot)
# ═══════════════════════════════════════════════════════════

ai_knowledge = [
    "Mathematics Form 1 covers algebra, geometry, and arithmetic.",
    "Science Form 1 covers biology, chemistry, and physics basics.",
    "School fees for Term 1 are 500,000 TZS due by February 1st.",
    "Library is open from 8am to 5pm on weekdays.",
    "To submit an assignment, go to Assignments then Submit.",
    "Grades are calculated as 40 percent coursework plus 60 percent final exam.",
]
for text in ai_knowledge:
    app.rag.add_document(f"doc_{int(time.time()*1000)}", text)

# ═══════════════════════════════════════════════════════════
# 9. NOTIFICATION CHANNELS
# ═══════════════════════════════════════════════════════════

app.notifications.register_channel("email", lambda title, body: app.email.send("all@school.ac.tz", title, body))
app.notifications.register_channel("sms", lambda title, body: app.sms.send("parent@family.com", title + ": " + body))

# ═══════════════════════════════════════════════════════════
# 10. HELPER FUNCTIONS
# ═══════════════════════════════════════════════════════════

def parse_body(raw):
    """Parse JSON body from raw request data."""
    if isinstance(raw, str):
        try:
            return json.loads(raw)
        except (json.JSONDecodeError, ValueError):
            return {}
    if isinstance(raw, dict):
        return raw
    return {}

import threading

_request_ctx = threading.local()

def set_request_headers(headers):
    """Set headers for current request (thread-local)."""
    _request_ctx.headers = headers or {}

def get_request_headers():
    """Get headers for current request."""
    h = getattr(_request_ctx, "headers", {})
    if h:
        return h
    try:
        return getattr(app.api._ctx, "headers", {})
    except AttributeError:
        return {}

_current_headers = {}

def make_req(raw_body="", headers=None, params=None, query=None, path=""):
    """Build a request dict for endpoint handlers."""
    h = headers or get_request_headers() or _current_headers or {}
    return {
        "json": parse_body(raw_body),
        "headers": h,
        "params": params or {},
        "query": query or {},
        "path": path,
        "raw": raw_body,
    }

def set_headers(headers):
    """Set current request headers (for testing)."""
    global _current_headers
    _current_headers = headers

def ok(data):
    return {"status": 200, "body": json.dumps(data, default=str),
            "headers": {"Content-Type": "application/json"}}

def err(msg, code=400):
    return {"status": code, "body": json.dumps({"status": "error", "message": msg}, default=str),
            "headers": {"Content-Type": "application/json"}}

def forbidden():
    return err("No permission", 403)

def check_perm(req, action, resource):
    user = req.get("headers", {}).get("X-User", "")
    if not app.permissions.check(user, action, resource):
        app.audit.log(user, "denied", resource, None, None, {"action": action})
        return False
    return True

# ═══════════════════════════════════════════════════════════
# 11. API ENDPOINTS — AUTH
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/auth/login")
def login(raw):
    req = make_req(raw)
    data = req["json"]
    username = data.get("username", "")
    password = data.get("password", "")
    token = app.auth.login(username, password)
    if token:
        user_info = app.auth.verify_token(token)
        role = user_info.get("role", "guest") if user_info else "guest"
        app.audit.log(username, "login", "system")
        app.usage.track("login", username)
        return ok({"status": "ok", "username": username, "role": role, "token": token})
    return err("Invalid credentials", 401)

@app.api.endpoint("/api/auth/logout")
def logout(raw):
    req = make_req(raw)
    user = req["headers"].get("X-User", "")
    app.audit.log(user, "logout", "system")
    return ok({"status": "ok", "message": "Logged out"})

# ═══════════════════════════════════════════════════════════
# 12. API ENDPOINTS — STUDENTS
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/students")
def students(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "students"):
        return forbidden()
    students_list = app.orm.query("Student").all()
    app.usage.track("view_students", req["headers"].get("X-User", ""))
    return ok({"status": "ok", "count": len(students_list), "students": students_list})

@app.api.endpoint("/api/students/create")
def create_student(raw):
    req = make_req(raw)
    if not check_perm(req, "write", "students"):
        return forbidden()
    data = req["json"]
    result = app.orm.create("Student",
        student_id=data.get("student_id", f"S{int(time.time())}"),
        name=data.get("name", ""),
        email=data.get("email", ""),
        class_id=data.get("class_id", ""),
        parent_email=data.get("parent_email", ""),
        enrolled_date=data.get("enrolled_date", ""),
        status="active",
    )
    app.audit.log(req["headers"].get("X-User", ""), "create", "student", None, data)
    app.tenants.record_usage("bright_future", "students", 1)
    app.email.send(data.get("email", ""), "Welcome to School",
                   f"Hello {data.get('name', '')}, welcome to {SCHOOL_NAME}!")
    return ok(result)

@app.api.endpoint("/api/students/detail")
def student_detail(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "students"):
        return forbidden()
    data = req["json"]
    sid = data.get("student_id", "")
    student = app.orm.query("Student").filter(student_id=sid).first()
    if not student:
        return err("Student not found", 404)
    grades = app.orm.query("Grade").filter(student_id=sid).all()
    fees = app.orm.query("Fee").filter(student_id=sid).all()
    attendance = app.timeseries.query(f"attendance_{sid}")
    return ok({"status": "ok", "student": student,
               "grades": grades, "fees": fees, "attendance_count": len(attendance)})

# ═══════════════════════════════════════════════════════════
# 13. API ENDPOINTS — TEACHERS
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/teachers")
def teachers(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "students"):
        return forbidden()
    teachers_list = app.orm.query("Teacher").all()
    return ok({"status": "ok", "count": len(teachers_list), "teachers": teachers_list})

@app.api.endpoint("/api/teachers/create")
def create_teacher(raw):
    req = make_req(raw)
    if not check_perm(req, "write", "students"):
        return forbidden()
    data = req["json"]
    result = app.orm.create("Teacher",
        teacher_id=data.get("teacher_id", f"T{int(time.time())}"),
        name=data.get("name", ""),
        email=data.get("email", ""),
        subject=data.get("subject", ""),
        hired_date=data.get("hired_date", ""),
        status="active",
    )
    app.audit.log(req["headers"].get("X-User", ""), "create", "teacher", None, data)
    return ok(result)

# ═══════════════════════════════════════════════════════════
# 14. API ENDPOINTS — COURSES
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/courses")
def courses(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "courses"):
        return forbidden()
    courses_list = app.orm.query("Course").all()
    return ok({"status": "ok", "count": len(courses_list), "courses": courses_list})

@app.api.endpoint("/api/courses/create")
def create_course(raw):
    req = make_req(raw)
    if not check_perm(req, "write", "courses"):
        return forbidden()
    data = req["json"]
    result = app.orm.create("Course",
        course_id=data.get("course_id", f"C{int(time.time())}"),
        name=data.get("name", ""),
        teacher_id=data.get("teacher_id", ""),
        class_id=data.get("class_id", ""),
        credits=data.get("credits", 3),
    )
    app.audit.log(req["headers"].get("X-User", ""), "create", "course", None, data)
    return ok(result)

# ═══════════════════════════════════════════════════════════
# 15. API ENDPOINTS — GRADES
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/grades/record")
def record_grade(raw):
    req = make_req(raw)
    if not check_perm(req, "write", "grades"):
        return forbidden()
    data = req["json"]
    result = app.orm.create("Grade",
        student_id=data.get("student_id", ""),
        course_id=data.get("course_id", ""),
        exam_type=data.get("exam_type", "test"),
        score=float(data.get("score", 0)),
        max_score=float(data.get("max_score", 100)),
        term=data.get("term", "Term 1"),
        recorded_by=req["headers"].get("X-User", ""),
        recorded_date=str(int(time.time())),
    )
    app.audit.log(req["headers"].get("X-User", ""), "record", "grade", None, data)
    student = app.orm.query("Student").filter(student_id=data.get("student_id", "")).first()
    if student and student.get("parent_email"):
        app.email.send(student["parent_email"], "Grade Update",
                       f"Your child received {data.get('score')}/{data.get('max_score')} "
                       f"in {data.get('course_id')}")
    return ok(result)

@app.api.endpoint("/api/grades/view")
def view_grades(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "grades"):
        return forbidden()
    data = req["json"]
    sid = data.get("student_id", "")
    grades = app.orm.query("Grade").filter(student_id=sid).all()
    total_score = sum(g.get("score", 0) for g in grades)
    total_max = sum(g.get("max_score", 100) for g in grades)
    pct = (total_score / total_max * 100) if total_max > 0 else 0
    return ok({"status": "ok", "student_id": sid, "grades": grades,
               "count": len(grades), "overall_percentage": round(pct, 2)})

# ═══════════════════════════════════════════════════════════
# 16. API ENDPOINTS — ATTENDANCE
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/attendance/record")
def record_attendance(raw):
    req = make_req(raw)
    if not check_perm(req, "write", "attendance"):
        return forbidden()
    data = req["json"]
    sid = data.get("student_id", "")
    present = data.get("present", True)
    app.timeseries.write(f"attendance_{sid}", 1.0 if present else 0.0,
                         tags={"course": data.get("course_id", ""),
                               "teacher": req["headers"].get("X-User", "")})
    app.audit.log(req["headers"].get("X-User", ""), "record", "attendance",
                  None, {"student_id": sid, "present": present})
    return ok({"status": "ok", "student_id": sid, "present": present})

@app.api.endpoint("/api/attendance/view")
def view_attendance(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "attendance"):
        return forbidden()
    data = req["json"]
    sid = data.get("student_id", "")
    records = app.timeseries.query(f"attendance_{sid}")
    present_count = sum(1 for r in records if r["value"] == 1.0)
    total = len(records)
    rate = (present_count / total * 100) if total > 0 else 0
    return ok({"status": "ok", "student_id": sid, "total_days": total,
               "present_days": present_count, "absent_days": total - present_count,
               "attendance_rate": round(rate, 2)})

# ═══════════════════════════════════════════════════════════
# 17. API ENDPOINTS — ASSIGNMENTS
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/assignments/create")
def create_assignment(raw):
    req = make_req(raw)
    if not check_perm(req, "write", "assignments"):
        return forbidden()
    data = req["json"]
    result = app.orm.create("Assignment",
        assignment_id=f"A{int(time.time())}",
        course_id=data.get("course_id", ""),
        title=data.get("title", ""),
        description=data.get("description", ""),
        due_date=data.get("due_date", ""),
        max_marks=float(data.get("max_marks", 100)),
    )
    app.audit.log(req["headers"].get("X-User", ""), "create", "assignment", None, data)
    return ok(result)

@app.api.endpoint("/api/assignments/submit")
def submit_assignment(raw):
    req = make_req(raw)
    if not check_perm(req, "write", "assignments"):
        return forbidden()
    data = req["json"]
    content = data.get("content", "")
    moderation = app.moderator.check_text(content)
    if not moderation["clean"]:
        return err(f"Content flagged: {moderation['flags']}", 400)
    result = app.orm.create("Submission",
        submission_id=f"SUB{int(time.time())}",
        assignment_id=data.get("assignment_id", ""),
        student_id=data.get("student_id", ""),
        content=content,
        submitted_date=str(int(time.time())),
        marks=0.0, graded=False,
    )
    app.audit.log(data.get("student_id", ""), "submit", "assignment", None, data)
    return ok(result)

@app.api.endpoint("/api/assignments/grade")
def grade_submission(raw):
    req = make_req(raw)
    if not check_perm(req, "write", "grades"):
        return forbidden()
    data = req["json"]
    sub = app.orm.query("Submission").filter(submission_id=data.get("submission_id", "")).first()
    if not sub:
        return err("Submission not found", 404)
    app.orm.update("Submission", sub["id"],
                   marks=float(data.get("marks", 0)), graded=True)
    app.audit.log(req["headers"].get("X-User", ""), "grade", "submission", None, data)
    return ok({"status": "ok", "message": "Graded successfully"})

# ═══════════════════════════════════════════════════════════
# 18. API ENDPOINTS — FEES
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/fees")
def fees(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "fees"):
        return forbidden()
    data = req["json"]
    sid = data.get("student_id", "")
    if sid:
        fees_list = app.orm.query("Fee").filter(student_id=sid).all()
    else:
        fees_list = app.orm.query("Fee").all()
    return ok({"status": "ok", "count": len(fees_list), "fees": fees_list})

@app.api.endpoint("/api/fees/pay")
def pay_fee(raw):
    req = make_req(raw)
    if not check_perm(req, "pay", "fees"):
        return forbidden()
    data = req["json"]
    fee = app.orm.query("Fee").filter(fee_id=data.get("fee_id", "")).first()
    if not fee:
        return err("Fee not found", 404)
    app.orm.update("Fee", fee["id"],
                   status="paid", paid_date=str(int(time.time())),
                   method=data.get("method", "mpesa"))
    app.audit.log(req["headers"].get("X-User", ""), "pay", "fee", None, fee)
    student = app.orm.query("Student").filter(student_id=fee["student_id"]).first()
    if student:
        app.email.send(student.get("email", ""), "Fee Payment Receipt",
                       f"Payment of {fee['amount']} TZS received. Thank you!")
    return ok({"status": "ok", "fee_id": fee["fee_id"], "message": "Payment successful"})

# ═══════════════════════════════════════════════════════════
# 19. API ENDPOINTS — LIBRARY
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/library")
def library(raw):
    items = app.orm.query("LibraryItem").all()
    available = [i for i in items if i.get("available")]
    return ok({"status": "ok", "total": len(items), "available": len(available), "items": items})

@app.api.endpoint("/api/library/borrow")
def borrow_book(raw):
    req = make_req(raw)
    data = req["json"]
    item = app.orm.query("LibraryItem").filter(item_id=data.get("item_id", "")).first()
    if not item:
        return err("Book not found", 404)
    if not item.get("available"):
        return err("Book not available", 400)
    app.orm.update("LibraryItem", item["id"],
                   available=False, borrowed_by=data.get("student_id", ""),
                   due_date=data.get("due_date", ""))
    app.audit.log(data.get("student_id", ""), "borrow", "book", None, item)
    return ok({"status": "ok", "message": "Book borrowed"})

@app.api.endpoint("/api/library/return")
def return_book(raw):
    req = make_req(raw)
    data = req["json"]
    item = app.orm.query("LibraryItem").filter(item_id=data.get("item_id", "")).first()
    if not item:
        return err("Book not found", 404)
    app.orm.update("LibraryItem", item["id"],
                   available=True, borrowed_by="", due_date="")
    app.audit.log(data.get("student_id", ""), "return", "book", None, item)
    return ok({"status": "ok", "message": "Book returned"})

# ═══════════════════════════════════════════════════════════
# 20. API ENDPOINTS — MESSAGES
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/messages/send")
def send_message(raw):
    req = make_req(raw)
    if not check_perm(req, "send", "messages"):
        return forbidden()
    data = req["json"]
    body_text = data.get("body", "")
    moderation = app.moderator.check_text(body_text)
    if not moderation["clean"]:
        app.audit.log(req["headers"].get("X-User", ""), "flagged", "message",
                      None, {"flags": moderation["flags"]})
        return err(f"Message flagged: {moderation['flags']}", 400)
    result = app.orm.create("Message",
        message_id=f"M{int(time.time())}",
        sender=req["headers"].get("X-User", ""),
        recipient=data.get("recipient", ""),
        subject=data.get("subject", ""),
        body=body_text, read=False,
        sent_date=str(int(time.time())),
    )
    app.audit.log(req["headers"].get("X-User", ""), "send", "message", None, data)
    return ok(result)

@app.api.endpoint("/api/messages/inbox")
def inbox(raw):
    req = make_req(raw)
    user = req["headers"].get("X-User", "")
    messages = app.orm.query("Message").filter(recipient=user).all()
    unread = [m for m in messages if not m.get("read")]
    return ok({"status": "ok", "total": len(messages), "unread": len(unread), "messages": messages})

# ═══════════════════════════════════════════════════════════
# 21. API ENDPOINTS — TIMETABLE
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/timetable")
def get_timetable(raw):
    tt = app.tenants.get_data("bright_future", "timetable", {})
    return ok({"status": "ok", "timetable": tt})

# ═══════════════════════════════════════════════════════════
# 22. API ENDPOINTS — ANALYTICS & REPORTS
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/analytics/dashboard")
def analytics_dashboard(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "students"):
        return forbidden()
    students_list = app.orm.query("Student").all()
    teachers_list = app.orm.query("Teacher").all()
    courses_list = app.orm.query("Course").all()
    fees_list = app.orm.query("Fee").all()
    paid_fees = [f for f in fees_list if f.get("status") == "paid"]
    pending_fees = [f for f in fees_list if f.get("status") == "pending"]
    total_revenue = sum(f.get("amount", 0) for f in paid_fees)
    pending_revenue = sum(f.get("amount", 0) for f in pending_fees)
    ab_assignment = app.ab.assign("dashboard_layout", req["headers"].get("X-User", "guest"))
    top_features = app.usage.top_features(5)
    return ok({
        "status": "ok", "school": SCHOOL_NAME,
        "academic_year": app.tenants.get_config("bright_future", "academic_year"),
        "term": app.tenants.get_config("bright_future", "term"),
        "stats": {
            "total_students": len(students_list),
            "total_teachers": len(teachers_list),
            "total_courses": len(courses_list),
            "total_fees": len(fees_list),
            "paid_fees": len(paid_fees),
            "pending_fees": len(pending_fees),
            "total_revenue": total_revenue,
            "pending_revenue": pending_revenue,
        },
        "ab_variant": ab_assignment.get("variant", "classic"),
        "top_features": top_features,
        "audit_count": app.audit.count,
    })

@app.api.endpoint("/api/analytics/performance")
def performance_analytics(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "grades"):
        return forbidden()
    students_list = app.orm.query("Student").all()
    report = []
    for s in students_list:
        grades = app.orm.query("Grade").filter(student_id=s["student_id"]).all()
        total_score = sum(g.get("score", 0) for g in grades)
        total_max = sum(g.get("max_score", 100) for g in grades)
        pct = (total_score / total_max * 100) if total_max > 0 else 0
        report.append({
            "student_id": s["student_id"], "name": s["name"],
            "grades_count": len(grades),
            "overall_percentage": round(pct, 2),
            "grade": "A" if pct >= 80 else "B" if pct >= 60 else "C" if pct >= 40 else "D",
        })
    return ok({"status": "ok", "report": report})

@app.api.endpoint("/api/analytics/attendance")
def attendance_analytics(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "attendance"):
        return forbidden()
    students_list = app.orm.query("Student").all()
    report = []
    for s in students_list:
        records = app.timeseries.query(f"attendance_{s['student_id']}")
        present = sum(1 for r in records if r["value"] == 1.0)
        total = len(records)
        rate = (present / total * 100) if total > 0 else 0
        report.append({"student_id": s["student_id"], "name": s["name"],
                       "present": present, "absent": total - present, "rate": round(rate, 2)})
    return ok({"status": "ok", "report": report})

# ═══════════════════════════════════════════════════════════
# 23. API ENDPOINTS — AI ASSISTANT
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/ai/ask")
def ai_assistant(raw):
    req = make_req(raw)
    data = req["json"]
    question = data.get("question", "")
    result = app.rag.query(question, top_k=3)
    app.usage.track("ai_ask", req["headers"].get("X-User", ""))
    return ok({"status": "ok", "question": question,
               "answer": result.get("answer", ""), "sources": len(result.get("sources", [])),
               "method": result.get("method", "retrieval-only")})

# ═══════════════════════════════════════════════════════════
# 24. API ENDPOINTS — AUDIT TRAIL
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/audit")
def audit_log(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "audit"):
        return forbidden()
    data = req["json"]
    entries = app.audit.query(user=data.get("user", ""),
                              action=data.get("action", ""),
                              limit=data.get("limit", 50))
    return ok({"status": "ok", "count": len(entries), "entries": entries})

# ═══════════════════════════════════════════════════════════
# 25. API ENDPOINTS — VERSION CONTROL
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/versions")
def versions(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "audit"):
        return forbidden()
    history = app.versions.history("school_data")
    return ok({"status": "ok", "history": history})

# ═══════════════════════════════════════════════════════════
# 26. API ENDPOINTS — SECURITY SCAN
# ═══════════════════════════════════════════════════════════

@app.api.endpoint("/api/security/scan")
def security_scan(raw):
    req = make_req(raw)
    if not check_perm(req, "read", "audit"):
        return forbidden()
    data = req["json"]
    result = app.security_scanner.scan(data.get("input", ""))
    app.audit.log(req["headers"].get("X-User", ""), "scan", "security", None, result)
    return ok(result)

# ═══════════════════════════════════════════════════════════
# 27. SCHEDULED TASKS (Cron)
# ═══════════════════════════════════════════════════════════

def send_fee_reminders():
    pending = app.orm.query("Fee").filter(status="pending").all()
    for fee in pending:
        student = app.orm.query("Student").filter(student_id=fee["student_id"]).first()
        if student:
            assignment = app.ab.assign("fee_reminder", fee["student_id"])
            channel = assignment.get("variant", "email")
            if channel == "email":
                app.email.send(student.get("email", ""), "Fee Reminder",
                               f"Your fee of {fee['amount']} TZS is due on {fee['due_date']}")
            else:
                app.sms.send(student.get("parent_email", ""),
                             f"Fee reminder: {fee['amount']} TZS due {fee['due_date']}")
            app.audit.log("system", "remind", "fee", None, fee)

app.scheduler.every("1h", "fee_reminders", send_fee_reminders)

# ═══════════════════════════════════════════════════════════
# 28. DASHBOARD HTML (Web UI)
# ═══════════════════════════════════════════════════════════

DASHBOARD_HTML = r"""<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=5.0">
    <title>PyTreX School Management System</title>
    <style>
        *{margin:0;padding:0;box-sizing:border-box}
        :root{
            --bg:#0f172a;--card:#1e293b;--border:#334155;--primary:#3b82f6;
            --primary-d:#2563eb;--text:#e2e8f0;--muted:#94a3b8;--green:#34d399;
            --yellow:#fbbf24;--red:#f87171;--dark:#0f172a;
        }
        body{font-family:'Segoe UI',system-ui,-apple-system,sans-serif;background:var(--bg);color:var(--text);min-height:100vh}
        /* Header */
        .header{background:linear-gradient(135deg,#1e3a8a,#3b82f6);padding:15px 20px;text-align:center;position:sticky;top:0;z-index:100;box-shadow:0 2px 10px rgba(0,0,0,.3)}
        .header h1{color:#fff;font-size:clamp(18px,4vw,28px)}
        .header p{color:#93c5fd;margin-top:4px;font-size:clamp(11px,2.5vw,14px)}
        /* Layout */
        .container{max-width:1280px;margin:0 auto;padding:15px}
        /* Login Bar */
        .login-bar{background:var(--card);padding:12px 15px;border-radius:10px;margin-bottom:15px;display:flex;gap:10px;align-items:center;flex-wrap:wrap;border:1px solid var(--border)}
        .login-bar input{padding:8px 12px;border-radius:6px;border:1px solid var(--border);background:var(--dark);color:var(--text);font-size:14px;flex:1;min-width:100px}
        .login-bar input:focus{outline:none;border-color:var(--primary)}
        .login-bar button{padding:8px 20px;border-radius:6px;border:none;background:var(--primary);color:#fff;cursor:pointer;font-size:14px;transition:.2s;white-space:nowrap}
        .login-bar button:hover{background:var(--primary-d)}
        .login-bar .label{font-size:14px;color:var(--muted);white-space:nowrap}
        #loginStatus{font-size:13px;color:var(--muted)}
        /* Navigation */
        .nav-wrap{background:var(--card);border-radius:10px;padding:10px;margin-bottom:15px;border:1px solid var(--border);position:sticky;top:70px;z-index:99}
        .nav-toggle{display:none;background:none;border:none;color:var(--text);font-size:20px;cursor:pointer;padding:5px 10px}
        .nav{display:flex;gap:8px;flex-wrap:wrap}
        .nav a{padding:8px 16px;background:var(--dark);border-radius:8px;color:#93c5fd;text-decoration:none;border:1px solid var(--border);transition:.2s;cursor:pointer;font-size:14px;white-space:nowrap;user-select:none}
        .nav a:hover{background:var(--primary);color:#fff;border-color:var(--primary)}
        .nav a.active{background:var(--primary);color:#fff;border-color:var(--primary)}
        /* Stats Grid */
        .stats-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(200px,1fr));gap:12px;margin:15px 0}
        .stat-card{background:var(--card);border-radius:12px;padding:18px;border:1px solid var(--border);transition:.2s}
        .stat-card:hover{transform:translateY(-3px);border-color:var(--primary);box-shadow:0 4px 15px rgba(59,130,246,.2)}
        .stat-card .icon{font-size:28px;margin-bottom:8px}
        .stat-card .value{font-size:clamp(22px,5vw,32px);font-weight:bold;color:var(--primary)}
        .stat-card .label{color:var(--muted);font-size:13px;margin-top:4px}
        /* Sections */
        .section{background:var(--card);border-radius:12px;padding:18px;margin:15px 0;border:1px solid var(--border)}
        .section h2{color:var(--primary);margin-bottom:12px;font-size:clamp(16px,3vw,20px)}
        /* Tables */
        .table-wrap{overflow-x:auto;-webkit-overflow-scrolling:touch}
        table{width:100%;border-collapse:collapse;min-width:500px}
        th,td{padding:10px 12px;text-align:left;border-bottom:1px solid var(--border);font-size:14px}
        th{color:var(--primary);font-weight:600;position:sticky;top:0;background:var(--card)}
        tr:hover{background:rgba(59,130,246,.05)}
        /* Badges */
        .badge{padding:3px 10px;border-radius:20px;font-size:11px;font-weight:600;display:inline-block}
        .badge-paid{background:#064e3b;color:var(--green)}
        .badge-pending{background:#78350f;color:var(--yellow)}
        .badge-active{background:#1e3a8a;color:#93c5fd}
        .badge-A{background:#064e3b;color:var(--green)}
        .badge-B{background:#1e3a8a;color:#93c5fd}
        .badge-C{background:#78350f;color:var(--yellow)}
        .badge-D{background:#7f1d1d;color:var(--red)}
        /* Forms */
        .form-row{display:flex;gap:10px;flex-wrap:wrap;margin-bottom:12px;align-items:flex-end}
        .form-group{flex:1;min-width:150px}
        .form-group label{display:block;font-size:13px;color:var(--muted);margin-bottom:4px}
        .form-group input,.form-group select,.form-group textarea{
            width:100%;padding:8px 12px;border-radius:6px;border:1px solid var(--border);
            background:var(--dark);color:var(--text);font-size:14px
        }
        .form-group input:focus,.form-group select:focus,.form-group textarea:focus{outline:none;border-color:var(--primary)}
        .btn{padding:8px 20px;border-radius:6px;border:none;background:var(--primary);color:#fff;cursor:pointer;font-size:14px;transition:.2s;white-space:nowrap}
        .btn:hover{background:var(--primary-d)}
        .btn-green{background:#059669}.btn-green:hover{background:#047857}
        .btn-yellow{background:#d97706}.btn-yellow:hover{background:#b45309}
        .btn-sm{padding:5px 12px;font-size:12px}
        /* AI */
        .ai-box{display:flex;gap:10px;margin-top:10px;flex-wrap:wrap}
        .ai-box input{flex:1;min-width:200px;padding:10px 14px;border-radius:8px;border:1px solid var(--border);background:var(--dark);color:var(--text);font-size:14px}
        .ai-box input:focus{outline:none;border-color:var(--primary)}
        .ai-answer{background:var(--dark);padding:15px;border-radius:8px;margin-top:10px;border:1px solid var(--border);line-height:1.6}
        .ai-answer strong{color:var(--primary)}
        /* API List */
        .api-list{display:grid;grid-template-columns:repeat(auto-fill,minmax(280px,1fr));gap:8px}
        .api-item{background:var(--dark);padding:10px 14px;border-radius:8px;border:1px solid var(--border);font-family:monospace;font-size:12px;overflow-x:auto}
        .method{font-weight:bold;color:var(--green)}
        .path{color:#93c5fd}
        /* Loading */
        .loading{text-align:center;padding:40px;color:var(--muted);font-size:16px}
        .spinner{display:inline-block;width:30px;height:30px;border:3px solid var(--border);border-top-color:var(--primary);border-radius:50%;animation:spin 1s linear infinite;margin-bottom:10px}
        @keyframes spin{to{transform:rotate(360deg)}}
        /* Toast */
        .toast{position:fixed;bottom:20px;right:20px;padding:12px 20px;border-radius:8px;background:var(--card);border:1px solid var(--border);color:var(--text);font-size:14px;z-index:1000;animation:slideIn .3s ease;box-shadow:0 4px 15px rgba(0,0,0,.3)}
        .toast-ok{border-color:var(--green);color:var(--green)}
        .toast-err{border-color:var(--red);color:var(--red)}
        @keyframes slideIn{from{transform:translateX(100%);opacity:0}to{transform:translateX(0);opacity:1}}
        /* Footer */
        .footer{text-align:center;padding:20px;color:#64748b;margin-top:30px;font-size:13px}
        /* Timetable */
        .tt-day{color:#93c5fd;margin:12px 0 6px;font-size:15px;font-weight:600}
        /* Mobile */
        @media(max-width:768px){
            .nav-toggle{display:block}
            .nav{display:none;flex-direction:column;gap:6px}
            .nav.open{display:flex}
            .nav a{width:100%;text-align:center}
            .stats-grid{grid-template-columns:repeat(2,1fr)}
            .login-bar{flex-direction:column;align-items:stretch}
            .login-bar input{width:100%}
            .login-bar button{width:100%}
            .form-row{flex-direction:column}
            .form-group{width:100%}
        }
        @media(max-width:480px){
            .stats-grid{grid-template-columns:1fr}
            .stat-card{text-align:center}
            .stat-card .icon{font-size:24px}
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>PyTreX School Management System</h1>
        <p>Bright Future Academy | Academic Year 2026 | Term 1</p>
    </div>
    <div class="container">
        <!-- Login -->
        <div class="login-bar">
            <span class="label">Login:</span>
            <input type="text" id="username" value="admin" placeholder="Username">
            <input type="password" id="password" value="admin123" placeholder="Password">
            <button onclick="doLogin()">Login</button>
            <span id="loginStatus">Not logged in</span>
        </div>
        <!-- Navigation -->
        <div class="nav-wrap">
            <button class="nav-toggle" onclick="toggleNav()">&#9776; Menu</button>
            <div class="nav" id="navMenu">
                <a onclick="navTo('dashboard',loadDashboard)">Dashboard</a>
                <a onclick="navTo('students',loadStudents)">Students</a>
                <a onclick="navTo('teachers',loadTeachers)">Teachers</a>
                <a onclick="navTo('courses',loadCourses)">Courses</a>
                <a onclick="navTo('fees',loadFees)">Fees</a>
                <a onclick="navTo('library',loadLibrary)">Library</a>
                <a onclick="navTo('timetable',loadTimetable)">Timetable</a>
                <a onclick="navTo('performance',loadPerformance)">Performance</a>
                <a onclick="navTo('attendance',loadAttendance)">Attendance</a>
                <a onclick="navTo('ai',loadAI)">AI Assistant</a>
                <a onclick="navTo('messages',loadMessages)">Messages</a>
                <a onclick="navTo('audit',loadAudit)">Audit Log</a>
                <a onclick="navTo('apis',loadAPIs)">API List</a>
            </div>
        </div>
        <div id="content"><div class="loading"><div class="spinner"></div><br>Loading...</div></div>
    </div>
    <div class="footer">
        <p>Powered by PyTreX System Framework | 130 Features | 395 Tests</p>
        <p>Built with Rust + Python + Elixir | Cross-Platform</p>
    </div>
    <script>
        let token='',role='',currentUser='admin';

        // === API Helper ===
        async function api(path, body=null) {
            try {
                const opts = {
                    method: body ? 'POST' : 'GET',
                    headers: { 'X-User': currentUser, 'X-Role': role, 'Content-Type': 'application/json' }
                };
                if (body) opts.body = JSON.stringify(body);
                const res = await fetch(path, opts);
                const text = await res.text();
                try { return JSON.parse(text); } catch(e) { return {status:'error',message:text.substring(0,200)}; }
            } catch(e) { return {status:'error', message:e.message}; }
        }

        // === Toast ===
        function toast(msg, type='ok') {
            const t = document.createElement('div');
            t.className = 'toast toast-' + type;
            t.textContent = msg;
            document.body.appendChild(t);
            setTimeout(() => t.remove(), 3000);
        }

        // === Navigation ===
        function toggleNav() { document.getElementById('navMenu').classList.toggle('open'); }
        function navTo(id, fn) {
            document.querySelectorAll('.nav a').forEach(a => a.classList.remove('active'));
            event && event.target && event.target.classList.add('active');
            if (window.innerWidth <= 768) document.getElementById('navMenu').classList.remove('open');
            document.getElementById('content').innerHTML = '<div class="loading"><div class="spinner"></div><br>Loading...</div>';
            fn();
        }

        // === Login ===
        async function doLogin() {
            currentUser = document.getElementById('username').value;
            const d = await api('/api/auth/login', {username:currentUser, password:document.getElementById('password').value});
            if (d.status === 'ok') {
                token = d.token; role = d.role;
                document.getElementById('loginStatus').textContent = '✅ ' + d.username + ' (' + d.role + ')';
                document.getElementById('loginStatus').style.color = '#34d399';
                toast('Welcome ' + d.username + '!');
                loadDashboard();
            } else {
                document.getElementById('loginStatus').textContent = '❌ Login failed';
                document.getElementById('loginStatus').style.color = '#f87171';
                toast('Login failed', 'err');
            }
        }

        // === Dashboard ===
        async function loadDashboard() {
            const d = await api('/api/analytics/dashboard', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Error loading dashboard</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="stats-grid">
                    <div class="stat-card"><div class="icon">🎓</div><div class="value">${d.stats.total_students}</div><div class="label">Students</div></div>
                    <div class="stat-card"><div class="icon">👨‍🏫</div><div class="value">${d.stats.total_teachers}</div><div class="label">Teachers</div></div>
                    <div class="stat-card"><div class="icon">📚</div><div class="value">${d.stats.total_courses}</div><div class="label">Courses</div></div>
                    <div class="stat-card"><div class="icon">💰</div><div class="value">${(d.stats.total_revenue/1000000).toFixed(1)}M</div><div class="label">Revenue (TZS)</div></div>
                    <div class="stat-card"><div class="icon">⏳</div><div class="value">${(d.stats.pending_revenue/1000000).toFixed(1)}M</div><div class="label">Pending (TZS)</div></div>
                    <div class="stat-card"><div class="icon">📝</div><div class="value">${d.audit_count}</div><div class="label">Audit Entries</div></div>
                </div>
                <div class="section"><h2>Top Features Used</h2><div class="table-wrap"><table><tr><th>Feature</th><th>Count</th></tr>
                    ${d.top_features.length ? d.top_features.map(f => `<tr><td>${f.feature}</td><td>${f.count}</td></tr>`).join('') : '<tr><td colspan="2" style="color:#94a3b8;text-align:center">No data yet</td></tr>'}
                </table></div></div>`;
        }

        // === Students ===
        async function loadStudents() {
            const d = await api('/api/students', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Permission denied or error</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Students (${d.count})</h2>
                <div class="form-row" id="addStudentForm">
                    <div class="form-group"><label>Name</label><input type="text" id="stName" placeholder="Student name"></div>
                    <div class="form-group"><label>Email</label><input type="text" id="stEmail" placeholder="email@school.ac.tz"></div>
                    <div class="form-group"><label>Class</label><input type="text" id="stClass" placeholder="C1A" value="C1A"></div>
                    <div class="form-group"><label>Parent Email</label><input type="text" id="stParent" placeholder="parent@family.com"></div>
                    <button class="btn" onclick="addStudent()">+ Add Student</button>
                </div>
                <div class="table-wrap"><table><tr><th>ID</th><th>Name</th><th>Email</th><th>Class</th><th>Status</th></tr>
                ${d.students.map(s => `<tr><td>${s.student_id}</td><td>${s.name}</td><td>${s.email}</td><td>${s.class_id}</td><td><span class="badge badge-active">${s.status}</span></td></tr>`).join('')}
                </table></div></div>`;
        }
        async function addStudent() {
            const d = await api('/api/students/create', {
                name: document.getElementById('stName').value,
                email: document.getElementById('stEmail').value,
                class_id: document.getElementById('stClass').value,
                parent_email: document.getElementById('stParent').value,
                enrolled_date: '2026-01-08'
            });
            if (d.status === 'ok') { toast('Student added!'); loadStudents(); }
            else { toast(d.message || 'Error', 'err'); }
        }

        // === Teachers ===
        async function loadTeachers() {
            const d = await api('/api/teachers', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Permission denied</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Teachers (${d.count})</h2>
                <div class="form-row">
                    <div class="form-group"><label>Name</label><input type="text" id="tcName" placeholder="Teacher name"></div>
                    <div class="form-group"><label>Email</label><input type="text" id="tcEmail" placeholder="email@school.ac.tz"></div>
                    <div class="form-group"><label>Subject</label><input type="text" id="tcSubject" placeholder="Mathematics"></div>
                    <button class="btn" onclick="addTeacher()">+ Add Teacher</button>
                </div>
                <div class="table-wrap"><table><tr><th>ID</th><th>Name</th><th>Subject</th><th>Email</th></tr>
                ${d.teachers.map(t => `<tr><td>${t.teacher_id}</td><td>${t.name}</td><td>${t.subject}</td><td>${t.email}</td></tr>`).join('')}
                </table></div></div>`;
        }
        async function addTeacher() {
            const d = await api('/api/teachers/create', {
                name: document.getElementById('tcName').value,
                email: document.getElementById('tcEmail').value,
                subject: document.getElementById('tcSubject').value,
                hired_date: '2026-01-01'
            });
            if (d.status === 'ok') { toast('Teacher added!'); loadTeachers(); }
            else { toast(d.message || 'Error', 'err'); }
        }

        // === Courses ===
        async function loadCourses() {
            const d = await api('/api/courses', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Permission denied</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Courses (${d.count})</h2>
                <div class="form-row">
                    <div class="form-group"><label>Course ID</label><input type="text" id="crId" placeholder="ENG101"></div>
                    <div class="form-group"><label>Name</label><input type="text" id="crName" placeholder="English Form 1"></div>
                    <div class="form-group"><label>Teacher ID</label><input type="text" id="crTeacher" placeholder="T001"></div>
                    <div class="form-group"><label>Credits</label><input type="number" id="crCredits" value="3" min="1" max="10"></div>
                    <button class="btn" onclick="addCourse()">+ Add Course</button>
                </div>
                <div class="table-wrap"><table><tr><th>Course ID</th><th>Name</th><th>Teacher</th><th>Credits</th></tr>
                ${d.courses.map(c => `<tr><td>${c.course_id}</td><td>${c.name}</td><td>${c.teacher_id}</td><td>${c.credits}</td></tr>`).join('')}
                </table></div></div>`;
        }
        async function addCourse() {
            const d = await api('/api/courses/create', {
                course_id: document.getElementById('crId').value,
                name: document.getElementById('crName').value,
                teacher_id: document.getElementById('crTeacher').value,
                class_id: 'C1A',
                credits: parseInt(document.getElementById('crCredits').value) || 3
            });
            if (d.status === 'ok') { toast('Course added!'); loadCourses(); }
            else { toast(d.message || 'Error', 'err'); }
        }

        // === Fees ===
        async function loadFees() {
            const d = await api('/api/fees', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Permission denied</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Fees (${d.count})</h2>
                <div class="table-wrap"><table><tr><th>Fee ID</th><th>Student</th><th>Type</th><th>Amount</th><th>Status</th><th>Action</th></tr>
                ${d.fees.map(f => `<tr><td>${f.fee_id}</td><td>${f.student_id}</td><td>${f.fee_type}</td><td>${(f.amount/1000).toFixed(0)}K TZS</td><td><span class="badge badge-${f.status}">${f.status}</span></td><td>${f.status==='pending' ? `<button class="btn btn-green btn-sm" onclick="payFee('${f.fee_id}')">Pay</button>` : '✅'}</td></tr>`).join('')}
                </table></div></div>`;
        }
        async function payFee(feeId) {
            const d = await api('/api/fees/pay', {fee_id: feeId, method: 'mpesa'});
            if (d.status === 'ok') { toast('Payment successful!'); loadFees(); }
            else { toast(d.message || 'Payment failed', 'err'); }
        }

        // === Library ===
        async function loadLibrary() {
            const d = await api('/api/library', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Error loading library</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Library (${d.available}/${d.total} available)</h2>
                <div class="table-wrap"><table><tr><th>ID</th><th>Title</th><th>Author</th><th>Category</th><th>Status</th><th>Action</th></tr>
                ${d.items.map(i => `<tr><td>${i.item_id}</td><td>${i.title}</td><td>${i.author}</td><td>${i.category}</td><td>${i.available ? '✅ Available' : '❌ Borrowed'}</td><td>${i.available ? `<button class="btn btn-sm" onclick="borrowBook('${i.item_id}')">Borrow</button>` : `<button class="btn btn-yellow btn-sm" onclick="returnBook('${i.item_id}')">Return</button>`}</td></tr>`).join('')}
                </table></div></div>`;
        }
        async function borrowBook(itemId) {
            const d = await api('/api/library/borrow', {item_id: itemId, student_id: 'S001', due_date: '2026-03-01'});
            if (d.status === 'ok') { toast('Book borrowed!'); loadLibrary(); }
            else { toast(d.message || 'Error', 'err'); }
        }
        async function returnBook(itemId) {
            const d = await api('/api/library/return', {item_id: itemId, student_id: 'S001'});
            if (d.status === 'ok') { toast('Book returned!'); loadLibrary(); }
            else { toast(d.message || 'Error', 'err'); }
        }

        // === Timetable ===
        async function loadTimetable() {
            const d = await api('/api/timetable', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Error</p></div>'; return; }
            let html = '<div class="section"><h2>Weekly Timetable</h2>';
            for (const [day, classes] of Object.entries(d.timetable)) {
                html += `<div class="tt-day">${day}</div><div class="table-wrap"><table><tr><th>Time</th><th>Course</th><th>Teacher</th></tr>`;
                for (const c of classes) { html += `<tr><td>${c[0]}</td><td>${c[1]}</td><td>${c[2]}</td></tr>`; }
                html += '</table></div>';
            }
            document.getElementById('content').innerHTML = html + '</div>';
        }

        // === Performance ===
        async function loadPerformance() {
            const d = await api('/api/analytics/performance', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Permission denied</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Student Performance</h2>
                <div class="table-wrap"><table><tr><th>Student</th><th>Name</th><th>Grades</th><th>%</th><th>Grade</th></tr>
                ${d.report.map(r => `<tr><td>${r.student_id}</td><td>${r.name}</td><td>${r.grades_count}</td><td>${r.overall_percentage}%</td><td><span class="badge badge-${r.grade}">${r.grade}</span></td></tr>`).join('')}
                </table></div></div>
                <div class="section"><h2>Record New Grade</h2>
                <div class="form-row">
                    <div class="form-group"><label>Student ID</label><input type="text" id="grStudent" placeholder="S001" value="S001"></div>
                    <div class="form-group"><label>Course ID</label><input type="text" id="grCourse" placeholder="MATH101" value="MATH101"></div>
                    <div class="form-group"><label>Exam Type</label><select id="grType"><option>test</option><option>midterm</option><option>final</option></select></div>
                    <div class="form-group"><label>Score</label><input type="number" id="grScore" placeholder="85" min="0" max="100"></div>
                    <div class="form-group"><label>Max Score</label><input type="number" id="grMax" value="100" min="1"></div>
                    <button class="btn" onclick="recordGrade()">Record</button>
                </div></div>`;
        }
        async function recordGrade() {
            const d = await api('/api/grades/record', {
                student_id: document.getElementById('grStudent').value,
                course_id: document.getElementById('grCourse').value,
                exam_type: document.getElementById('grType').value,
                score: parseFloat(document.getElementById('grScore').value) || 0,
                max_score: parseFloat(document.getElementById('grMax').value) || 100
            });
            if (d.status === 'ok') { toast('Grade recorded!'); loadPerformance(); }
            else { toast(d.message || 'Error', 'err'); }
        }

        // === Attendance ===
        async function loadAttendance() {
            const d = await api('/api/analytics/attendance', {});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Permission denied</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Attendance Report</h2>
                <div class="table-wrap"><table><tr><th>Student</th><th>Name</th><th>Present</th><th>Absent</th><th>Rate</th></tr>
                ${d.report.map(r => `<tr><td>${r.student_id}</td><td>${r.name}</td><td>${r.present}</td><td>${r.absent}</td><td>${r.rate}%</td></tr>`).join('')}
                </table></div></div>
                <div class="section"><h2>Record Attendance</h2>
                <div class="form-row">
                    <div class="form-group"><label>Student ID</label><input type="text" id="atStudent" placeholder="S001" value="S001"></div>
                    <div class="form-group"><label>Course ID</label><input type="text" id="atCourse" placeholder="MATH101" value="MATH101"></div>
                    <div class="form-group"><label>Present?</label><select id="atPresent"><option value="true">Present ✅</option><option value="false">Absent ❌</option></select></div>
                    <button class="btn" onclick="recordAttendance()">Record</button>
                </div></div>`;
        }
        async function recordAttendance() {
            const d = await api('/api/attendance/record', {
                student_id: document.getElementById('atStudent').value,
                course_id: document.getElementById('atCourse').value,
                present: document.getElementById('atPresent').value === 'true'
            });
            if (d.status === 'ok') { toast('Attendance recorded!'); loadAttendance(); }
            else { toast(d.message || 'Error', 'err'); }
        }

        // === AI Assistant ===
        async function loadAI() {
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>🤖 AI Assistant</h2>
                <p style="color:#94a3b8;margin-bottom:10px">Ask any question about the school:</p>
                <div class="ai-box"><input type="text" id="aiQuestion" placeholder="e.g. What does Mathematics Form 1 cover?" onkeydown="if(event.key==='Enter')askAI()">
                <button class="btn" onclick="askAI()">Ask 🤖</button></div>
                <div id="aiResult"></div>
                <div style="margin-top:15px">
                    <p style="color:#94a3b8;font-size:13px;margin-bottom:8px">Quick questions:</p>
                    <button class="btn btn-sm" onclick="quickAI('What does Mathematics Form 1 cover?')">Math topics?</button>
                    <button class="btn btn-sm" onclick="quickAI('What are the school fees?')">Fees?</button>
                    <button class="btn btn-sm" onclick="quickAI('When is the library open?')">Library hours?</button>
                    <button class="btn btn-sm" onclick="quickAI('How are grades calculated?')">Grading?</button>
                </div></div>`;
        }
        function quickAI(q) { document.getElementById('aiQuestion').value = q; askAI(); }
        async function askAI() {
            const q = document.getElementById('aiQuestion').value;
            if (!q) return;
            document.getElementById('aiResult').innerHTML = '<div class="loading"><div class="spinner"></div><br>AI thinking...</div>';
            const d = await api('/api/ai/ask', {question: q});
            if (d.status === 'ok') {
                document.getElementById('aiResult').innerHTML = `<div class="ai-answer"><strong>Q:</strong> ${q}<br><strong>A:</strong> ${d.answer}<br><small style="color:#64748b">Sources: ${d.sources} | Method: ${d.method}</small></div>`;
            } else {
                document.getElementById('aiResult').innerHTML = `<div class="ai-answer" style="border-color:#f87171"><strong>Error:</strong> ${d.message || 'Failed'}</div>`;
            }
        }

        // === Messages ===
        async function loadMessages() {
            const d = await api('/api/messages/inbox', {});
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Send Message</h2>
                <div class="form-row">
                    <div class="form-group"><label>To</label><input type="text" id="msgTo" placeholder="student1"></div>
                    <div class="form-group"><label>Subject</label><input type="text" id="msgSubject" placeholder="Subject"></div>
                </div>
                <div class="form-row">
                    <div class="form-group" style="flex:1"><label>Message</label><textarea id="msgBody" rows="3" placeholder="Type your message..." style="width:100%;resize:vertical"></textarea></div>
                </div>
                <button class="btn" onclick="sendMessage()">Send</button></div>
                <div class="section"><h2>Inbox (${d.total || 0}, ${d.unread || 0} unread)</h2>
                <div class="table-wrap"><table><tr><th>From</th><th>Subject</th><th>Date</th></tr>
                ${(d.messages || []).map(m => `<tr><td>${m.sender}</td><td>${m.subject}</td><td>${m.sent_date}</td></tr>`).join('') || '<tr><td colspan="3" style="color:#94a3b8;text-align:center">No messages</td></tr>'}
                </table></div></div>`;
        }
        async function sendMessage() {
            const d = await api('/api/messages/send', {
                recipient: document.getElementById('msgTo').value,
                subject: document.getElementById('msgSubject').value,
                body: document.getElementById('msgBody').value
            });
            if (d.status === 'ok') { toast('Message sent!'); loadMessages(); }
            else { toast(d.message || 'Error', 'err'); }
        }

        // === Audit ===
        async function loadAudit() {
            const d = await api('/api/audit', {limit: 20});
            if (d.status !== 'ok') { document.getElementById('content').innerHTML = '<div class="section"><p>Permission denied</p></div>'; return; }
            document.getElementById('content').innerHTML = `
                <div class="section"><h2>Audit Log (${d.count} entries)</h2>
                <div class="table-wrap"><table><tr><th>#</th><th>User</th><th>Action</th><th>Resource</th><th>Time</th></tr>
                ${d.entries.map(e => `<tr><td>${e.id}</td><td>${e.user}</td><td>${e.action}</td><td>${e.resource}</td><td>${e.timestamp.substring(11,19)}</td></tr>`).join('')}
                </table></div></div>`;
        }

        // === API List ===
        function loadAPIs() {
            const apis = [
                ['POST','/api/auth/login'],['POST','/api/auth/logout'],
                ['POST','/api/students'],['POST','/api/students/create'],['POST','/api/students/detail'],
                ['POST','/api/teachers'],['POST','/api/teachers/create'],
                ['POST','/api/courses'],['POST','/api/courses/create'],
                ['POST','/api/grades/record'],['POST','/api/grades/view'],
                ['POST','/api/attendance/record'],['POST','/api/attendance/view'],
                ['POST','/api/assignments/create'],['POST','/api/assignments/submit'],['POST','/api/assignments/grade'],
                ['POST','/api/fees'],['POST','/api/fees/pay'],
                ['POST','/api/library'],['POST','/api/library/borrow'],['POST','/api/library/return'],
                ['POST','/api/messages/send'],['POST','/api/messages/inbox'],
                ['POST','/api/timetable'],
                ['POST','/api/analytics/dashboard'],['POST','/api/analytics/performance'],['POST','/api/analytics/attendance'],
                ['POST','/api/ai/ask'],['POST','/api/audit'],['POST','/api/versions'],['POST','/api/security/scan'],
            ];
            document.getElementById('content').innerHTML = `<div class="section"><h2>API Endpoints (${apis.length})</h2>
                <div class="api-list">${apis.map(([m,p]) => `<div class="api-item"><span class="method">${m}</span> <span class="path">${p}</span></div>`).join('')}</div></div>`;
        }

        // === Init ===
        doLogin();
    </script>
</body>
</html>"""

@app.api.endpoint("/")
def dashboard(raw):
    return {"status": 200, "body": DASHBOARD_HTML, "headers": {"Content-Type": "text/html"}}

# ═══════════════════════════════════════════════════════════
# 29. START THE SYSTEM
# ═══════════════════════════════════════════════════════════

if __name__ == "__main__":
    print("\n" + "=" * 60)
    print(f"  {SCHOOL_NAME} — School Management System")
    print("=" * 60)
    print(f"  Features: 130 PyTreX modules loaded")
    print(f"  Students: {app.orm.count('Student')}")
    print(f"  Teachers: {app.orm.count('Teacher')}")
    print(f"  Courses:  {app.orm.count('Course')}")
    print(f"  Library:  {app.orm.count('LibraryItem')} books")
    print(f"  Fees:     {app.orm.count('Fee')} records")
    print(f"  API Endpoints: {len(app.api._endpoints)}")
    print(f"  AI Assistant: {app.rag.document_count} documents loaded")
    print(f"  Audit Trail: Active ({app.audit.count} entries)")
    print(f"  Multi-tenant: {app.tenants.tenant_count} schools")
    print(f"  A/B Tests: {len(app.ab._experiments)} experiments")
    print("=" * 60)
    print(f"\n  Dashboard: http://localhost:8080")
    print(f"  API Base:  http://localhost:8080/api")
    print(f"\n  Login: admin / admin123")
    print(f"         teacher1 / teach123")
    print(f"         student1 / stud123")
    print(f"         parent1 / parent123")
    print("\n  Press Ctrl+C to stop.\n")

    app.api.port = 8080
    app.api.start()
    try:
        while True:
            time.sleep(1)
    except KeyboardInterrupt:
        print("\nShutting down...")
        app.api.stop()

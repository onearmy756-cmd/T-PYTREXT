"""Test the school management system endpoints."""
import sys
sys.path.insert(0, "c:/Pytrex/examples")
import school_management as sm
import json

def call(fn, body_dict=None, headers=None):
    """Call an endpoint with a dict body (converted to JSON string)."""
    body = json.dumps(body_dict or {})
    sm.set_headers(headers or {"X-User": "admin"})
    r = fn(body)
    sm.set_headers({})
    return json.loads(r["body"])

# 1. Login
d = call(sm.login, {"username": "admin", "password": "admin123"})
print(f"1. Login: {d['status']} - {d.get('username')} ({d.get('role')})")
assert d["status"] == "ok"

# 2. List students
d = call(sm.students)
print(f"2. Students: {d['count']}")
assert d["count"] == 2

# 3. List teachers
d = call(sm.teachers)
print(f"3. Teachers: {d['count']}")
assert d["count"] == 2

# 4. List courses
d = call(sm.courses)
print(f"4. Courses: {d['count']}")
assert d["count"] == 2

# 5. Dashboard
d = call(sm.analytics_dashboard)
print(f"5. Dashboard: students={d['stats']['total_students']}, revenue={d['stats']['total_revenue']}")
assert d["stats"]["total_students"] == 2

# 6. Library
d = call(sm.library)
print(f"6. Library: {d['total']} books, {d['available']} available")
assert d["total"] == 2

# 7. Fees
d = call(sm.fees)
print(f"7. Fees: {d['count']} records")
assert d["count"] == 2

# 8. Timetable
d = call(sm.get_timetable)
print(f"8. Timetable: {len(d['timetable'])} days")
assert "Monday" in d["timetable"]

# 9. Record grade
d = call(sm.record_grade, {"student_id": "S001", "course_id": "MATH101", "exam_type": "midterm", "score": 85, "max_score": 100})
print(f"9. Record grade: {d['status']}")
assert d["status"] == "ok"

# 10. View grades
d = call(sm.view_grades, {"student_id": "S001"})
print(f"10. Grades for S001: {d['count']} entries, {d['overall_percentage']}%")
assert d["count"] == 1

# 11. Record attendance
d = call(sm.record_attendance, {"student_id": "S001", "present": True, "course_id": "MATH101"})
print(f"11. Attendance: {d['status']}")
assert d["status"] == "ok"

# 12. View attendance
d = call(sm.view_attendance, {"student_id": "S001"})
print(f"12. Attendance for S001: {d['present_days']}/{d['total_days']} ({d['attendance_rate']}%)")
assert d["total_days"] == 1

# 13. Create assignment
d = call(sm.create_assignment, {"course_id": "MATH101", "title": "Algebra HW", "description": "Solve 10 problems", "due_date": "2026-03-01", "max_marks": 50})
print(f"13. Create assignment: {d['status']}")
assert d["status"] == "ok"

# 14. Submit assignment
d = call(sm.submit_assignment, {"assignment_id": "A1", "student_id": "S001", "content": "Here are my answers for the algebra homework."})
print(f"14. Submit assignment: {d['status']}")
assert d["status"] == "ok"

# 15. Grade submission
subs = sm.app.orm.query("Submission").all()
sub_id = subs[0]["submission_id"]
d = call(sm.grade_submission, {"submission_id": sub_id, "marks": 45})
print(f"15. Grade submission: {d['status']}")
assert d["status"] == "ok"

# 16. Performance analytics
d = call(sm.performance_analytics)
print(f"16. Performance: {len(d['report'])} students")
assert len(d["report"]) == 2

# 17. AI Assistant
d = call(sm.ai_assistant, {"question": "What does Mathematics Form 1 cover?"}, {"X-User": "student1"})
print(f"17. AI Answer: {d['answer'][:60]}...")
assert d["status"] == "ok"

# 18. Send message
d = call(sm.send_message, {"recipient": "student1", "subject": "Hello", "body": "Welcome to school!"}, {"X-User": "teacher1"})
print(f"18. Send message: {d['status']}")
assert d["status"] == "ok"

# 19. Inbox
d = call(sm.inbox, {}, {"X-User": "student1"})
print(f"19. Inbox: {d['total']} messages, {d['unread']} unread")
assert d["total"] == 1

# 20. Borrow book
d = call(sm.borrow_book, {"item_id": "B001", "student_id": "S001"})
print(f"20. Borrow book: {d['status']}")
assert d["status"] == "ok"

# 21. Return book
d = call(sm.return_book, {"item_id": "B001", "student_id": "S001"})
print(f"21. Return book: {d['status']}")
assert d["status"] == "ok"

# 22. Pay fee
d = call(sm.pay_fee, {"fee_id": "F001", "method": "mpesa"})
print(f"22. Pay fee: {d['status']}")
assert d["status"] == "ok"

# 23. Audit log
d = call(sm.audit_log, {"limit": 10})
print(f"23. Audit: {d['count']} entries")
assert d["count"] > 0

# 24. Security scan
d = call(sm.security_scan, {"input": "SELECT * FROM users; DROP TABLE users--"})
print(f"24. Security scan: secure={d['secure']}, vulns={len(d['vulnerabilities'])}")
assert d["secure"] is False

# 25. Versions
d = call(sm.versions)
print(f"26. Versions: {len(d['history'])} commits")
assert len(d["history"]) >= 1

# 26. Permission check - student trying to record grade
d = call(sm.record_grade, {"student_id": "S001", "course_id": "MATH101", "score": 100, "max_score": 100}, {"X-User": "student1"})
print(f"25. Student denied grade recording: status={d.get('status', 'error')}")
assert d.get("status") == "error" or d.get("message") == "No permission"

print("\n" + "=" * 50)
print("ALL 26 API TESTS PASSED!")
print("=" * 50)
print(f"Endpoints: {len(sm.app.api._endpoints)}")
print(f"Students: {sm.app.orm.count('Student')}")
print(f"Teachers: {sm.app.orm.count('Teacher')}")
print(f"Courses: {sm.app.orm.count('Course')}")
print(f"AI docs: {sm.app.rag.document_count}")
print(f"Audit entries: {sm.app.audit.count}")

import { useSearchParams } from "react-router";
import { ExamStudentPage } from "#/components/features/education/exam-student-page";

/**
 * `/ide/mtihani?exam=CODE` — ukurasa wa mwanafunzi (hufunguliwa kwa QR code).
 */
export default function MtihaniRoute() {
  const [params] = useSearchParams();
  const code = (params.get("exam") ?? params.get("code") ?? "").trim().toUpperCase();
  if (!code) {
    return (
      <div className="mx-auto max-w-lg p-6 text-sm text-muted">
        Changanua QR code ya mtihani au ongeza <code>?exam=CODE</code> kwenye
        anwani.
      </div>
    );
  }
  return <ExamStudentPage code={code} />;
}

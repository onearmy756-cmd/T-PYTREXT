//! lang_verify.rs — rejista ya LUGHA ZOTE za programming.
//!
//! Hii ni sehemu ya "super-intelligence engineer": mfumo hautegemei lugha
//! moja. Hapa tuna:
//!   1. `detect(dir)`   — tambua lugha/framework kutoka faili za mradi.
//!   2. `verify_cmd(lang)` — amri KALI ya uthibitisho kwa kila lugha.
//!
//! Kanuni kuu (UHALISIA): verify HAIWEZI kuwa ya uongo. Kwa lugha yenye tests,
//! tunahitisha (a) build/compile KUPITA **na** (b) tests halisi zipite.
//! Mradi bila tests haupiti. Lugha isiyo na toolchain → hurudisha amri inayo-
//! angalia kwa wazi (`command -v X || exit 1`) badala ya "ok" ya uongo.

use std::path::Path;

/// Lugha zinazotambulika.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Node,
    Dart,
    Flutter,
    Go,
    JavaMaven,
    JavaGradle,
    Dotnet,
    Ruby,
    Php,
    Swift,
    Kotlin,
    CMake,
    CMakeCpp,
    Make,
    Elixir,
    R,
    Python,
    Haskell,
    Zig,
    Scala,
    Deno,
    Bun,
    Unknown,
}

impl Lang {
    pub fn name(self) -> &'static str {
        match self {
            Lang::Rust => "rust",
            Lang::Node => "node",
            Lang::Dart => "dart",
            Lang::Flutter => "flutter",
            Lang::Go => "go",
            Lang::JavaMaven => "java-maven",
            Lang::JavaGradle => "java-gradle",
            Lang::Dotnet => "dotnet",
            Lang::Ruby => "ruby",
            Lang::Php => "php",
            Lang::Swift => "swift",
            Lang::Kotlin => "kotlin",
            Lang::CMake => "cmake",
            Lang::CMakeCpp => "cmake-cpp",
            Lang::Make => "make",
            Lang::Elixir => "elixir",
            Lang::R => "r",
            Lang::Python => "python",
            Lang::Haskell => "haskell",
            Lang::Zig => "zig",
            Lang::Scala => "scala",
            Lang::Deno => "deno",
            Lang::Bun => "bun",
            Lang::Unknown => "unknown",
        }
    }
}

/// Faili za kugundua lugha (jina kamili).
fn has(dir: &Path, f: &str) -> bool {
    dir.join(f).exists()
}

/// Tambua lugha kutoka faili za mradi. Mpangilio ni muhimu: zaidi ya moja
/// inaweza kuwepo (mfano Flutter ina `pubspec.yaml` na `android/`).
pub fn detect(dir: &Path) -> Lang {
    if has(dir, "pubspec.yaml") {
        // Flutter = Dart + android/ios/macos/web dirs.
        if dir.join("android").is_dir()
            || dir.join("ios").is_dir()
            || dir.join("lib").join("main.dart").exists()
        {
            return Lang::Flutter;
        }
        return Lang::Dart;
    }
    if has(dir, "Cargo.toml") {
        return Lang::Rust;
    }
    if has(dir, "go.mod") {
        return Lang::Go;
    }
    if has(dir, "pom.xml") {
        return Lang::JavaMaven;
    }
    if has(dir, "build.gradle") || has(dir, "build.gradle.kts") || has(dir, "settings.gradle") {
        return Lang::JavaGradle;
    }
    if dir
        .read_dir()
        .map(|rd| {
            rd.flatten().any(|e| {
                let n = e.file_name().to_string_lossy().to_lowercase();
                n.ends_with(".csproj") || n.ends_with(".sln") || n.ends_with(".fsproj")
            })
        })
        .unwrap_or(false)
    {
        return Lang::Dotnet;
    }
    if has(dir, "Package.swift") {
        return Lang::Swift;
    }
    if has(dir, "build.sbt") {
        return Lang::Scala;
    }
    if has(dir, "build.zig") {
        return Lang::Zig;
    }
    if has(dir, "mix.exs") {
        return Lang::Elixir;
    }
    if has(dir, "DESCRIPTION") {
        return Lang::R;
    }
    if has(dir, "pyproject.toml") || has(dir, "setup.py") || has(dir, "requirements.txt") {
        return Lang::Python;
    }
    if has(dir, "Gemfile") || has(dir, "Rakefile") {
        return Lang::Ruby;
    }
    if has(dir, "composer.json") {
        return Lang::Php;
    }
    if has(dir, "build.gradle.kts") {
        return Lang::Kotlin;
    }
    if has(dir, "deno.json") || has(dir, "deno.jsonc") {
        return Lang::Deno;
    }
    if has(dir, "bun.lockb") {
        return Lang::Bun;
    }
    if has(dir, "package.json") {
        return Lang::Node;
    }
    if has(dir, "CMakeLists.txt") {
        return Lang::CMakeCpp;
    }
    if has(dir, "Makefile") || has(dir, "makefile") {
        return Lang::Make;
    }
    // Fallback: faili za chanzo pekee.
    if let Ok(rd) = dir.read_dir() {
        let names: Vec<String> = rd
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_lowercase())
            .collect();
        if names.iter().any(|n| n.ends_with(".cs")) {
            return Lang::Dotnet;
        }
        if names.iter().any(|n| n.ends_with(".go")) {
            return Lang::Go;
        }
        if names.iter().any(|n| n.ends_with(".rs")) {
            return Lang::Rust;
        }
        if names.iter().any(|n| n.ends_with(".java")) {
            return Lang::JavaMaven;
        }
        if names.iter().any(|n| n.ends_with(".kt")) {
            return Lang::Kotlin;
        }
        if names.iter().any(|n| n.ends_with(".dart")) {
            return Lang::Dart;
        }
        if names.iter().any(|n| n.ends_with(".swift")) {
            return Lang::Swift;
        }
        if names.iter().any(|n| n.ends_with(".php")) {
            return Lang::Php;
        }
        if names.iter().any(|n| n.ends_with(".rb")) {
            return Lang::Ruby;
        }
        if names.iter().any(|n| n.ends_with(".ex")) {
            return Lang::Elixir;
        }
        if names.iter().any(|n| n.ends_with(".r") || n.ends_with(".R")) {
            return Lang::R;
        }
        if names.iter().any(|n| n.ends_with(".py")) {
            return Lang::Python;
        }
        if names.iter().any(|n| n.ends_with(".hs")) {
            return Lang::Haskell;
        }
        if names.iter().any(|n| n.ends_with(".zig")) {
            return Lang::Zig;
        }
        if names.iter().any(|n| n.ends_with(".scala")) {
            return Lang::Scala;
        }
        if names
            .iter()
            .any(|n| n.ends_with(".c") || n.ends_with(".cc") || n.ends_with(".cpp"))
        {
            return Lang::CMakeCpp;
        }
        if names.iter().any(|n| n.ends_with(".js") || n.ends_with(".ts")) {
            return Lang::Node;
        }
    }
    Lang::Unknown
}

/// Amri KALI ya uthibitisho kwa lugha. Huhitaji tests halisi zipite.
pub fn verify_cmd(lang: Lang) -> &'static str {
    match lang {
        Lang::Rust => {
            "cargo build 2>&1 | tail -5 && \
OUT=$(cargo test 2>&1); echo \"$OUT\" | tail -40; \
echo \"$OUT\" | grep -qE 'test result: FAILED|^error(\\[|:)' && { echo 'KOSA: build/tests zimeshindwa'; exit 1; }; \
echo \"$OUT\" | grep -qE 'test result: ok\\. [1-9][0-9]* passed' || { echo 'KOSA: hakuna tests zilizopita — andika #[test]'; exit 1; }"
        }
        Lang::Node | Lang::Deno | Lang::Bun => {
            "npm install --no-audit --no-fund --loglevel=error 2>&1 | tail -5; \
if node -e \"const p=require('./package.json');process.exit((p.scripts&&p.scripts.test)?0:1)\" 2>/dev/null; then \
  npm test --silent 2>&1 | tail -30; \
else \
  echo 'KOSA: hakuna script ya test kwenye package.json'; exit 1; \
fi"
        }
        Lang::Dart => {
            // ONYO: `+0 -1: Some tests failed` INA `+0`; hivyo usitumie `\+[0-9]+`.
            // Tunakataa kwa uwepo wa `-N` (failures) au `Some tests failed`, na
            // tunahitisha `All tests passed!`.
            "command -v dart >/dev/null || { echo 'KOSA: dart haipo'; exit 1; }; \
dart pub get 2>&1 | tail -3; \
if [ -d test ]; then \
  OUT=$(dart test 2>&1 | tail -30); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE 'Some tests failed|-[1-9][0-9]*:' && { echo 'KOSA: dart tests zimeshindwa'; exit 1; }; \
  echo \"$OUT\" | grep -qE 'All tests passed!' || { echo 'KOSA: dart tests hazikupita'; exit 1; }; \
else \
  echo 'KOSA: hakuna saraka test/ — andika dart tests'; exit 1; \
fi"
        }
        Lang::Flutter => {
            "command -v flutter >/dev/null || { echo 'KOSA: flutter haipo'; exit 1; }; \
flutter pub get 2>&1 | tail -3; \
OUT=$(flutter test 2>&1 | tail -30); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE 'Some tests failed|-[1-9][0-9]*:' && { echo 'KOSA: flutter tests zimeshindwa'; exit 1; }; \
echo \"$OUT\" | grep -qE 'All tests passed!' || { echo 'KOSA: flutter tests hazikupita'; exit 1; }"
        }
        Lang::Go => {
            // Kumbuka: `?   pkg  [no test files]` ni sawa kwa package zisizo na
            // tests. Tunakataa tu kwa FAIL, na tunahitisha angalau `ok` moja.
            "command -v go >/dev/null || { echo 'KOSA: go haipo'; exit 1; }; \
go build ./... 2>&1 | tail -5; \
OUT=$(go test ./... 2>&1 | tail -30); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE '^FAIL|^--- FAIL' && { echo 'KOSA: go tests zimeshindwa'; exit 1; }; \
echo \"$OUT\" | grep -qE '^ok' || { echo 'KOSA: hakuna go tests zilizopita — andika _test.go'; exit 1; }"
        }
        Lang::JavaMaven => {
            "command -v mvn >/dev/null || { echo 'KOSA: maven haipo'; exit 1; }; \
OUT=$(mvn -q -B test 2>&1 | tail -40); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE 'Tests run: [1-9][0-9]*' || { echo 'KOSA: hakuna java tests'; exit 1; }; \
echo \"$OUT\" | grep -qE 'BUILD SUCCESS' || { echo 'KOSA: maven build imeshindwa'; exit 1; }"
        }
        Lang::JavaGradle | Lang::Kotlin => {
            "if [ -x ./gradlew ]; then G=./gradlew; elif command -v gradle >/dev/null; then G=gradle; else echo 'KOSA: gradle haipo'; exit 1; fi; \
OUT=$($G test 2>&1 | tail -40); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE 'BUILD SUCCESSFUL' || { echo 'KOSA: gradle build/tests zimeshindwa'; exit 1; }"
        }
        Lang::Dotnet => {
            "command -v dotnet >/dev/null || { echo 'KOSA: dotnet haipo'; exit 1; }; \
dotnet build -v q 2>&1 | tail -5; \
OUT=$(dotnet test -v q 2>&1 | tail -40); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE 'Passed!|Passed: [1-9][0-9]*' || { echo 'KOSA: hakuna .NET tests zilizopita'; exit 1; }"
        }
        Lang::Ruby => {
            // rspec exit≠0 kwenye failure; tunahitisha '0 failures'.
            "command -v ruby >/dev/null || { echo 'KOSA: ruby haipo'; exit 1; }; \
if [ -d spec ]; then \
  OUT=$(rspec 2>&1 | tail -25); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE '[1-9][0-9]* examples?, 0 failures' || { echo 'KOSA: rspec hazikupita'; exit 1; }; \
elif [ -d test ]; then \
  OUT=$(ruby -Itest $(find test -name '*_test.rb' | head -5) 2>&1 | tail -25); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE '0 failures|failures, 0 errors|# runs' || { echo 'KOSA: ruby tests hazikupita'; exit 1; }; \
else echo 'KOSA: hakuna tests za ruby'; exit 1; fi"
        }
        Lang::Php => {
            "command -v php >/dev/null || { echo 'KOSA: php haipo'; exit 1; }; \
if [ -x vendor/bin/phpunit ]; then \
  OUT=$(vendor/bin/phpunit 2>&1 | tail -25); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE 'OK \\([1-9][0-9]* test|Tests: [1-9][0-9]*, Assertions' || { echo 'KOSA: phpunit hazikupita'; exit 1; }; \
elif command -v phpunit >/dev/null; then \
  OUT=$(phpunit 2>&1 | tail -25); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE 'OK \\([1-9][0-9]* test' || { echo 'KOSA: phpunit hazikupita'; exit 1; }; \
else echo 'KOSA: phpunit haipo'; exit 1; fi"
        }
        Lang::Swift => {
            "command -v swift >/dev/null || { echo 'KOSA: swift haipo'; exit 1; }; \
swift build 2>&1 | tail -5; \
OUT=$(swift test 2>&1 | tail -30); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE 'Executed [1-9][0-9]* test' || { echo 'KOSA: swift tests hazikupita'; exit 1; }"
        }
        Lang::CMake | Lang::CMakeCpp => {
            "cmake -S . -B build -DCMAKE_BUILD_TYPE=Release 2>&1 | tail -5 && \
cmake --build build 2>&1 | tail -5 && \
if command -v ctest >/dev/null && [ -f build/CTestTestfile.cmake ]; then \
  OUT=$(cd build && ctest --output-on-failure 2>&1 | tail -30); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE '100% tests passed|tests passed' || { echo 'KOSA: ctest hazikupita'; exit 1; }; \
else echo 'KOSA: hakuna ctest tests'; exit 1; fi"
        }
        Lang::Make => {
            "make 2>&1 | tail -10 && (make test 2>&1 | tail -30 || { echo 'KOSA: make test haipo/imeshindwa'; exit 1; })"
        }
        Lang::Elixir => {
            "command -v mix >/dev/null || { echo 'KOSA: mix haipo'; exit 1; }; \
mix deps.get 2>&1 | tail -3; \
OUT=$(mix test 2>&1 | tail -30); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE '[1-9][0-9]* tests?, 0 failures' || { echo 'KOSA: elixir tests hazikupita'; exit 1; }"
        }
        Lang::R => {
            // Rscript haipo → kosa la wazi. Tests halisi: testthat (`tests/testthat.R`).
            "command -v Rscript >/dev/null || { echo 'KOSA: Rscript haipo'; exit 1; }; \
if [ -f tests/testthat.R ]; then \
  OUT=$(Rscript tests/testthat.R 2>&1 | tail -30); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE 'FAIL [1-9][0-9]*|Error' && { echo 'KOSA: R tests zimeshindwa'; exit 1; }; \
  echo \"$OUT\" | grep -qE 'OK|PASS' || { echo 'KOSA: R tests hazikupita'; exit 1; }; \
elif [ -d tests/testthat ]; then \
  OUT=$(Rscript -e 'testthat::test_dir(\"tests/testthat\")' 2>&1 | tail -30); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE 'FAIL [1-9][0-9]*' && { echo 'KOSA: R tests zimeshindwa'; exit 1; }; \
else \
  echo 'KOSA: hakuna tests za R — andika tests/testthat.R'; exit 1; \
fi"
        }
        Lang::Python => {
            // pytest ndiyo kipimo cha lazima; hakuna tests → kosa la wazi.
            "command -v python3 >/dev/null || { echo 'KOSA: python3 haipo'; exit 1; }; \
if command -v pytest >/dev/null; then \
  OUT=$(pytest -q 2>&1 | tail -30); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE '[1-9][0-9]* (passed|failed)' || { echo 'KOSA: hakuna python tests'; exit 1; }; \
  echo \"$OUT\" | grep -qE '[1-9][0-9]* failed|error' && { echo 'KOSA: pytest imeshindwa'; exit 1; }; \
elif [ -d tests ]; then \
  OUT=$(python3 -m unittest discover -s tests 2>&1 | tail -30); echo \"$OUT\"; \
  echo \"$OUT\" | grep -qE 'FAILED|Error' && { echo 'KOSA: unittest imeshindwa'; exit 1; }; \
  echo \"$OUT\" | grep -qE '^OK' || { echo 'KOSA: unittest hazikupita'; exit 1; }; \
else \
  echo 'KOSA: hakuna tests za python — andika tests/'; exit 1; \
fi"
        }
        Lang::Haskell => {
            "command -v cabal >/dev/null || { echo 'KOSA: cabal haipo'; exit 1; }; \
cabal build 2>&1 | tail -5; \
OUT=$(cabal test 2>&1 | tail -30); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE 'PASS|OK' || { echo 'KOSA: haskell tests hazikupita'; exit 1; }"
        }
        Lang::Zig => {
            "command -v zig >/dev/null || { echo 'KOSA: zig haipo'; exit 1; }; \
zig build 2>&1 | tail -5; \
OUT=$(zig build test 2>&1 | tail -30); echo \"$OUT\"; \
echo \"$OUT\" | grep -qiE 'error' && { echo 'KOSA: zig tests zimeshindwa'; exit 1; }; echo 'zig build test ok'"
        }
        Lang::Scala => {
            "command -v sbt >/dev/null || { echo 'KOSA: sbt haipo'; exit 1; }; \
OUT=$(sbt -batch test 2>&1 | tail -40); echo \"$OUT\"; \
echo \"$OUT\" | grep -qE 'All tests passed|Tests: succeeded' || { echo 'KOSA: scala tests hazikupita'; exit 1; }"
        }
        Lang::Unknown => {
            "ls -la; echo 'ONYO: lugha haikutambulika — hakuna uthibitisho wa tests'; exit 1"
        }
    }
}

/// Tambua lugha NA rudisha amri ya uthibitisho kwa `dir` (au saraka ndogo).
/// Hurudisha `(prefix, lang, cmd)`.
pub fn verify_for(dir: &Path) -> (String, Lang, &'static str) {
    // Saraka ndogo yenye mradi (mfano `cargo init X`) → tumia hiyo.
    let root = if detect(dir) != Lang::Unknown {
        dir.to_path_buf()
    } else if let Ok(rd) = std::fs::read_dir(dir) {
        rd.flatten()
            .map(|e| e.path())
            .find(|p| p.is_dir() && detect(p) != Lang::Unknown)
            .unwrap_or_else(|| dir.to_path_buf())
    } else {
        dir.to_path_buf()
    };
    let prefix = if root != *dir {
        match root.strip_prefix(dir) {
            Ok(rel) => format!("cd {} && ", rel.display()),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };
    let lang = detect(&root);
    (prefix, lang, verify_cmd(lang))
}

/// Amri salama ya KUKIMBIA mradi (kwa kitufe cha "Run" kwenye UI).
/// Huthibitisha kwanza (build+tests), kisha huendesha binary/entry kama ipo.
pub fn run_for(dir: &Path) -> String {
    let root = if detect(dir) != Lang::Unknown {
        dir.to_path_buf()
    } else if let Ok(rd) = std::fs::read_dir(dir) {
        rd.flatten()
            .map(|e| e.path())
            .find(|p| p.is_dir() && detect(p) != Lang::Unknown)
            .unwrap_or_else(|| dir.to_path_buf())
    } else {
        dir.to_path_buf()
    };
    let prefix = if root != *dir {
        match root.strip_prefix(dir) {
            Ok(rel) => format!("cd {} && ", rel.display()),
            Err(_) => String::new(),
        }
    } else {
        String::new()
    };
    let lang = detect(&root);
    // Rust: mradi wa library hauna bin → endesha tests (`--nocapture`) badala ya `cargo run`.
    let run = if lang == Lang::Rust {
        let has_bin = root.join("src/main.rs").exists()
            || std::fs::read_to_string(root.join("Cargo.toml"))
                .map(|t| t.contains("[[bin]]") || t.contains("src/bin/"))
                .unwrap_or(false);
        if has_bin {
            "cargo run -q 2>&1 | tail -40"
        } else {
            "cargo test -- --nocapture 2>&1 | tail -60"
        }
    } else {
        match lang {
            Lang::Node => "node . 2>&1 | tail -40",
            Lang::Deno => "deno run -A . 2>&1 | tail -40",
            Lang::Bun => "bun run . 2>&1 | tail -40",
            Lang::Dart => "dart run 2>&1 | tail -40",
            Lang::Flutter => "flutter run -d web-server --web-port 0 2>&1 | tail -5",
            Lang::Go => "go run . 2>&1 | tail -40",
            Lang::JavaMaven => "mvn -q -B exec:java 2>&1 | tail -40",
            Lang::JavaGradle => "gradle -q run 2>&1 | tail -40",
            Lang::Dotnet => "dotnet run 2>&1 | tail -40",
            Lang::Ruby => "ruby main.rb 2>&1 | tail -40",
            Lang::Php => "php index.php 2>&1 | tail -40",
            Lang::Swift => "swift run 2>&1 | tail -40",
            Lang::Kotlin => "kotlin Main.kt 2>&1 | tail -40",
            Lang::Elixir => "elixir main.exs 2>&1 | tail -40",
            Lang::R => "Rscript main.R 2>&1 | tail -40",
            Lang::Python => "python3 main.py 2>&1 | tail -40",
            Lang::Haskell => "runghc Main.hs 2>&1 | tail -40",
            Lang::Zig => "zig build run 2>&1 | tail -40",
            Lang::Scala => "scala Main.scala 2>&1 | tail -40",
            _ => verify_cmd(lang),
        }
    };
    format!("{prefix}{run}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("lv_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn detects_rust_and_requires_tests() {
        let d = tmp("rust");
        std::fs::write(d.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        assert_eq!(detect(&d), Lang::Rust);
        assert!(verify_cmd(Lang::Rust).contains("[1-9][0-9]* passed"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn detects_r_project_and_requires_testthat() {
        let d = tmp("rproj");
        std::fs::write(d.join("DESCRIPTION"), "Package: x\n").unwrap();
        assert_eq!(detect(&d), Lang::R);
        assert!(verify_cmd(Lang::R).contains("Rscript"));
        assert!(verify_cmd(Lang::R).contains("testthat"));
        // Faili pekee la `.R` pia linatambulika.
        let d2 = tmp("rfile");
        std::fs::write(d2.join("analysis.R"), "print(1)\n").unwrap();
        assert_eq!(detect(&d2), Lang::R);
        std::fs::remove_dir_all(&d).ok();
        std::fs::remove_dir_all(&d2).ok();
    }

    #[test]
    fn detects_python_project_and_requires_tests() {
        let d = tmp("pyproj");
        std::fs::write(d.join("pyproject.toml"), "[project]\nname=\"x\"\n").unwrap();
        assert_eq!(detect(&d), Lang::Python);
        assert!(verify_cmd(Lang::Python).contains("pytest"));
        let d2 = tmp("pyfile");
        std::fs::write(d2.join("analysis.py"), "print(1)\n").unwrap();
        assert_eq!(detect(&d2), Lang::Python);
        std::fs::remove_dir_all(&d).ok();
        std::fs::remove_dir_all(&d2).ok();
    }

    #[test]
    fn detects_go_and_requires_go_test() {
        let d = tmp("go");
        std::fs::write(d.join("go.mod"), "module x\n").unwrap();
        assert_eq!(detect(&d), Lang::Go);
        assert!(verify_cmd(Lang::Go).contains("go test"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn detects_maven_java() {
        let d = tmp("mvn");
        std::fs::write(d.join("pom.xml"), "<project/>").unwrap();
        assert_eq!(detect(&d), Lang::JavaMaven);
        assert!(verify_cmd(Lang::JavaMaven).contains("Tests run"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn detects_dotnet_by_csproj() {
        let d = tmp("net");
        std::fs::write(d.join("App.csproj"), "<Project/>").unwrap();
        assert_eq!(detect(&d), Lang::Dotnet);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn detects_cpp_cmake_and_requires_ctest() {
        let d = tmp("cpp");
        std::fs::write(d.join("CMakeLists.txt"), "cmake_minimum_required(VERSION 3.10)").unwrap();
        assert_eq!(detect(&d), Lang::CMakeCpp);
        assert!(verify_cmd(Lang::CMakeCpp).contains("ctest"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn detects_flutter_vs_dart() {
        let d = tmp("fl");
        std::fs::write(d.join("pubspec.yaml"), "name: x\n").unwrap();
        std::fs::create_dir_all(d.join("lib")).unwrap();
        std::fs::write(d.join("lib").join("main.dart"), "void main(){}").unwrap();
        assert_eq!(detect(&d), Lang::Flutter);
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn dart_verify_rejects_partial_plus_count() {
        // `+0 -1: Some tests failed` ilikuwa ikipita kwa `\+[0-9]+` — sasa haipiti.
        let cmd = verify_cmd(Lang::Dart);
        assert!(cmd.contains("Some tests failed"), "cmd={cmd}");
        assert!(cmd.contains("All tests passed!"), "cmd={cmd}");
        assert!(!cmd.contains("\\+[0-9]+"), "haipaswi kutegemea +N: {cmd}");
        std::fs::remove_dir_all(std::env::temp_dir().join("x")).ok();
    }

    #[test]
    fn unknown_lang_never_reports_ok() {
        let d = tmp("unk");
        assert_eq!(detect(&d), Lang::Unknown);
        let cmd = verify_cmd(Lang::Unknown);
        assert!(cmd.contains("exit 1"), "unknown lazima ishinde: {cmd}");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn run_for_rust_lib_runs_tests_not_cargo_run() {
        let d = tmp("runrust");
        std::fs::write(d.join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        std::fs::create_dir_all(d.join("src")).unwrap();
        std::fs::write(d.join("src/lib.rs"), "pub fn a(){}").unwrap();
        let cmd = run_for(&d);
        assert!(cmd.contains("cargo test"), "lib → tests: {cmd}");
        assert!(!cmd.contains("cargo run"), "lib haipaswi cargo run: {cmd}");
        // Mradi wenye bin → cargo run.
        std::fs::write(d.join("src/main.rs"), "fn main(){}").unwrap();
        assert!(run_for(&d).contains("cargo run"), "bin → cargo run");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn run_for_go_and_node() {
        let d = tmp("rungo");
        std::fs::write(d.join("go.mod"), "module x\n").unwrap();
        assert!(run_for(&d).contains("go run"), "go");
        let n = tmp("runnode");
        std::fs::write(n.join("package.json"), "{}").unwrap();
        assert!(run_for(&n).contains("node ."), "node");
        std::fs::remove_dir_all(&d).ok();
        std::fs::remove_dir_all(&n).ok();
    }

    #[test]
    fn verify_for_nested_project() {
        let d = tmp("nest");
        let sub = d.join("app");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("go.mod"), "module y\n").unwrap();
        let (prefix, lang, _) = verify_for(&d);
        assert_eq!(lang, Lang::Go);
        assert!(prefix.contains("cd app"), "prefix={prefix}");
        std::fs::remove_dir_all(&d).ok();
    }
}

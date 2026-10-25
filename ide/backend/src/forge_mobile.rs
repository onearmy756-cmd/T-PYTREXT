//! forge_mobile.rs — FLUTTER PRODUCTION generator (domain-driven, bila LLM).
//!
//! Tofauti na `forge_generators` (scaffold) na `forge_projects` (PWA), hapa
//! tunazalisha MRADI KAMILI wa Flutter kutoka kwa `Domain` (entities + fields):
//! models (fromJson/toJson/copyWith), REST client, list + form screens, na
//! majaribio ya Dart (pure, yasiyohitaji flutter_test).
//!
//! Kasi: kizazi ni deterministic (template synthesis), hivyo hufikia mamilioni
//! ya "tokens" kwa sekunde — SI kasi ya LLM (~400 tok/s).

use crate::forge_domain::{to_pascal, to_snake, Domain, Entity, FieldType};

/// Je, kind hii ni Flutter/mobile-app production?
pub fn is_flutter(kind: &str) -> bool {
    matches!(
        kind,
        "flutter" | "flutter-app" | "mobile-app" | "mobile-flutter" | "dart-app"
    )
}

/// Aina ya Dart kutoka FieldType.
fn dart_type(t: FieldType) -> &'static str {
    match t {
        FieldType::Int => "int",
        FieldType::Float => "double",
        FieldType::Bool => "bool",
        FieldType::Timestamp => "DateTime",
        FieldType::Text | FieldType::Email | FieldType::Uuid => "String",
    }
}

/// Usemi wa fromJson kwa field.
fn from_json(name: &str, t: FieldType) -> String {
    match t {
        FieldType::Int => format!("(json['{name}'] as num).toInt()"),
        FieldType::Float => format!("(json['{name}'] as num).toDouble()"),
        FieldType::Bool => format!("json['{name}'] as bool"),
        FieldType::Timestamp => format!("DateTime.parse(json['{name}'] as String)"),
        _ => format!("json['{name}'] as String"),
    }
}

/// Usemi wa toJson kwa field (key → thamani).
fn to_json(name: &str, t: FieldType) -> String {
    match t {
        FieldType::Timestamp => format!("'{name}': {name}.toIso8601String()"),
        _ => format!("'{name}': {name}"),
    }
}

/// Thamani ya mfano (tests).
fn sample(t: FieldType) -> &'static str {
    match t {
        FieldType::Int => "1",
        FieldType::Float => "1.5",
        FieldType::Bool => "true",
        FieldType::Timestamp => "DateTime.utc(2024, 1, 1)",
        FieldType::Text | FieldType::Email | FieldType::Uuid => "'sample'",
    }
}

/// Thamani ya JSON ya mfano (tests) — kwa `json` map.
fn sample_json(t: FieldType) -> &'static str {
    match t {
        FieldType::Int => "1",
        FieldType::Float => "1.5",
        FieldType::Bool => "true",
        FieldType::Timestamp => "'2024-01-01T00:00:00.000Z'",
        FieldType::Text | FieldType::Email | FieldType::Uuid => "'sample'",
    }
}

/// Maneno ya Dart yaliyohifadhiwa (epuka kama jina la field).
const DART_RESERVED: &[&str] = &[
    "class", "for", "if", "else", "in", "is", "new", "var", "final", "const", "return",
    "switch", "case", "default", "this", "super", "null", "true", "false", "void", "int",
    "double", "bool", "String", "List", "Map", "Set", "enum", "do", "while", "break",
];

/// Jina la field linalofaa Dart (epuka neno lililohifadhiwa).
fn dart_field(name: &str) -> String {
    let n = to_snake(name);
    if DART_RESERVED.contains(&n.as_str()) {
        format!("{n}_")
    } else {
        n
    }
}

/// Je, field ni "managed" (id/created_at/updated_at — hazitumwa kwenye payload ya create)?
fn is_managed(name: &str) -> bool {
    matches!(name, "id" | "created_at" | "updated_at")
}

/// Zalisha mradi kamili wa Flutter (files, note).
pub fn generate_flutter(domain: &Domain) -> (Vec<(String, String)>, String) {
    let name = to_snake(&domain.project);
    let name = if name.is_empty() { "app".to_string() } else { name };
    let mut files: Vec<(String, String)> = Vec::new();

    files.push(("pubspec.yaml".into(), pubspec_yaml(&name)));
    files.push(("README.md".into(), readme(&name, domain)));
    files.push(("analysis_options.yaml".into(), analysis_options()));
    files.push(("lib/services/api_client.dart".into(), api_client()));
    files.push(("lib/main.dart".into(), main_dart(&name, domain)));

    for e in &domain.entities {
        let snake = to_snake(&e.name);
        files.push((
            format!("lib/models/{snake}.dart"),
            model_dart(&name, e),
        ));
        files.push((
            format!("lib/screens/{snake}_list_screen.dart"),
            list_screen(e),
        ));
        files.push((
            format!("lib/screens/{snake}_form_screen.dart"),
            form_screen(e),
        ));
        files.push((format!("test/{snake}_test.dart"), model_test(&name, e)));
        files.push((format!("test/{snake}_api_test.dart"), api_test(&name, e)));
    }

    // Badilisha placeholder ya jina la package kwenye majaribio (imports).
    for (_, content) in files.iter_mut() {
        *content = content.replace("APP_PKG", &name);
    }

    let note = format!(
        "flutter (production: models+api+screens+tests) entities {}",
        domain.entities.len()
    );
    (files, note)
}

/// Jaribio la ApiClient (pure — inatumia `MockClient`, SI mtandao halisi).
fn api_test(_pkg: &str, e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let table = e.table();
    let mut json_entries = String::new();
    for f in &e.fields {
        let n = dart_field(&f.name);
        json_entries.push_str(&format!("        '{n}': {},\n", sample_json(f.ty)));
    }
    format!(
        "import 'dart:convert';\n\
import 'package:http/http.dart' as http;\n\
import 'package:http/testing.dart';\n\
import 'package:test/test.dart';\n\
import 'package:APP_PKG/models/{snake}.dart';\n\
import 'package:APP_PKG/services/api_client.dart';\n\
\n\
void main() {{\n\
  group('ApiClient', () {{\n\
    test('list husoma items', () async {{\n\
      final mock = MockClient((req) async {{\n\
        return http.Response(\n\
          jsonEncode({{\n\
            'items': [\n\
              {{\n\
{json_entries}\
              }}\n\
            ],\n\
            'total': 1,\n\
          }}),\n\
          200,\n\
          headers: {{'content-type': 'application/json'}},\n\
        );\n\
      }});\n\
      final api = ApiClient(baseUrl: 'http://x', client: mock);\n\
      final items = await api.list('{table}');\n\
      expect(items.length, 1);\n\
      final m = {cls}.fromJson(items.first);\n\
      expect(m, isA<{cls}>());\n\
    }});\n\
\n\
    test('list hutupa ApiException kwa 500', () async {{\n\
      final mock = MockClient((req) async => http.Response('boom', 500));\n\
      final api = ApiClient(baseUrl: 'http://x', client: mock);\n\
      expect(() => api.list('{table}'), throwsA(isA<ApiException>()));\n\
    }});\n\
  }});\n\
}}\n",
        snake = to_snake(&e.name)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge_domain::parse_domain;

    #[test]
    fn flutter_inazalisha_nyaraka_zote() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        let (files, note) = generate_flutter(&domain);
        assert!(note.contains("production"), "note: {note}");
        assert!(note.contains("flutter"));
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        for want in [
            "pubspec.yaml",
            "lib/main.dart",
            "lib/services/api_client.dart",
            "lib/models/bidhaa.dart",
            "lib/screens/bidhaa_list_screen.dart",
            "lib/screens/bidhaa_form_screen.dart",
            "test/bidhaa_test.dart",
            "test/bidhaa_api_test.dart",
        ] {
            assert!(paths.contains(&want), "haipo: {want}");
        }
    }

    #[test]
    fn model_ina_fromjson_tojson_na_payload() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        let (files, _) = generate_flutter(&domain);
        let model = files
            .iter()
            .find(|(p, _)| p == "lib/models/bidhaa.dart")
            .unwrap();
        assert!(model.1.contains("class Bidhaa {"));
        assert!(model.1.contains("factory Bidhaa.fromJson"));
        assert!(model.1.contains("Map<String, dynamic> toJson()"));
        assert!(model.1.contains("Map<String, dynamic> toCreatePayload()"));
        assert!(model.1.contains("final double bei;"), "bei iwe double");
        assert!(model.1.contains("final int idadi;"), "idadi iwe int");
        // timestamp inatumia DateTime.parse.
        assert!(model.1.contains("DateTime.parse(json['created_at'] as String)"));
        // copyWith ipo.
        assert!(model.1.contains("Bidhaa copyWith({"));
    }

    #[test]
    fn payload_haijumuishi_managed_fields() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        let (files, _) = generate_flutter(&domain);
        let model = files
            .iter()
            .find(|(p, _)| p == "lib/models/bidhaa.dart")
            .unwrap();
        // Sehemu ya toCreatePayload haipaswi kuwa na id/created_at.
        let start = model.1.find("toCreatePayload").unwrap();
        let tail = &model.1[start..];
        let end = tail.find("};").map(|i| i + start).unwrap_or(model.1.len());
        let payload = &model.1[start..end];
        assert!(!payload.contains("'id'"), "payload isiwe na id");
        assert!(!payload.contains("'created_at'"), "payload isiwe na created_at");
        assert!(payload.contains("'jina'"), "payload iwe na jina");
    }

    #[test]
    fn majaribio_yanatumia_package_test_si_flutter_test() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        let (files, _) = generate_flutter(&domain);
        for (p, c) in &files {
            if p.starts_with("test/") {
                assert!(c.contains("package:test/test.dart"), "{p} itumie package:test");
                assert!(!c.contains("flutter_test"), "{p} isitumie flutter_test");
            }
        }
    }

    #[test]
    fn api_client_ina_crud_zote() {
        let domain = parse_domain("mradi wa Bidhaa(jina)");
        let (files, _) = generate_flutter(&domain);
        let api = files
            .iter()
            .find(|(p, _)| p == "lib/services/api_client.dart")
            .unwrap();
        for m in ["Future<List<Map<String, dynamic>>> list(", "Future<Map<String, dynamic>> create(", "Future<Map<String, dynamic>> update(", "Future<void> delete("] {
            assert!(api.1.contains(m), "ApiClient haipo: {m}");
        }
    }

    #[test]
    fn dart_field_huepuka_neno_lililohifadhiwa() {
        assert_eq!(dart_field("class"), "class_");
        assert_eq!(dart_field("name"), "name");
        assert_eq!(dart_field("Jina"), "jina");
    }

    #[test]
    fn entities_nyingi_huzalisha_skrini_nyingi() {
        let domain = parse_domain("mradi wa Duka lenye Bidhaa(jina, bei), Mteja(jina, email)");
        let (files, _) = generate_flutter(&domain);
        let paths: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();
        assert!(paths.contains(&"lib/models/bidhaa.dart"));
        assert!(paths.contains(&"lib/models/mteja.dart"));
        assert!(paths.contains(&"lib/screens/mteja_list_screen.dart"));
    }
}

fn pubspec_yaml(name: &str) -> String {
    format!(
        r#"name: {name}
description: Flutter mobile app (production, generated by MTECHOS FORGE).
publish_to: 'none'
version: 1.0.0+1

environment:
  sdk: '>=3.0.0 <4.0.0'

dependencies:
  flutter:
    sdk: flutter
  http: ^1.2.0
  intl: ^0.19.0

dev_dependencies:
  flutter_test:
    sdk: flutter
  test: ^1.24.0
  flutter_lints: ^3.0.0

flutter:
  uses-material-design: true
"#
    )
}

fn analysis_options() -> String {
    r#"include: package:flutter_lints/flutter.yaml

linter:
  rules:
    prefer_const_constructors: true
    avoid_print: true
"#
    .to_string()
}

fn readme(name: &str, domain: &Domain) -> String {
    let mut s = format!(
        "# {name}\n\n\
Flutter mobile app (production) — imezalishwa na MTECHOS FORGE (domain-driven, bila LLM).\n\n\
## Entities\n\n"
    );
    for e in &domain.entities {
        s.push_str(&format!("- **{}** ({})\n", e.class(), e.table()));
    }
    s.push_str(
        "\n## Endesha\n\n\
```bash\n\
flutter pub get\n\
flutter test\n\
flutter run\n\
```\n",
    );
    s
}

/// REST client wa jumla (CRUD kwa rasilimali zote).
fn api_client() -> String {
    r#"import 'dart:convert';
import 'package:http/http.dart' as http;

/// REST client (production) — CRUD kwa rasilimali zote.
class ApiClient {
  final String baseUrl;
  final http.Client _client;

  ApiClient({required this.baseUrl, http.Client? client})
      : _client = client ?? http.Client();

  Uri _uri(String path, [Map<String, String>? query]) =>
      Uri.parse('$baseUrl$path').replace(queryParameters: query);

  Future<List<Map<String, dynamic>>> list(String resource,
      {int offset = 0, int limit = 50}) async {
    final res = await _client.get(_uri('/$resource',
        {'offset': '$offset', 'limit': '$limit'}));
    if (res.statusCode != 200) {
      throw ApiException(res.statusCode, res.body);
    }
    final body = jsonDecode(res.body) as Map<String, dynamic>;
    return (body['items'] as List).cast<Map<String, dynamic>>();
  }

  Future<Map<String, dynamic>> create(
      String resource, Map<String, dynamic> data) async {
    final res = await _client.post(_uri('/$resource'),
        headers: {'Content-Type': 'application/json'}, body: jsonEncode(data));
    if (res.statusCode != 201) {
      throw ApiException(res.statusCode, res.body);
    }
    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  Future<Map<String, dynamic>> update(
      String resource, String id, Map<String, dynamic> data) async {
    final res = await _client.patch(_uri('/$resource/$id'),
        headers: {'Content-Type': 'application/json'}, body: jsonEncode(data));
    if (res.statusCode != 200) {
      throw ApiException(res.statusCode, res.body);
    }
    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  Future<void> delete(String resource, String id) async {
    final res = await _client.delete(_uri('/$resource/$id'));
    if (res.statusCode != 204 && res.statusCode != 200) {
      throw ApiException(res.statusCode, res.body);
    }
  }
}

/// Hitilafu ya API (status + mwili).
class ApiException implements Exception {
  final int statusCode;
  final String body;
  ApiException(this.statusCode, this.body);

  @override
  String toString() => 'ApiException($statusCode): $body';
}
"#
    .to_string()
}

/// Model ya entity (fromJson/toJson/copyWith + payload ya create).
fn model_dart(_pkg: &str, e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let mut s = String::new();
    s.push_str(&format!("/// {cls} model (production).\n"));
    s.push_str(&format!("class {cls} {{\n"));

    for f in &e.fields {
        let n = dart_field(&f.name);
        let ty = dart_type(f.ty);
        s.push_str(&format!("  final {ty} {n};\n"));
    }
    s.push('\n');

    // Constructor.
    s.push_str(&format!("  const {cls}({{\n"));
    for f in &e.fields {
        let n = dart_field(&f.name);
        s.push_str(&format!("    required this.{n},\n"));
    }
    s.push_str("  });\n\n");

    // fromJson.
    s.push_str(&format!(
        "  factory {cls}.fromJson(Map<String, dynamic> json) => {cls}(\n"
    ));
    for f in &e.fields {
        let n = dart_field(&f.name);
        s.push_str(&format!("    {n}: {},\n", from_json(&n, f.ty)));
    }
    s.push_str("  );\n\n");

    // toJson.
    s.push_str("  Map<String, dynamic> toJson() => {\n");
    for f in &e.fields {
        let n = dart_field(&f.name);
        s.push_str(&format!("    {},\n", to_json(&n, f.ty)));
    }
    s.push_str("  };\n\n");

    // Payload ya create (bila managed fields).
    s.push_str("  /// Payload ya POST (bila id/created_at/updated_at).\n");
    s.push_str("  Map<String, dynamic> toCreatePayload() => {\n");
    for f in &e.fields {
        let n = dart_field(&f.name);
        if is_managed(&n) {
            continue;
        }
        s.push_str(&format!("    {},\n", to_json(&n, f.ty)));
    }
    s.push_str("  };\n\n");

    // copyWith.
    s.push_str(&format!("  {cls} copyWith({{\n"));
    for f in &e.fields {
        let n = dart_field(&f.name);
        let ty = dart_type(f.ty);
        s.push_str(&format!("    {ty}? {n},\n"));
    }
    s.push_str(&format!("  }}) => {cls}(\n"));
    for f in &e.fields {
        let n = dart_field(&f.name);
        s.push_str(&format!("    {n}: {n} ?? this.{n},\n"));
    }
    s.push_str("  );\n");

    s.push_str("}\n");
    s
}

/// Skrini ya orodha (ListView + FutureBuilder).
fn list_screen(e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let snake = to_snake(&e.name);
    let table = e.table();
    // Kichwa cha orodha: field ya kwanza isiyo managed (name/title/…).
    let title_field = e
        .fields
        .iter()
        .map(|f| dart_field(&f.name))
        .find(|n| !is_managed(n))
        .unwrap_or_else(|| "id".to_string());
    format!(
        "import 'package:flutter/material.dart';\n\
import '../models/{snake}.dart';\n\
import '../services/api_client.dart';\n\
\n\
/// Skrini ya orodha ya {cls} (production).\n\
class {cls}ListScreen extends StatefulWidget {{\n\
  final ApiClient api;\n\
  const {cls}ListScreen({{super.key, required this.api}});\n\
\n\
  @override\n\
  State<{cls}ListScreen> createState() => _{cls}ListScreenState();\n\
}}\n\
\n\
class _{cls}ListScreenState extends State<{cls}ListScreen> {{\n\
  late Future<List<{cls}>> _future;\n\
\n\
  @override\n\
  void initState() {{\n\
    super.initState();\n\
    _future = _load();\n\
  }}\n\
\n\
  Future<List<{cls}>> _load() async {{\n\
    final items = await widget.api.list('{table}');\n\
    return items.map({cls}.fromJson).toList();\n\
  }}\n\
\n\
  Future<void> _refresh() async {{\n\
    setState(() {{ _future = _load(); }});\n\
    await _future;\n\
  }}\n\
\n\
  @override\n\
  Widget build(BuildContext context) {{\n\
    return Scaffold(\n\
      appBar: AppBar(title: const Text('{cls}'), actions: [\n\
        IconButton(\n\
          icon: const Icon(Icons.add),\n\
          onPressed: () => Navigator.of(context).push(MaterialPageRoute(\n\
            builder: (_) => {cls}FormScreen(api: widget.api),\n\
          )).then((_) => _refresh()),\n\
        ),\n\
      ]),\n\
      body: FutureBuilder<List<{cls}>>(\n\
        future: _future,\n\
        builder: (context, snap) {{\n\
          if (snap.connectionState == ConnectionState.waiting) {{\n\
            return const Center(child: CircularProgressIndicator());\n\
          }}\n\
          if (snap.hasError) {{\n\
            return Center(child: Text('Hitilafu: ${{snap.error}}'));\n\
          }}\n\
          final items = snap.data ?? const [];\n\
          if (items.isEmpty) {{\n\
            return const Center(child: Text('Hakuna data'));\n\
          }}\n\
          return RefreshIndicator(\n\
            onRefresh: _refresh,\n\
            child: ListView.builder(\n\
              itemCount: items.length,\n\
              itemBuilder: (context, i) => ListTile(\n\
                leading: const Icon(Icons.chevron_right),\n\
                title: Text(items[i].{title_field}.toString()),\n\
                subtitle: Text(items[i].id),\n\
              ),\n\
            ),\n\
          );\n\
        }},\n\
      ),\n\
    );\n\
  }}\n\
}}\n"
    )
}

/// Skrini ya fomu (kuunda).
fn form_screen(e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let snake = to_snake(&e.name);
    let table = e.table();
    let mut controllers = String::new();
    let mut fields = String::new();
    let mut payload = String::new();
    let mut dispose = String::new();
    for f in &e.fields {
        let n = dart_field(&f.name);
        if is_managed(&n) {
            continue;
        }
        controllers.push_str(&format!(
            "  final _{n}Controller = TextEditingController();\n"
        ));
        let label = to_pascal(&n);
        fields.push_str(&format!(
            "          TextField(\n\
            controller: _{n}Controller,\n\
            decoration: const InputDecoration(labelText: '{label}'),\n\
          ),\n"
        ));
        let expr = match f.ty {
            FieldType::Int => format!("int.tryParse(_{n}Controller.text) ?? 0"),
            FieldType::Float => format!("double.tryParse(_{n}Controller.text) ?? 0"),
            FieldType::Bool => format!("_{n}Controller.text == 'true'"),
            _ => format!("_{n}Controller.text"),
        };
        payload.push_str(&format!("      '{n}': {expr},\n"));
        dispose.push_str(&format!("    _{n}Controller.dispose();\n"));
    }
    let _ = snake;
    format!(
        "import 'package:flutter/material.dart';\n\
import '../models/{snake}.dart';\n\
import '../services/api_client.dart';\n\
\n\
/// Fomu ya kuunda {cls} (production).\n\
class {cls}FormScreen extends StatefulWidget {{\n\
  final ApiClient api;\n\
  const {cls}FormScreen({{super.key, required this.api}});\n\
\n\
  @override\n\
  State<{cls}FormScreen> createState() => _{cls}FormScreenState();\n\
}}\n\
\n\
class _{cls}FormScreenState extends State<{cls}FormScreen> {{\n\
{controllers}\
  bool _saving = false;\n\
\n\
  @override\n\
  void dispose() {{\n\
{dispose}\
    super.dispose();\n\
  }}\n\
\n\
  Future<void> _save() async {{\n\
    setState(() {{ _saving = true; }});\n\
    try {{\n\
      await widget.api.create('{table}', {{\n\
{payload}\
      }});\n\
      if (mounted) Navigator.of(context).pop(true);\n\
    }} catch (e) {{\n\
      if (mounted) {{\n\
        ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Hitilafu: $e')));\n\
      }}\n\
    }} finally {{\n\
      if (mounted) setState(() {{ _saving = false; }});\n\
    }}\n\
  }}\n\
\n\
  @override\n\
  Widget build(BuildContext context) {{\n\
    return Scaffold(\n\
      appBar: AppBar(title: const Text('Ongeza {cls}')),\n\
      body: ListView(padding: const EdgeInsets.all(16), children: [\n\
{fields}\
        const SizedBox(height: 24),\n\
        FilledButton(\n\
          onPressed: _saving ? null : _save,\n\
          child: _saving ? const CircularProgressIndicator() : const Text('Hifadhi'),\n\
        ),\n\
      ]),\n\
    );\n\
  }}\n\
}}\n"
    )
}

/// main.dart — MaterialApp + njia za skrini zote.
fn main_dart(_pkg: &str, domain: &Domain) -> String {
    let mut imports = String::new();
    let mut routes = String::new();
    for e in &domain.entities {
        let cls = to_pascal(&e.name);
        let snake = to_snake(&e.name);
        imports.push_str(&format!("import 'screens/{snake}_list_screen.dart';\n"));
        routes.push_str(&format!(
            "            '{snake}': (ctx) => {cls}ListScreen(api: api),\n"
        ));
    }
    let first = domain
        .entities
        .first()
        .map(|e| format!("{}ListScreen(api: api)", to_pascal(&e.name)))
        .unwrap_or_else(|| "const Center(child: Text('Hakuna entity'))".to_string());
    format!(
        "import 'package:flutter/material.dart';\n\
import 'services/api_client.dart';\n\
{imports}\
\n\
void main() => runApp(const App());\n\
\n\
class App extends StatelessWidget {{\n\
  const App({{super.key}});\n\
\n\
  @override\n\
  Widget build(BuildContext context) {{\n\
    final api = ApiClient(baseUrl: 'https://api.example.com');\n\
    return MaterialApp(\n\
      title: 'MTECHOS App',\n\
      theme: ThemeData(colorSchemeSeed: Colors.indigo, useMaterial3: true),\n\
      home: {first},\n\
      routes: {{\n\
{routes}\
      }},\n\
    );\n\
  }}\n\
}}\n"
    )
}

/// Jaribio la Dart (pure — linatumia `package:test`, SI flutter_test).
fn model_test(_pkg: &str, e: &Entity) -> String {
    let cls = to_pascal(&e.name);
    let snake = to_snake(&e.name);
    let mut json_entries = String::new();
    let mut ctor_args = String::new();
    for f in &e.fields {
        let n = dart_field(&f.name);
        json_entries.push_str(&format!("      '{n}': {},\n", sample_json(f.ty)));
        ctor_args.push_str(&format!("      {n}: {},\n", sample(f.ty)));
    }
    // Assertions: kila field ina thamani sahihi.
    let mut asserts = String::new();
    for f in &e.fields {
        let n = dart_field(&f.name);
        let val = match f.ty {
            FieldType::Timestamp => "DateTime.utc(2024, 1, 1)".to_string(),
            _ => sample(f.ty).to_string(),
        };
        asserts.push_str(&format!("    expect(m.{n}, equals({val}));\n"));
    }
    format!(
        "import 'package:test/test.dart';\n\
import 'package:APP_PKG/models/{snake}.dart';\n\
\n\
void main() {{\n\
  group('{cls} model', () {{\n\
    final json = <String, dynamic>{{\n\
{json_entries}\
    }};\n\
\n\
    test('fromJson husoma fields zote', () {{\n\
      final m = {cls}.fromJson(json);\n\
{asserts}\
    }});\n\
\n\
    test('toJson huzunguka (roundtrip)', () {{\n\
      final m = {cls}(\n\
{ctor_args}\
      );\n\
      final out = m.toJson();\n\
      for (final k in json.keys) {{\n\
        expect(out.containsKey(k), isTrue, reason: 'k=$k');\n\
      }}\n\
    }});\n\
\n\
    test('toCreatePayload haitoi managed fields', () {{\n\
      final m = {cls}.fromJson(json);\n\
      final p = m.toCreatePayload();\n\
      expect(p.containsKey('id'), isFalse);\n\
      expect(p.containsKey('created_at'), isFalse);\n\
    }});\n\
  }});\n\
}}\n"
    )
}

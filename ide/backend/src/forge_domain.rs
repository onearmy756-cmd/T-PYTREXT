//! forge_domain.rs — kuchambua DOMAIN kutoka kwa spec (entities + fields + rules).
//!
//! Kikwazo cha "scaffolding": generator haina habari ya domain, hivyo hutoa stubs.
//! Hapa tunachambua spec (maandishi ya mtumiaji) → entities, fields, types, rules.
//! Bila LLM: kuchambua kwa neno-msingi + jedwali la aina. Matokeo ni deterministic
//! na yanaweza kutumika kuzalisha kodi ya PRODUCTION (models, validation, tests).

use serde_json::{json, Value};

/// Field ya entity (jina, aina ya data, required, kikomo).
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: FieldType,
    pub required: bool,
    pub unique: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Int,
    Float,
    Text,
    Bool,
    Timestamp,
    Email,
    Uuid,
}

impl FieldType {
    pub fn as_str(self) -> &'static str {
        match self {
            FieldType::Int => "int",
            FieldType::Float => "float",
            FieldType::Text => "text",
            FieldType::Bool => "bool",
            FieldType::Timestamp => "timestamp",
            FieldType::Email => "email",
            FieldType::Uuid => "uuid",
        }
    }

    /// Aina ya SQL ya portable.
    pub fn sql(self) -> &'static str {
        match self {
            FieldType::Int => "BIGINT",
            FieldType::Float => "DOUBLE PRECISION",
            FieldType::Text => "TEXT",
            FieldType::Bool => "BOOLEAN",
            FieldType::Timestamp => "TIMESTAMPTZ",
            FieldType::Email => "TEXT",
            FieldType::Uuid => "TEXT",
        }
    }
}

/// Entity (jedwali/resource) yenye fields.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub name: String,
    pub fields: Vec<Field>,
}

impl Entity {
    /// Jina la jedwali (snake_case, wingi rahisi).
    pub fn table(&self) -> String {
        let mut s = to_snake(&self.name);
        if !s.ends_with('s') {
            s.push('s');
        }
        s
    }

    /// Jina la darasa (PascalCase).
    pub fn class(&self) -> String {
        to_pascal(&self.name)
    }
}

/// Domain kamili ya mradi.
#[derive(Debug, Clone, PartialEq)]
pub struct Domain {
    pub project: String,
    pub entities: Vec<Entity>,
}

/// Jedwali la aina: neno → FieldType (Swahili + English).
const TYPE_HINTS: &[(&[&str], FieldType)] = &[
    (&["email", "barua pepe", "pepe"], FieldType::Email),
    (&["uuid", "kitambulisho cha kipekee"], FieldType::Uuid),
    (&["tarehe", "muda", "date", "time", "created", "updated", "timestamp", "created_at"], FieldType::Timestamp),
    (&["bei", "price", "amount", "kiasi", "cost", "salary", "malipo", "total", "rate", "wastani"], FieldType::Float),
    (&["idadi", "count", "quantity", "qty", "idadi ya", "number", "namba", "age", "umri", "stock", "id"], FieldType::Int),
    (&["hai", "active", "enabled", "imewashwa", "is_", "deleted", "verified", "imehakikiwa"], FieldType::Bool),
    (&["jina", "name", "title", "kichwa", "maelezo", "description", "text", "content", "anwani", "address", "note", "message", "ujumbe", "status", "hali"], FieldType::Text),
];

/// Amua aina ya field kutoka jina lake.
fn infer_type(raw: &str) -> FieldType {
    let s = raw.to_lowercase();
    for (keys, ty) in TYPE_HINTS {
        for k in *keys {
            if s.contains(k) {
                return *ty;
            }
        }
    }
    FieldType::Text
}

/// Fields za msingi zinazoongezwa kwa kila entity (production default).
fn base_fields() -> Vec<Field> {
    vec![
        Field { name: "id".into(), ty: FieldType::Uuid, required: true, unique: true },
        Field { name: "created_at".into(), ty: FieldType::Timestamp, required: true, unique: false },
        Field { name: "updated_at".into(), ty: FieldType::Timestamp, required: true, unique: false },
    ]
}

fn field(name: &str) -> Field {
    let ty = infer_type(name);
    Field {
        name: to_snake(name),
        ty,
        required: !matches!(ty, FieldType::Bool),
        unique: ty == FieldType::Email,
    }
}

/// Maneno ya kawaida yasiyokuwa entity (kuepuka kuchukua kelele kama entities).
const STOPWORDS: &[&str] = &[
    "api", "app", "mradi", "project", "system", "mfumo", "service", "huduma", "server",
    "crud", "rest", "json", "http", "https", "database", "db", "sqlite", "postgres",
    "tengeneza", "unda", "nahitaji", "nataka", "kwa", "na", "ya", "za", "wa", "la",
    "production", "kodi", "code", "mtandao", "web", "mobile", "frontend", "backend",
    "test", "tests", "testi", "docker", "deploy", "usimamizi", "management", "full",
    "stack", "real", "halisi", "kubwa", "ndogo", "kila", "zote", "nyingi",
    "fastapi", "express", "django", "flask", "axum", "sqlx", "postgres", "sqlalchemy",
    "rust", "typescript", "javascript", "golang", "elixir", "kotlin", "swift",
    "productions", "kutumia", "yenye", "lenye", "wenye", "hii", "hiyo", "kama", "pia",
];

/// Chambua spec → Domain. Hii ni njia ya kwanza (deterministic).
pub fn parse_domain(spec: &str) -> Domain {
    let (project, rest) = extract_project(spec);
    let entities = extract_entities(rest);
    let entities = if entities.is_empty() {
        vec![Entity {
            name: "item".into(),
            fields: {
                let mut f = vec![
                    Field { name: "name".into(), ty: FieldType::Text, required: true, unique: false },
                    Field { name: "description".into(), ty: FieldType::Text, required: false, unique: false },
                ];
                f.extend(base_fields());
                f
            },
        }]
    } else {
        entities
    };
    Domain { project, entities }
}

/// Toa jina la mradi kutoka spec (au default).
fn extract_project(spec: &str) -> (String, &str) {
    let lower = spec.to_lowercase();
    for kw in ["mradi wa", "project ya", "project", "mfumo wa", "system ya"] {
        if let Some(pos) = lower.find(kw) {
            let after = &spec[pos + kw.len()..];
            let word = after
                .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                .find(|w| !w.trim().is_empty() && w.chars().any(|c| c.is_alphabetic()));
            if let Some(w) = word {
                let w = w.trim();
                if w.len() >= 2 && !STOPWORDS.contains(&w.to_lowercase().as_str()) {
                    return (sanitize(w), spec);
                }
            }
        }
    }
    ("mradi".into(), spec)
}

fn sanitize(s: &str) -> String {
    to_snake(s).trim_matches('_').to_string()
}

/// Toa entities + fields kutoka maandishi.
///
/// Miundo inayotumika:
/// - `Entity(field1, field2, ...)` — mabano (wazi kabisa)
/// - `Entity: field1, field2, ...` — koloni hadi kituo/mstari
/// - orodha ya nomino (kila neno la KISWAHILI/English linaloonekana kama entity)
fn extract_entities(text: &str) -> Vec<Entity> {
    let mut out: Vec<Entity> = Vec::new();

    // 1) Muundo wa mabano: Name(a, b, c)
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            // tafuta neno kabla ya '('
            let mut j = i;
            while j > 0 && (bytes[j - 1] as char).is_alphanumeric() {
                j -= 1;
            }
            let name = text[j..i].trim();
            // tafuta ')' inayofunga
            if let Some(close_rel) = text[i + 1..].find(')') {
                let inside = &text[i + 1..i + 1 + close_rel];
                if is_entity_name(name) && inside.contains(|c: char| c.is_alphabetic()) {
                    let mut ent = Entity {
                        name: to_snake(name),
                        fields: inside
                            .split(',')
                            .map(|f| f.trim())
                            .filter(|f| !f.is_empty())
                            .map(field)
                            .collect(),
                    };
                    ent.fields.extend(base_fields());
                    out.push(ent);
                    i = i + 1 + close_rel + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    if !out.is_empty() {
        return dedupe(out);
    }

    // 2) Muundo wa koloni: `Entity: f1, f2` kwa kila mstari.
    for line in text.lines() {
        let line = line.trim();
        if let Some(colon) = line.find(':') {
            let name = line[..colon].trim();
            let rest = line[colon + 1..].trim();
            if is_entity_name(name) && rest.contains(|c: char| c.is_alphabetic()) {
                let mut ent = Entity {
                    name: to_snake(name),
                    fields: rest
                        .split(|c| c == ',' || c == ';')
                        .map(|f| f.trim())
                        .filter(|f| !f.is_empty() && f.len() < 40)
                        .map(field)
                        .collect(),
                };
                if ent.fields.len() >= 2 {
                    ent.fields.extend(base_fields());
                    out.push(ent);
                }
            }
        }
    }
    if !out.is_empty() {
        return dedupe(out);
    }

    // 3) Fallback: nomino MOJA inayoonekana kama entity (kuepuka kelele).
    for w in text.split(|c: char| !c.is_alphanumeric()) {
        if is_entity_name(w) && w.chars().count() >= 4 {
            let mut ent = Entity {
                name: to_snake(w),
                fields: vec![
                    Field { name: "name".into(), ty: FieldType::Text, required: true, unique: false },
                    Field { name: "description".into(), ty: FieldType::Text, required: false, unique: false },
                ],
            };
            ent.fields.extend(base_fields());
            out.push(ent);
            break;
        }
    }
    dedupe(out)
}

fn is_entity_name(w: &str) -> bool {
    let t = w.trim();
    if t.chars().count() < 3 || t.chars().count() > 30 {
        return false;
    }
    if !t.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
        return false;
    }
    let low = t.to_lowercase();
    !STOPWORDS.contains(&low.as_str())
}

fn dedupe(mut v: Vec<Entity>) -> Vec<Entity> {
    let mut seen = std::collections::HashSet::new();
    v.retain(|e| seen.insert(e.name.clone()));
    v
}

pub fn to_snake(s: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() && prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
            prev_lower = c.is_lowercase();
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
            prev_lower = false;
        }
    }
    out.trim_matches('_').to_string()
}

pub fn to_pascal(s: &str) -> String {
    to_snake(s)
        .split('_')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// JSON ya domain (kwa UI/debug).
pub fn to_json(d: &Domain) -> Value {
    json!({
        "project": d.project,
        "entities": d.entities.iter().map(|e| json!({
            "name": e.name,
            "table": e.table(),
            "class": e.class(),
            "fields": e.fields.iter().map(|f| json!({
                "name": f.name, "type": f.ty.as_str(), "sql": f.ty.sql(),
                "required": f.required, "unique": f.unique,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inachambua_muundo_wa_mabano() {
        let d = parse_domain("Tengeneza API ya mradi wa Duka lenye Bidhaa(jina, bei, idadi)");
        assert_eq!(d.project, "duka");
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].name, "bidhaa");
        let names: Vec<_> = d.entities[0].fields.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"jina"));
        assert!(names.contains(&"bei"));
        assert!(names.contains(&"idadi"));
        assert!(names.contains(&"id"));
        assert!(names.contains(&"created_at"));
    }

    #[test]
    fn inatambua_aina_za_fields() {
        let d = parse_domain("mradi wa CRM yenye Customer(email, age, active, jina)");
        let f = &d.entities[0].fields;
        let get = |n: &str| f.iter().find(|x| x.name == n).unwrap();
        assert_eq!(get("email").ty, FieldType::Email);
        assert!(get("email").unique);
        assert_eq!(get("age").ty, FieldType::Int);
        assert_eq!(get("active").ty, FieldType::Bool);
        assert_eq!(get("jina").ty, FieldType::Text);
    }

    #[test]
    fn entities_nyingi() {
        let d = parse_domain("mradi wa Shule: Wanafunzi(jina, umri), Walimu(jina, email)");
        assert_eq!(d.project, "shule");
        let names: Vec<_> = d.entities.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"wanafunzi"));
        assert!(names.contains(&"walimu"));
    }

    #[test]
    fn fallback_kwa_entity_moja() {
        let d = parse_domain("tengeneza bidhaa");
        assert_eq!(d.entities.len(), 1);
        assert_eq!(d.entities[0].name, "bidhaa");
    }

    #[test]
    fn snake_na_pascal() {
        assert_eq!(to_snake("BidhaaZote"), "bidhaa_zote");
        assert_eq!(to_pascal("bidhaa_zote"), "BidhaaZote");
        assert_eq!(to_snake("Order Item"), "order_item");
    }
}

//! forge_generators.rs — FORGE GENERATORS: scaffolds DETERMINISTIC (hakuna LLM).
//!
//! Kikwazo cha "milioni ya faili": LLM per-file ~10-15s/faili — haiwezi kufikia
//! mamilioni. Generators huzalisha faili nyingi kwa sekunde kwa kutengeneza kodi
//! kutoka TEMPLATES + DATA (fomu, moduli, resources). Hii ni njia ya kweli kwa
//! miradi mikubwa (mfano: scaffold ya Flutter app yenye moduli 1000, au API
//! yenye resources 500).
//!
//! Kanuni: kila faili ni HALISI (ina compile) — templates zinaandikwa kwa mkono.
//! Kasi = maelfu ya faili/sekunde; uwezo = ukomo (data-driven).

use serde_json::{json, Value};

/// Aina ya generator inayotambulika.
pub fn detect(spec: &str) -> Option<&'static str> {
    let s = spec.to_lowercase();
    // 3D kwa Rust (production, std-only) — kabla ya rust-api.
    if s.contains("rust")
        && (s.contains("3d") || s.contains("3-d") || s.contains("mchezo") || s.contains("game"))
    {
        return Some("rust-3d");
    }
    if s.contains("moduli nyingi") || s.contains("modules nyingi") || s.contains("milioni") {
        return Some("modules");
    }
    // Flutter/Dart production (models+api+screens+tests) — kabla ya scaffold nyingine.
    if s.contains("flutter") || s.contains("dart app") || s.contains("app ya dart") {
        return Some("flutter");
    }
    // ML/AI (framework-specific — nyongeza, hakuna kubadilisha zilizopo)
    if s.contains("candle") {
        return Some("ml-rust");
    }
    if s.contains("crud") || s.contains("resources nyingi") || s.contains("api nyingi") {
        return Some("crud");
    }
    if s.contains("elixir") || s.contains("phoenix") {
        return Some("elixir-app");
    }
    if s.contains("r analytics") || s.contains("r scripts") || s.contains("uchanganuzi wa r") {
        return Some("r-analytics");
    }
    if s.contains("golang") || s.contains("go api") || s.contains("go server") || s.contains("go crud") {
        return Some("go-crud");
    }
    // Lugha mpya za API (nyongeza — hazibadilishi tabia za awali).
    if s.contains("typescript") || s.contains("ts api") || s.contains("fastify") {
        return Some("ts-api");
    }
    if s.contains("express") || s.contains("node api") || s.contains("node server") {
        return Some("node-api");
    }
    if s.contains("axum") || s.contains("rust api") || s.contains("rust server") {
        return Some("rust-api");
    }
    if s.contains("migration") || s.contains("schema ya sql") {
        return Some("sql-migrations");
    }
    if s.contains("shell script") || s.contains("bash script") || s.contains("scripts za shell") {
        return Some("shell-scripts");
    }
    if s.contains("java api") || s.contains("spring") {
        return Some("java-api");
    }
    if s.contains("php api") || s.contains("php controller") {
        return Some("php-api");
    }
    if s.contains("ruby api") || s.contains("sinatra") {
        return Some("ruby-api");
    }
    if s.contains("kotlin api") || s.contains("ktor") {
        return Some("kotlin-api");
    }
    if s.contains("swift api") || s.contains("vapor") {
        return Some("swift-api");
    }
    if s.contains("c modules") || s.contains("c api") {
        return Some("c-crud");
    }
    if s.contains("cpp") || s.contains("c++") {
        return Some("cpp-crud");
    }
    if s.contains("csharp") || s.contains("c#") || s.contains("dotnet") {
        return Some("csharp-api");
    }
    if s.contains("html pages") || s.contains("kurasa za html") || s.contains("static pages") {
        return Some("html-pages");
    }
    if s.contains("css") || s.contains("styles") {
        return Some("css-components");
    }
    None
}

/// Zalisha faili nyingi kwa scaffold. Rudisha (files, notes).
/// `count` = idadi ya vitengo (moduli/resources); kila kimoja huzalisha faili 1-2.
pub fn generate(kind: &str, project: &str, count: usize) -> (Vec<(String, String)>, String) {
    let mut files: Vec<(String, String)> = Vec::new();
    let n = count.clamp(1, 10_000);
    match kind {
        "modules" => {
            for i in 0..n {
                let name = format!("module_{:04}", i);
                let class = format!("Module{:04}", i);
                files.push((
                    format!("lib/{}.dart", name),
                    format!(
                        "// Moduli {} (kizazi cha scaffold)\nclass {} {{\n  final String id;\n  final String name;\n\n  const {}({{required this.id, required this.name}});\n\n  factory {}.fromJson(Map<String, dynamic> j) =>\n      {}(id: j['id'] as String, name: j['name'] as String);\n\n  Map<String, dynamic> toJson() => {{'id': id, 'name': name}};\n}}\n",
                        i, class, class, class, class
                    ),
                ));
            }
            files.push((
                "lib/modules.dart".into(),
                module_barrel(n),
            ));
            (files, format!("modules {} (scaffold)", n))
        }
        "crud" => {
            for i in 0..n {
                let name = format!("resource_{:04}", i);
                let class = format!("Resource{:04}", i);
                files.push((
                    format!("src/{}.rs", name),
                    format!(
                        "//! Resource {} (kizazi cha scaffold)\nuse serde::{{Deserialize, Serialize}};\n\n#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {} {{\n    pub id: i64,\n    pub name: String,\n}}\n\nimpl {} {{\n    pub fn new(id: i64, name: impl Into<String>) -> Self {{\n        Self {{ id, name: name.into() }}\n    }}\n}}\n",
                        i, class, class
                    ),
                ));
            }
            (files, format!("crud resources {} (scaffold)", n))
        }
        // ML RUST (Candle) — struct halisi ya candle-nn, ina compile.
        "ml-rust" => {
            for i in 0..n {
                let tpl = r#"//! Modeli __IDX__ (kizazi cha scaffold — Candle).
use candle_core::{Result, Tensor};
use candle_nn::{linear, Linear, Module, VarBuilder};

/// Mtandao mwepesi wa mfano (linear).
pub struct Model__IDX__ {
    linear: Linear,
}

impl Model__IDX__ {
    pub fn new(in_dim: usize, out_dim: usize, vb: VarBuilder) -> Result<Self> {
        let linear = linear(in_dim, out_dim, vb.pp("linear"))?;
        Ok(Self { linear })
    }

    pub fn forward(&self, x: &Tensor) -> Result<Tensor> {
        self.linear.forward(x)
    }
}
"#;
                files.push((
                    format!("src/model_{:04}.rs", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("Cargo.toml".into(), ML_RUST_CARGO.into()));
            (files, format!("ml-rust (candle) models {} (scaffold)", n))
        }
        // GO CRUD — package halisi ina compile.
        "go-crud" => {
            for i in 0..n {
                let tpl = r#"// Package resource__IDX__ — kizazi cha scaffold (CRUD).
package resource__IDX__

// Resource__IDX__ ni model ya CRUD.
type Resource__IDX__ struct {
	ID   int64
	Name string
}

// NewResource__IDX__ huunda Resource__IDX__ mpya.
func NewResource__IDX__(id int64, name string) Resource__IDX__ {
	return Resource__IDX__{ID: id, Name: name}
}
"#;
                files.push((
                    format!("internal/resource_{:04}/resource.go", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("go.mod".into(), "module mradi\n\ngo 1.22\n".into()));
            (files, format!("go-crud resources {} (scaffold)", n))
        }
        // ELIXIR — moduli halisi (mix project).
        "elixir-app" => {
            for i in 0..n {
                let tpl = r#"defmodule Module__IDX__ do
  @moduledoc "Moduli __IDX__ (kizazi cha scaffold)."

  @type t :: %__MODULE__{id: String.t(), name: String.t()}
  defstruct [:id, :name]

  @spec new(String.t(), String.t()) :: t()
  def new(id, name), do: %__MODULE__{id: id, name: name}
end
"#;
                files.push((
                    format!("lib/module_{:04}.ex", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("mix.exs".into(), ELIXIR_MIX_EXS.into()));
            (files, format!("elixir modules {} (scaffold)", n))
        }
        // R ANALYTICS — kazi halisi za R (base R, hakuna eval/system/setwd).
        "r-analytics" => {
            for i in 0..n {
                let tpl = r#"# Uchanganuzi __IDX__ (kizazi cha scaffold).
summarize___IDX__ <- function(values) {
  if (length(values) == 0) {
    return(list(count = 0, mean = 0, min = 0, max = 0))
  }
  list(
    count = length(values),
    mean = mean(values),
    min = min(values),
    max = max(values)
  )
}
"#;
                files.push((
                    format!("R/analysis_{:04}.R", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            let runner = format!(
                "# main.R — huingiza moduli zote na kuendesha (kizazi cha scaffold).\nfiles <- sprintf(\"R/analysis_%04d.R\", 0:{})\nfor (f in files) source(f)\nvalues <- c(12, 7, 9, 15, 3, 22, 8)\nprint(summarize_0000(values))\n",
                n.saturating_sub(1)
            );
            files.push(("main.R".into(), runner));
            files.push((
                "DESCRIPTION".into(),
                "Package: mradi\nType: Project\nTitle: Uchanganuzi wa R\nVersion: 0.1.0\n".into(),
            ));
            (files, format!("r-analytics scripts {} (scaffold)", n))
        }
        // TYPESCRIPT API (Fastify) — handlers halisi, hakuna `any`/eval.
        "ts-api" => {
            for i in 0..n {
                let tpl = r#"// Router __IDX__ (kizazi cha scaffold).
import type { FastifyInstance } from "fastify";

interface Item__IDX__ {
  id: number;
  name: string;
}

export async function register__IDX__(app: FastifyInstance): Promise<void> {
  app.get("/resource__IDX__/health", async () => ({ ok: true, module: "__IDX__" }));
  app.get<{ Params: { id: string } }>("/resource__IDX__/:id", async (req) => {
    const id = Number(req.params.id);
    const item: Item__IDX__ = { id, name: "item-" + id };
    return item;
  });
}
"#;
                files.push((
                    format!("src/controller_{:04}.ts", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("package.json".into(), TS_PKG_JSON.into()));
            files.push(("tsconfig.json".into(), TS_CONFIG_JSON.into()));
            (files, format!("ts-api (fastify) controllers {} (scaffold)", n))
        }
        // NODE API (Express) — routers halisi.
        "node-api" => {
            for i in 0..n {
                let tpl = r#"// Router __IDX__ (kizazi cha scaffold).
const express = require("express");

const router = express.Router();

router.get("/resource__IDX__/health", (req, res) => {
  res.json({ ok: true, module: "__IDX__" });
});

router.get("/resource__IDX__/:id", (req, res) => {
  const id = Number(req.params.id);
  res.json({ id, name: "item-" + id });
});

module.exports = router;
"#;
                files.push((
                    format!("src/router_{:04}.js", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("package.json".into(), NODE_PKG_JSON.into()));
            (files, format!("node-api (express) routers {} (scaffold)", n))
        }
        // RUST API (Axum) — handlers halisi (hakuna unwrap nje ya test).
        "rust-api" => {
            for i in 0..n {
                let tpl = r#"//! Handler __IDX__ (kizazi cha scaffold — Axum).
use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct Item__IDX__ {
    pub id: i64,
    pub name: String,
}

pub fn router__IDX__() -> Router {
    Router::new().route("/resource__IDX__/health", get(health__IDX__))
}

async fn health__IDX__() -> Json<Item__IDX__> {
    Json(Item__IDX__ { id: 0, name: String::from("ok") })
}
"#;
                files.push((
                    format!("src/handler_{:04}.rs", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            let mut lib_rs = String::from("//! lib.rs — moduli zote (kizazi cha scaffold).\n");
            for i in 0..n {
                lib_rs.push_str(&format!("pub mod handler_{:04};\n", i));
            }
            files.push(("src/lib.rs".into(), lib_rs));
            files.push(("Cargo.toml".into(), RUST_API_CARGO.into()));
            (files, format!("rust-api (axum) handlers {} (scaffold)", n))
        }
        // SQL MIGRATIONS — schema halisi (parameterized, hakuna concatenation).
        "sql-migrations" => {
            for i in 0..n {
                let tpl = r#"-- Migration __IDX__ (kizazi cha scaffold).
CREATE TABLE IF NOT EXISTS resource___IDX__ (
    id   BIGINT PRIMARY KEY,
    name TEXT NOT NULL
);

-- Query ya usalama (parameterized — hakuna concatenation):
--   SELECT id, name FROM resource___IDX__ WHERE id = $1;
"#;
                files.push((
                    format!("migrations/{:04}_resource.sql", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            (files, format!("sql-migrations {} (scaffold)", n))
        }
        // SHELL SCRIPTS — scripts salama (set -euo pipefail).
        "shell-scripts" => {
            for i in 0..n {
                let tpl = r#"#!/usr/bin/env bash
# Script __IDX__ (kizazi cha scaffold).
set -euo pipefail

main() {
  local target="__IDX__"
  echo "running ${target}"
}

main "$@"
"#;
                files.push((
                    format!("scripts/script_{:04}.sh", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            (files, format!("shell-scripts {} (scaffold)", n))
        }
        // JAVA API (Spring) — controllers halisi (hakuna Runtime.exec).
        "java-api" => {
            for i in 0..n {
                let tpl = r#"// Controller __IDX__ (kizazi cha scaffold).
package com.mradi.api;

public final class Controller__IDX__ {

    public String health() {
        return "ok:__IDX__";
    }
}
"#;
                files.push((
                    format!("src/main/java/com/mradi/api/Controller{:04}.java", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("build.gradle".into(), JAVA_BUILD_GRADLE.into()));
            (files, format!("java-api (spring) controllers {} (scaffold)", n))
        }
        // PHP API — controllers halisi (hakuna eval/unserialize/shell_exec).
        "php-api" => {
            for i in 0..n {
                let tpl = r#"<?php
// Controller __IDX__ (kizazi cha scaffold).
declare(strict_types=1);

final class Controller__IDX__
{
    /** @return array{ok: bool, module: string} */
    public function health(): array
    {
        return ['ok' => true, 'module' => '__IDX__'];
    }
}
"#;
                files.push((
                    format!("src/Controller{:04}.php", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("composer.json".into(), PHP_COMPOSER_JSON.into()));
            (files, format!("php-api controllers {} (scaffold)", n))
        }
        // RUBY API (Sinatra) — routes halisi (hakuna eval/YAML.load/send).
        "ruby-api" => {
            for i in 0..n {
                let tpl = r#"# Route __IDX__ (kizazi cha scaffold).
require "json"
require "sinatra/base"

class Route__IDX__ < Sinatra::Base
  get "/resource__IDX__/health" do
    { ok: true, module: "__IDX__" }.to_json
  end
end
"#;
                files.push((
                    format!("lib/route_{:04}.rb", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("Gemfile".into(), RUBY_GEMFILE.into()));
            (files, format!("ruby-api (sinatra) routes {} (scaffold)", n))
        }
        // KOTLIN API (Ktor) — routes halisi (hakuna `!!`).
        "kotlin-api" => {
            for i in 0..n {
                let tpl = r#"// Route __IDX__ (kizazi cha scaffold).
package com.mradi.api

import io.ktor.server.routing.Route

fun Route.route__IDX__() {
    // handler ya resource __IDX__
}
"#;
                files.push((
                    format!("src/main/kotlin/com/mradi/api/Route{:04}.kt", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("build.gradle.kts".into(), KOTLIN_BUILD_GRADLE.into()));
            (files, format!("kotlin-api (ktor) routes {} (scaffold)", n))
        }
        // SWIFT API (Vapor) — controllers halisi (hakuna try!/as!).
        "swift-api" => {
            for i in 0..n {
                let tpl = r#"// Controller __IDX__ (kizazi cha scaffold).
import Vapor

struct Controller__IDX__ {
    func health() -> String {
        return "ok:__IDX__"
    }
}
"#;
                files.push((
                    format!("Sources/Controller{:04}.swift", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("Package.swift".into(), SWIFT_PACKAGE.into()));
            (files, format!("swift-api (vapor) controllers {} (scaffold)", n))
        }
        // C MODULES — functions salama (strncpy, hakuna gets/strcpy).
        "c-crud" => {
            for i in 0..n {
                let tpl = r#"/* Module __IDX__ (kizazi cha scaffold). */
#include <stddef.h>
#include <string.h>

int resource___IDX___init(char *buf, size_t n, const char *name) {
    if (buf == NULL || n == 0) {
        return -1;
    }
    strncpy(buf, name, n - 1);
    buf[n - 1] = '\0';
    return 0;
}
"#;
                files.push((
                    format!("src/module_{:04}.c", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("Makefile".into(), C_MAKEFILE.into()));
            (files, format!("c-crud modules {} (scaffold)", n))
        }
        // CPP MODULES — structs salama (hakuna system()/strcpy).
        "cpp-crud" => {
            for i in 0..n {
                let tpl = r#"// Module __IDX__ (kizazi cha scaffold).
#include <string>

namespace mradi {

struct Resource__IDX__ {
    long id;
    std::string name;
};

}  // namespace mradi
"#;
                files.push((
                    format!("src/module_{:04}.cpp", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("CMakeLists.txt".into(), CPP_CMAKE.into()));
            (files, format!("cpp-crud modules {} (scaffold)", n))
        }
        // CSHARP API (ASP.NET) — controllers halisi (hakuna SHA1/MD5/shell).
        "csharp-api" => {
            for i in 0..n {
                let tpl = r#"// Controller __IDX__ (kizazi cha scaffold).
namespace Mradi.Api;

public sealed class Controller__IDX__
{
    public string Health()
    {
        return "ok:__IDX__";
    }
}
"#;
                files.push((
                    format!("src/Controller{:04}.cs", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            files.push(("mradi.csproj".into(), CSHARP_CSPROJ.into()));
            (files, format!("csharp-api (aspnet) controllers {} (scaffold)", n))
        }
        // HTML PAGES — kurasa halisi (hakuna inline script ya nje).
        "html-pages" => {
            for i in 0..n {
                let tpl = r#"<!DOCTYPE html>
<html lang="sw">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Safu __IDX__</title>
</head>
<body>
  <main id="page-__IDX__">
    <h1>Safu __IDX__</h1>
  </main>
</body>
</html>
"#;
                files.push((
                    format!("public/page_{:04}.html", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            (files, format!("html-pages {} (scaffold)", n))
        }
        // CSS COMPONENTS — styles halisi.
        "css-components" => {
            for i in 0..n {
                let tpl = r#"/* Component __IDX__ (kizazi cha scaffold). */
.component-__IDX__ {
  display: block;
  padding: 0.5rem;
  border-radius: 0.25rem;
}
"#;
                files.push((
                    format!("styles/component_{:04}.css", i),
                    tpl.replace("__IDX__", &format!("{:04}", i)),
                ));
            }
            (files, format!("css-components {} (scaffold)", n))
        }
        _ => (files, "generator haijulikani".into()),
    }
}

fn module_barrel(n: usize) -> String {
    let mut s = String::from("// Barrel ya moduli zote (kizazi cha scaffold)\n");
    for i in 0..n {
        s.push_str(&format!("export 'module_{:04}.dart';\n", i));
    }
    s
}

/// Cargo.toml ya mradi wa Candle (ml-rust).
const ML_RUST_CARGO: &str = r#"[package]
name = "mradi-ml"
version = "0.1.0"
edition = "2021"

[dependencies]
candle-core = "0.8"
candle-nn = "0.8"
anyhow = "1"
"#;

/// mix.exs ya mradi wa Elixir.
const ELIXIR_MIX_EXS: &str = r#"defmodule Mradi.MixProject do
  use Mix.Project

  def project do
    [
      app: :mradi,
      version: "0.1.0",
      elixir: "~> 1.15",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [extra_applications: [:logger]]
  end

  defp deps, do: []
end
"#;

/// package.json ya TypeScript API (ts-api).
const TS_PKG_JSON: &str = r#"{
  "name": "mradi-api",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "dependencies": { "fastify": "^4.26.0" },
  "devDependencies": { "typescript": "^5.4.0" }
}
"#;

/// tsconfig.json ya TypeScript API.
const TS_CONFIG_JSON: &str = r#"{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "strict": true,
    "noImplicitAny": true,
    "outDir": "dist"
  },
  "include": ["src"]
}
"#;

/// package.json ya Node API (node-api).
const NODE_PKG_JSON: &str = r#"{
  "name": "mradi-node",
  "version": "0.1.0",
  "private": true,
  "dependencies": { "express": "^4.19.0" }
}
"#;

/// Cargo.toml ya Rust API (rust-api).
const RUST_API_CARGO: &str = r#"[package]
name = "mradi-api"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
"#;

/// build.gradle ya Java API (java-api).
const JAVA_BUILD_GRADLE: &str = r#"plugins {
    id 'java'
}

group = 'com.mradi'
version = '0.1.0'

java {
    toolchain { languageVersion = JavaLanguageVersion.of(21) }
}

repositories { mavenCentral() }
"#;

/// composer.json ya PHP API (php-api).
const PHP_COMPOSER_JSON: &str = r#"{
  "name": "mradi/api",
  "type": "project",
  "require": { "php": ">=8.2" },
  "autoload": { "psr-4": { "Mradi\\": "src/" } }
}
"#;

/// Gemfile ya Ruby API (ruby-api).
const RUBY_GEMFILE: &str = r#"source "https://rubygems.org"

gem "sinatra", "~> 4.0"
gem "json", "~> 2.7"
"#;

/// build.gradle.kts ya Kotlin API (kotlin-api).
const KOTLIN_BUILD_GRADLE: &str = r#"plugins {
    kotlin("jvm") version "1.9.22"
}

group = "com.mradi"
version = "0.1.0"

repositories { mavenCentral() }

dependencies {
    implementation("io.ktor:ktor-server-core:2.3.8")
}
"#;

/// Package.swift ya Swift API (swift-api).
const SWIFT_PACKAGE: &str = r#"// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "mradi",
    platforms: [.macOS(.v13)],
    dependencies: [.package(url: "https://github.com/vapor/vapor.git", from: "4.92.0")],
    targets: [.executableTarget(name: "mradi", dependencies: [.product(name: "Vapor", package: "vapor")])]
)
"#;

/// Makefile ya C modules (c-crud).
const C_MAKEFILE: &str = r#"CC ?= cc
CFLAGS ?= -std=c11 -Wall -Wextra -O2

SRC := $(wildcard src/*.c)
OBJ := $(SRC:.c=.o)

all: libmradi.a

libmradi.a: $(OBJ)
	ar rcs $@ $(OBJ)

clean:
	rm -f $(OBJ) libmradi.a

.PHONY: all clean
"#;

/// CMakeLists.txt ya C++ modules (cpp-crud).
const CPP_CMAKE: &str = r#"cmake_minimum_required(VERSION 3.20)
project(mradi CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

file(GLOB MODULES CONFIGURE_DEPENDS "src/*.cpp")
add_library(mradi STATIC ${MODULES})
"#;

/// mradi.csproj ya C# API (csharp-api).
const CSHARP_CSPROJ: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <Nullable>enable</Nullable>
    <ImplicitUsings>enable</ImplicitUsings>
  </PropertyGroup>
</Project>
"#;

/// Meta ya generator (kwa UI/blueprint).
pub fn describe(kind: &str) -> Value {
    match kind {
        "modules" => json!({ "kind": "modules", "lang": "dart", "per_unit": 1 }),
        "rust-3d" => json!({ "kind": "rust-3d", "lang": "rust", "per_unit": 1, "framework": "std-only software rasterizer", "production": true }),
        "crud" => json!({ "kind": "crud", "lang": "rust", "per_unit": 1 }),
        "ml-rust" => json!({ "kind": "ml-rust", "lang": "rust", "per_unit": 1, "framework": "candle" }),
        "go-crud" => json!({ "kind": "go-crud", "lang": "go", "per_unit": 1 }),
        "elixir-app" => json!({ "kind": "elixir-app", "lang": "elixir", "per_unit": 1 }),
        "r-analytics" => json!({ "kind": "r-analytics", "lang": "r", "per_unit": 1 }),
        "ts-api" => json!({ "kind": "ts-api", "lang": "typescript", "per_unit": 1, "framework": "fastify" }),
        "node-api" => json!({ "kind": "node-api", "lang": "javascript", "per_unit": 1, "framework": "express" }),
        "rust-api" => json!({ "kind": "rust-api", "lang": "rust", "per_unit": 1, "framework": "axum" }),
        "sql-migrations" => json!({ "kind": "sql-migrations", "lang": "sql", "per_unit": 1 }),
        "shell-scripts" => json!({ "kind": "shell-scripts", "lang": "shell", "per_unit": 1 }),
        "java-api" => json!({ "kind": "java-api", "lang": "java", "per_unit": 1, "framework": "spring" }),
        "php-api" => json!({ "kind": "php-api", "lang": "php", "per_unit": 1 }),
        "ruby-api" => json!({ "kind": "ruby-api", "lang": "ruby", "per_unit": 1, "framework": "sinatra" }),
        "kotlin-api" => json!({ "kind": "kotlin-api", "lang": "kotlin", "per_unit": 1, "framework": "ktor" }),
        "swift-api" => json!({ "kind": "swift-api", "lang": "swift", "per_unit": 1, "framework": "vapor" }),
        "c-crud" => json!({ "kind": "c-crud", "lang": "c", "per_unit": 1 }),
        "cpp-crud" => json!({ "kind": "cpp-crud", "lang": "cpp", "per_unit": 1 }),
        "csharp-api" => json!({ "kind": "csharp-api", "lang": "csharp", "per_unit": 1, "framework": "aspnet" }),
        "html-pages" => json!({ "kind": "html-pages", "lang": "html", "per_unit": 1 }),
        "css-components" => json!({ "kind": "css-components", "lang": "css", "per_unit": 1 }),
        _ => json!({ "kind": kind }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_inatambua_maneno() {
        assert_eq!(detect("nahitaji moduli nyingi za app"), Some("modules"));
        assert_eq!(detect("tengeneza CRUD nyingi"), Some("crud"));
        assert_eq!(detect("tovuti ndogo"), None);
    }

    #[test]
    fn modules_huzalisha_faili_na_barrel() {
        let (files, note) = generate("modules", "proj", 10);
        assert_eq!(files.len(), 11, "moduli 10 + barrel 1");
        assert!(note.contains("modules"));
        assert!(files.iter().all(|(_, c)| !c.trim().is_empty()), "hakuna faili tupu");
        let barrel = files.iter().find(|(p, _)| p == "lib/modules.dart").unwrap();
        assert!(barrel.1.contains("export 'module_0000.dart';"));
        assert!(barrel.1.contains("export 'module_0009.dart';"));
    }

    #[test]
    fn crud_huzalisha_struct_halisi() {
        let (files, _) = generate("crud", "proj", 3);
        assert_eq!(files.len(), 3);
        assert!(files[0].1.contains("pub struct Resource0000"));
        assert!(files[0].1.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"));
    }

    #[test]
    fn count_inaclamp() {
        let (files, _) = generate("modules", "proj", 0);
        assert_eq!(files.len(), 2, "count 0 → 1 moduli + barrel");
        let (files, _) = generate("modules", "proj", 999_999);
        assert_eq!(files.len(), 10_001, "clamp juu 10_000");
    }

    #[test]
    fn detect_ml_elixir_r_go() {
        assert_eq!(detect("candle rust model"), Some("ml-rust"));
        assert_eq!(detect("elixir phoenix api"), Some("elixir-app"));
        assert_eq!(detect("r analytics scripts"), Some("r-analytics"));
        assert_eq!(detect("golang api server"), Some("go-crud"));
    }

    #[test]
    fn ml_rust_huzalisha_candle_halisi() {
        let (files, _) = generate("ml-rust", "proj", 1);
        let m = files.iter().find(|(p, _)| p == "src/model_0000.rs").unwrap();
        assert!(m.1.contains("use candle_core"));
        assert!(m.1.contains("pub struct Model0000"));
        let cargo = files.iter().find(|(p, _)| p == "Cargo.toml").unwrap();
        assert!(cargo.1.contains("candle-core"));
    }

    #[test]
    fn elixir_huzalisha_defmodule_na_mix() {
        let (files, _) = generate("elixir-app", "proj", 2);
        let m = files.iter().find(|(p, _)| p == "lib/module_0001.ex").unwrap();
        assert!(m.1.contains("defmodule Module0001 do"));
        let mix = files.iter().find(|(p, _)| p == "mix.exs").unwrap();
        assert!(mix.1.contains("Mix.Project"));
    }

    #[test]
    fn r_analytics_huzalisha_kazi_halisi() {
        let (files, _) = generate("r-analytics", "proj", 3);
        let m = files.iter().find(|(p, _)| p == "R/analysis_0002.R").unwrap();
        assert!(m.1.contains("summarize_0002 <- function"));
        assert!(m.1.contains("mean(values)"));
        let main = files.iter().find(|(p, _)| p == "main.R").unwrap();
        assert!(main.1.contains("sprintf(\"R/analysis_%04d.R\", 0:2)"));
    }

    #[test]
    fn go_crud_huzalisha_package_halisi() {
        let (files, _) = generate("go-crud", "proj", 2);
        let m = files.iter().find(|(p, _)| p == "internal/resource_0000/resource.go").unwrap();
        assert!(m.1.contains("package resource0000"));
        assert!(m.1.contains("type Resource0000 struct"));
    }

    #[test]
    fn generators_mpya_zote_huzalisha_faili_halisi() {
        let kinds = [
            "ts-api", "node-api", "rust-api", "sql-migrations", "shell-scripts", "java-api",
            "php-api", "ruby-api", "kotlin-api", "swift-api", "c-crud", "cpp-crud",
            "csharp-api", "html-pages", "css-components",
        ];
        for k in kinds {
            let (files, note) = generate(k, "proj", 3);
            assert!(files.len() >= 3, "{} inapaswa kuzalisha >=3 faili", k);
            assert!(files.iter().all(|(_, c)| !c.trim().is_empty()), "{} ina faili tupu", k);
            assert!(note.contains(k), "{} note: {}", k, note);
            assert!(describe(k)["lang"].as_str().is_some(), "{} haina lang", k);
        }
    }

    #[test]
    fn detect_inaunga_lugha_mpya_za_api() {
        assert_eq!(detect("tengeneza typescript api"), Some("ts-api"));
        assert_eq!(detect("node api ya express"), Some("node-api"));
        assert_eq!(detect("rust api ya axum"), Some("rust-api"));
        assert_eq!(detect("sql migration za schema"), Some("sql-migrations"));
        assert_eq!(detect("bash script nyingi"), Some("shell-scripts"));
        assert_eq!(detect("java api ya spring"), Some("java-api"));
        assert_eq!(detect("php api controllers"), Some("php-api"));
        assert_eq!(detect("ruby api ya sinatra"), Some("ruby-api"));
        assert_eq!(detect("kotlin api ya ktor"), Some("kotlin-api"));
        assert_eq!(detect("swift api ya vapor"), Some("swift-api"));
        assert_eq!(detect("c modules"), Some("c-crud"));
        assert_eq!(detect("cpp modules"), Some("cpp-crud"));
        assert_eq!(detect("csharp api ya dotnet"), Some("csharp-api"));
        assert_eq!(detect("html pages nyingi"), Some("html-pages"));
        assert_eq!(detect("css components"), Some("css-components"));
    }

    #[test]
    fn generators_mpya_hupita_linter_ya_repo() {
        // Generators mpya tu (templates za awali zina print() ya loss — output halisi).
        let kinds = [
            "ts-api", "node-api", "rust-api", "sql-migrations", "shell-scripts", "java-api",
            "php-api", "ruby-api", "kotlin-api", "swift-api", "c-crud", "cpp-crud",
            "csharp-api", "html-pages", "css-components",
        ];
        let mut violations = 0;
        for k in kinds {
            let (files, _) = generate(k, "proj", 2);
            for (p, c) in &files {
                let lang = crate::code_audit::lang_for(p);
                let v = crate::code_audit::analyze_content(lang, c);
                if !v.is_empty() {
                    violations += v.len();
                    println!("VIOLATION kind={} path={} lang={} -> {:?}", k, p, lang, v);
                }
            }
        }
        assert_eq!(violations, 0, "kodi mpya inavunja kanuni za repo");
    }
}

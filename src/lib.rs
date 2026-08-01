//! Python bindings for the kaiv format — the same core crate the
//! CLI and the browser wasm wrap, surfaced pythonically: strings
//! in, strings (or bytes) out, exceptions instead of envelopes.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(kaiv, KaivError, PyException, "A kaiv pipeline error.");
create_exception!(
    kaiv,
    ValidationError,
    KaivError,
    "Canonical data failed schema validation."
);

fn kerr(e: impl std::fmt::Display) -> PyErr {
    KaivError::new_err(e.to_string())
}

/// The bindings (and wrapped core) version.
#[pyfunction]
fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Authored `.kaiv` -> relational `.raiv`.
#[pyfunction]
fn compile(text: &str) -> PyResult<String> {
    ::kaiv::compile(text.as_bytes()).map_err(kerr)
}

/// Relational `.raiv` -> canonical `.daiv` (reference resolution;
/// schema-declared documents also materialize absent fields).
#[pyfunction]
fn denormalize(raiv: &str) -> PyResult<String> {
    let r = ::kaiv::Resolver::offline();
    ::kaiv::denormalize_with(raiv, &r).map_err(kerr)
}

/// Authored `.kaiv` -> canonical `.daiv` (compile + denormalize).
/// An authored `.saiv` may ride along: it is preloaded under its
/// declared name, and a document that does not already carry a
/// `.!schema` declaration gets one injected — the schema-aware
/// build (defaults, head-type lifts, unit conversion) works
/// offline, and the call applies the schema it was handed.
#[pyfunction]
#[pyo3(signature = (text, schema=None))]
fn build(text: &str, schema: Option<&str>) -> PyResult<String> {
    let r = ::kaiv::Resolver::offline();
    let mut doc = text.to_string();
    if let Some(saiv) = schema {
        let name = saiv_name(saiv)?;
        r.preload(&name, "saiv", saiv.as_bytes().to_vec());
        let csaiv = ::kaiv::compile_schema_with(saiv.as_bytes(), &r).map_err(kerr)?;
        r.preload(&name, "csaiv", csaiv.into_bytes());
        let declares = doc
            .lines()
            .any(|l| l.trim_start_matches([' ', '\t']).starts_with(".!schema"));
        if !declares {
            let decl = format!(".!schema:{name}\n");
            if doc.trim_start_matches([' ', '\t']).starts_with(".!kaiv") {
                let eol = doc.find('\n').map(|i| i + 1).unwrap_or(doc.len());
                doc.insert_str(eol, &decl);
            } else {
                doc.insert_str(0, &decl);
            }
        }
    }
    ::kaiv::compile_with(doc.as_bytes(), &r)
        .and_then(|raiv| ::kaiv::denormalize_with(&raiv, &r))
        .map_err(kerr)
}

/// The schema's declared identity from its `.!saiv` header.
fn saiv_name(saiv: &str) -> PyResult<String> {
    let head = saiv.lines().next().unwrap_or_default();
    let rest = head
        .strip_prefix(".!saiv")
        .ok_or_else(|| kerr("schema must open with a .!saiv declaration"))?;
    rest.split_ascii_whitespace()
        .find(|t| !t.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .map(str::to_string)
        .ok_or_else(|| kerr("the .!saiv declaration names no schema"))
}

/// Authored `.saiv` -> compiled `.csaiv` (the validation contract).
#[pyfunction]
fn compile_schema(saiv: &str) -> PyResult<String> {
    let r = ::kaiv::Resolver::offline();
    ::kaiv::compile_schema_with(saiv.as_bytes(), &r).map_err(kerr)
}

/// Validate canonical `.daiv` against a schema — authored `.saiv`
/// or compiled `.csaiv`, told apart by the header. Returns None on
/// pass; raises ValidationError on a validation verdict and
/// KaivError when the schema itself won't compile or parse.
#[pyfunction]
fn validate(daiv: &str, schema: &str) -> PyResult<()> {
    let header = schema.lines().next().unwrap_or_default();
    let csaiv: String = if header.trim_start().starts_with(".!csaiv") {
        schema.to_string()
    } else {
        let r = ::kaiv::Resolver::offline();
        ::kaiv::compile_schema_with(schema.as_bytes(), &r)
            .map_err(|e| kerr(format!("schema: {e}")))?
    };
    let compiled = ::kaiv::parse_csaiv(&csaiv).map_err(|e| kerr(format!("schema: {e}")))?;
    ::kaiv::validate(daiv, &compiled).map_err(|e| ValidationError::new_err(e.to_string()))
}

/// The standard formatter: canonical spelling, authored intent kept.
#[pyfunction]
fn fmt(text: &str) -> PyResult<String> {
    ::kaiv::format_data(text).map_err(kerr)
}

/// Infer an authored `.saiv` schema from a document.
#[pyfunction]
#[pyo3(signature = (text, name=""))]
fn infer(text: &str, name: &str) -> PyResult<String> {
    let r = ::kaiv::Resolver::offline();
    let daiv = ::kaiv::compile_with(text.as_bytes(), &r)
        .and_then(|raiv| ::kaiv::denormalize_with(&raiv, &r))
        .map_err(kerr)?;
    ::kaiv::infer::infer(&daiv, name).map_err(kerr)
}

/// Foreign text format -> authored `.kaiv`.
/// Formats: json | yaml | toml | xml.
#[pyfunction]
fn import_from(format: &str, text: &str) -> PyResult<String> {
    let b = text.as_bytes();
    match format {
        "json" => ::kaiv::json::import(b),
        "yaml" => ::kaiv::yaml::import(b),
        "toml" => ::kaiv::toml::import(b),
        "xml" => ::kaiv::xml::import(b),
        other => return Err(kerr(format!("unsupported import format: {other}"))),
    }
    .map_err(kerr)
}

/// Canonical `.daiv` -> foreign format. Text formats return str;
/// the binary formats (cbor, avro, asn1) return bytes.
#[pyfunction]
fn export_to(py: Python<'_>, format: &str, daiv: &str) -> PyResult<Py<PyAny>> {
    use pyo3::types::{PyBytes, PyString};
    let text = |s: String| PyString::new(py, &s).into_any().unbind();
    let bin = |b: Vec<u8>| PyBytes::new(py, &b).into_any().unbind();
    match format {
        "json" => ::kaiv::json::export(daiv).map(text),
        "yaml" => ::kaiv::yaml::export(daiv).map(text),
        "toml" => ::kaiv::toml::export(daiv).map(text),
        "xml" => ::kaiv::xml::export(daiv).map(text),
        "cbor" => ::kaiv::cbor::export(daiv).map(bin),
        "avro" => ::kaiv::avro::export(daiv).map(bin),
        "asn1" => ::kaiv::asn1::export(daiv).map(bin),
        other => return Err(kerr(format!("unsupported export format: {other}"))),
    }
    .map_err(kerr)
}

/// Foreign schema -> authored `.saiv` (sound weakening).
/// Formats: jsonschema | xsd | proto | avsc | graphql. `message`
/// picks the root when the schema declares several.
#[pyfunction]
#[pyo3(signature = (format, text, name="", message=None))]
fn import_schema(format: &str, text: &str, name: &str, message: Option<&str>) -> PyResult<String> {
    let b = text.as_bytes();
    match format {
        "jsonschema" => ::kaiv::jsonschema::import(b, name),
        "xsd" => ::kaiv::xsd::import_schema(b, message, name),
        "proto" => ::kaiv::proto::import_schema(text, message, name),
        "avsc" => ::kaiv::avro::import_schema(b, name),
        "graphql" => ::kaiv::graphql::import_schema(b, message, name),
        other => return Err(kerr(format!("unsupported schema format: {other}"))),
    }
    .map_err(kerr)
}

#[pymodule]
fn kaiv(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(version, m)?)?;
    m.add_function(wrap_pyfunction!(compile, m)?)?;
    m.add_function(wrap_pyfunction!(denormalize, m)?)?;
    m.add_function(wrap_pyfunction!(build, m)?)?;
    m.add_function(wrap_pyfunction!(compile_schema, m)?)?;
    m.add_function(wrap_pyfunction!(validate, m)?)?;
    m.add_function(wrap_pyfunction!(fmt, m)?)?;
    m.add_function(wrap_pyfunction!(infer, m)?)?;
    m.add_function(wrap_pyfunction!(import_from, m)?)?;
    m.add_function(wrap_pyfunction!(export_to, m)?)?;
    m.add_function(wrap_pyfunction!(import_schema, m)?)?;
    m.add("KaivError", py.get_type::<KaivError>())?;
    m.add("ValidationError", py.get_type::<ValidationError>())?;
    Ok(())
}

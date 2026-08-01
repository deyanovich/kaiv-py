"""Binding-level tests: each function once, plus the error model.

The core's correctness is the conformance tree in the spec repo,
run by kaiv-rs; these tests pin the Python surface — signatures,
return types, exception mapping — not the format semantics.
"""

import pytest

import kaiv


def test_version():
    assert kaiv.version() == "0.11.0"


def test_build_plain():
    daiv = kaiv.build("host=api.example.com\n!int\nport=8080\n")
    assert daiv.startswith(".!daiv\n")
    assert "!int'::port=8080" in daiv


def test_compile_then_denormalize():
    raiv = kaiv.compile("a=1\nb=$a\n")
    assert raiv.startswith(".!raiv\n")
    daiv = kaiv.denormalize(raiv)
    assert "::b=1" in daiv


def test_build_with_schema_converts_units():
    daiv = kaiv.build(
        "!float:km\n/trip::dist=42\n",
        schema=".!saiv acme/trip\n\n!float:m\n/trip::dist=\n",
    )
    assert "!float:m'/trip::dist=42000" in daiv


def test_validate_pass_and_verdict():
    schema = ".!saiv demo/server\n.!types std/net\n\n&port\nport=\n"
    kaiv.validate(kaiv.build("port=8080\n"), schema)
    with pytest.raises(kaiv.ValidationError):
        kaiv.validate(kaiv.build("port=70000\n"), schema)


def test_validate_accepts_compiled_schema():
    csaiv = kaiv.compile_schema(".!saiv acme/cfg\n\n!int\nn=\n")
    assert csaiv.startswith(".!csaiv acme/cfg\n")
    kaiv.validate(kaiv.build("!int\nn=1\n"), csaiv)


def test_fmt():
    assert kaiv.fmt(".!kaiv 1\na=1\n") == ".!kaiv\n\na=1\n"


def test_unbuild_is_builds_inverse_direction():
    daiv = kaiv.build("title=hi\n\n(/owner)\nname=Ada\n!bool\nactive=true\n()\n")
    authored = kaiv.unbuild(daiv)
    assert authored.startswith(".!kaiv\n")
    assert "(/owner)" in authored
    assert kaiv.build(authored) == daiv
    with pytest.raises(kaiv.KaivError):
        kaiv.unbuild("a=1\n")


def test_infer_declares_name():
    saiv = kaiv.infer("host=x\nport=8080\n", "demo/server")
    assert saiv.startswith(".!saiv demo/server\n")


def test_import_export_roundtrip():
    k = kaiv.import_from("json", '{"host": "x", "port": 8080}')
    daiv = kaiv.build(k)
    j = kaiv.export_to("json", daiv)
    assert isinstance(j, str) and '"port":8080' in j
    assert isinstance(kaiv.export_to("cbor", daiv), bytes)


def test_import_schema():
    js = '{"type": "object", "properties": {"n": {"type": "integer"}}}'
    saiv = kaiv.import_schema("jsonschema", js, "acme/n")
    assert saiv.startswith(".!saiv acme/n\n")


def test_errors_are_kaiv_error():
    with pytest.raises(kaiv.KaivError):
        kaiv.compile("no trailing newline")
    with pytest.raises(kaiv.KaivError):
        kaiv.export_to("bson", ".!daiv\n")
    assert issubclass(kaiv.ValidationError, kaiv.KaivError)

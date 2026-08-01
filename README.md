# kaiv — Python bindings

Python bindings for [kaiv](https://kaiv.io/), an immutable
structural type system for data at rest. The same Rust core the
[CLI](https://crates.io/crates/kaiv-cli) and the
[playground](https://demo.kaiv.io/) wrap, surfaced pythonically:
strings in, strings (or bytes) out, exceptions instead of error
envelopes.

```sh
pip install kaiv
```

## The pipeline

```python
import kaiv

# Authored .kaiv -> canonical .daiv (compile + denormalize).
daiv = kaiv.build("host=api.example.com\n!int\nport=8080\n")

# Schema-aware build: defaults materialize, head types lift,
# authored units convert (exactly, or the build fails).
daiv = kaiv.build(
    "!float:km\n/trip::dist=42\n",
    schema=".!saiv acme/trip\n\n!float:m\n/trip::dist=\n",
)
assert "/trip::dist=42000" in daiv

# Validation: None on pass, kaiv.ValidationError on a verdict.
schema = ".!saiv demo/server\n.!types std/net\n\n&port\nport=\n"
kaiv.validate(kaiv.build("port=8080\n"), schema)

# Schema inference: a document to its authored .saiv.
saiv = kaiv.infer("host=x\nport=8080\n", "demo/server")

# The standard formatter.
canonical = kaiv.fmt("a  =  1\n")
```

## The format hub

```python
# Foreign text formats -> authored .kaiv.
k = kaiv.import_from("json", '{"host": "x", "port": 8080}')

# Canonical .daiv -> foreign formats. Text formats return str,
# binary formats (cbor, avro, asn1) return bytes.
j = kaiv.export_to("json", kaiv.build(k))
b = kaiv.export_to("cbor", kaiv.build(k))

# Foreign schemas -> authored .saiv (sound weakening):
# jsonschema | xsd | proto | avsc | graphql.
s = kaiv.import_schema("jsonschema", open("api.schema.json").read(),
                       "acme/api")
```

## Individual pipeline stages

```python
raiv = kaiv.compile(text)        # .kaiv -> .raiv (relational)
daiv = kaiv.denormalize(raiv)    # .raiv -> .daiv (canonical)
csaiv = kaiv.compile_schema(s)   # .saiv -> .csaiv (contract)
text = kaiv.unbuild(daiv)        # .daiv/.raiv -> authored .kaiv
                                 # (a view: resolved sugar --
                                 # comments, variables, refs --
                                 # does not come back)
```

Errors raise `kaiv.KaivError`; validation verdicts raise
`kaiv.ValidationError` (a subclass). `kaiv.version()` reports the
bindings/core version.

The format is specified at
[kaiv.io/kaiv/spec/latest](https://kaiv.io/kaiv/spec/latest);
resolution here is offline (the `std/*` libraries are embedded —
registry-resolving builds belong to the CLI and `kaiv db`-class
tooling).

## License

MIT or Apache-2.0, at your option.

"""Read compiler-resolved public API edges, without copying docs or source bodies.

One graph is one target/feature configuration, never the full cross-platform API.
The nightly JSON schema is intentionally checked; an unknown format needs review.
"""
import argparse
from collections import deque
import hashlib
import json
from pathlib import Path

FORMAT = 61
EXTRACTOR_SHA256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()


def exports(document, source_root):
    if document.get("format_version") != FORMAT:
        raise ValueError(f"review rustdoc format {document.get('format_version')}; expected {FORMAT}")
    index = document["index"]
    root_id = str(document["root"])
    local_crate = index[root_id]["crate_id"]
    pending = deque([(root_id, index[root_id]["name"], None, "root", True)])
    visited, items, edges, impls, external, signature_types = set(), [], [], [], [], []
    implicit_traits = {}
    paths = document["paths"]
    source_root = Path(source_root).resolve()

    def location(item):
        span = item.get("span")
        if not span:
            return None
        path = Path(span["filename"])
        if path.is_absolute():
            try:
                filename = path.resolve().relative_to(source_root).as_posix()
            except ValueError:
                return {"file": "${EXTERNAL_SOURCE}/" + path.name,
                        "line": span["begin"][0], "outside_reference": True}
        else:
            filename = path.as_posix()
        return {"file": filename, "line": span["begin"][0], "column": span["begin"][1]}

    def child(parent, child_id, prefix, relation, implicit_public=False, alias=None, nameable=None):
        child_id = str(child_id)
        item = index.get(child_id)
        if item is not None and not implicit_public and item["visibility"] != "public":
            return
        if item is not None and "use" in item["inner"]:
            use = item["inner"]["use"]
            name = None if use["is_glob"] else use["name"]
        else:
            name = alias or (item or paths.get(child_id, {})).get("name")
        path = f"{prefix}::{name}" if name else prefix
        pending.append((child_id, path, parent, relation, route_nameable if nameable is None else nameable))

    def api_metadata(kind, data):
        fields = {
            "function": ["sig", "generics", "header"],
            "struct": ["generics"], "union": ["generics"], "enum": ["generics"],
            "trait": ["generics", "bounds", "is_auto", "is_unsafe", "is_dyn_compatible"],
            "trait_alias": ["generics", "params"], "type_alias": ["type", "generics"],
            "assoc_type": ["type", "generics", "bounds"],
            "assoc_const": ["type"], "constant": ["type"], "static": ["type", "is_mutable"],
            "proc_macro": ["kind", "helpers"],
        }
        if kind == "struct_field":
            return {"type": data}
        return {field: data[field] for field in fields.get(kind, []) if field in data}

    def referenced_types(data):
        if isinstance(data, dict):
            if "id" in data and "path" in data and "args" in data:
                identity = str(data["id"])
                target = paths.get(identity)
                if target:
                    yield identity, target
            for value in data.values():
                yield from referenced_types(value)
        elif isinstance(data, list):
            for value in data:
                yield from referenced_types(value)

    def canonical_api(data):
        if isinstance(data, dict):
            return {key: (paths.get(str(value), {}).get("path", "unresolved") if key == "id" else canonical_api(value))
                    for key, value in data.items()}
        if isinstance(data, list):
            return [canonical_api(value) for value in data]
        return data

    while pending:
        item_id, path, parent, relation, route_nameable = pending.popleft()
        if parent is not None:
            edges.append({"from": parent, "to": item_id, "path": path, "relation": relation,
                          "path_is_nameable": route_nameable})
        if item_id in visited:
            continue
        visited.add(item_id)
        item = index.get(item_id)
        if item is None or item["crate_id"] != local_crate:
            target = paths.get(item_id)
            external.append({"rustdoc_id": item_id, "public_path": path, "target": target})
            continue
        kind, data = next(iter(item["inner"].items()))
        if kind == "use":
            use_record = {"rustdoc_id": item_id, "public_path": path,
                          "name": data["name"], "kind": kind, "source": location(item),
                          "visibility": item["visibility"], "target_id": data["id"],
                          "glob": data["is_glob"],
                          "path_is_nameable": route_nameable,
                          "target_path": paths.get(str(data["id"]), {}).get("path", data["source"])}
            use_record["profile_item_key"] = hashlib.sha256(json.dumps(
                [path, kind, use_record["source"], use_record["target_path"]], sort_keys=True).encode()).hexdigest()[:24]
            items.append(use_record)
            if data["id"] is not None:
                target_id = str(data["id"])
                target = index.get(target_id)
                if data["is_glob"] and target and "module" in target["inner"]:
                    for member in target["inner"]["module"]["items"]:
                        child(item_id, member, path, "glob_member")
                elif data["is_glob"] and target and "enum" in target["inner"]:
                    for variant in target["inner"]["enum"]["variants"]:
                        child(item_id, variant, path, "glob_variant", True)
                elif data["is_glob"] and (not target or target["crate_id"] != local_crate):
                    external.append({"rustdoc_id": target_id, "public_path": path,
                                     "target": paths.get(target_id), "glob": True})
                else:
                    child(item_id, target_id, path.rsplit("::", 1)[0], "reexport", True, data["name"])
            else:
                external.append({"rustdoc_id": item_id, "public_path": path,
                                 "unresolved_source": data["source"]})
            continue
        record = {"rustdoc_id": item_id, "public_path": path, "name": item["name"],
                  "kind": kind, "visibility": item["visibility"], "source": location(item),
                  "path_is_nameable": route_nameable, "discovery_relation": relation}
        # Attr strings can contain implementation-bearing macros. Keep only
        # machine-readable availability metadata, never entire attr/source text.
        record["deprecated"] = item.get("deprecation") is not None
        record["api"] = api_metadata(kind, data)
        record["api_fingerprint"] = hashlib.sha256(json.dumps(canonical_api(record["api"]), sort_keys=True).encode()).hexdigest()
        record["profile_item_key"] = hashlib.sha256(json.dumps(
            [path, kind, record["source"], record["api_fingerprint"]], sort_keys=True).encode()).hexdigest()[:24]
        for identity, target in referenced_types(record["api"]):
            if target["crate_id"] != local_crate:
                signature_types.append({"from": item_id, "to": identity, "target": target})
            elif identity in index:
                target_path = target["path"]
                child(item_id, identity, "::".join(target_path[:-1]), "signature_type",
                      False, target_path[-1], False)
            else:
                signature_types.append({"from": item_id, "to": identity, "target": target,
                                        "unresolved_local_type": True})
        items.append(record)
        if kind == "extern_crate":
            external.append({"rustdoc_id": item_id, "public_path": path,
                             "crate_name": data["name"]})
        if kind == "module":
            for member in data["items"]:
                child(item_id, member, path, "module_member")
        elif kind in ("struct", "union", "enum"):
            fields = []
            if kind == "struct" and isinstance(data["kind"], dict):
                if "plain" in data["kind"]:
                    fields = data["kind"]["plain"]["fields"]
                elif "tuple" in data["kind"]:
                    fields = [i for i in data["kind"]["tuple"] if i is not None]
            elif kind == "union":
                fields = data["fields"]
            for field in fields:
                child(item_id, field, path, "field")
            if kind == "enum":
                for variant in data["variants"]:
                    child(item_id, variant, path, "variant", True)
            for implementation in data["impls"]:
                impl_item = index[str(implementation)]
                implementation_data = impl_item["inner"]["impl"]
                trait = implementation_data["trait"]
                if trait is None:
                    for member in implementation_data["items"]:
                        child(item_id, member, path, "inherent_member")
                else:
                    synthetic = implementation_data["is_synthetic"]
                    blanket = implementation_data["blanket_impl"] is not None
                    impl_source = location(impl_item)
                    # A macro can point its span outside the invoking crate;
                    # that location alone does not make its impl external.
                    external_impl = impl_item["crate_id"] != local_crate
                    trait_path = paths.get(str(trait["id"]), {}).get("path", [trait["path"]])
                    # rustdoc instantiates external standard blankets with a local
                    # crate_id and no span. A foreign std trait for bare generic T
                    # cannot originate here under Rust's orphan rules. Retain
                    # local-trait and local-wrapper generic impls for review.
                    standard_generic = (impl_source is None and blanket
                                        and "generic" in implementation_data["blanket_impl"]
                                        and trait_path[0] in ("core", "std", "alloc"))
                    if synthetic or (blanket and external_impl) or standard_generic:
                        category = ("synthetic:" if synthetic else "external_blanket:" if external_impl else "standard_blanket:") + "::".join(trait_path)
                        implicit_traits[category] = implicit_traits.get(category, 0) + 1
                        continue
                    impl_record = {"for": item_id, "trait": trait,
                                   "source": impl_source, "synthetic": False, "blanket": blanket,
                                   "negative": implementation_data["is_negative"],
                                   "generics": implementation_data.get("generics"),
                                   "member_ids": implementation_data["items"]}
                    impl_record["profile_impl_key"] = hashlib.sha256(json.dumps(
                        [path, impl_source, canonical_api(trait), canonical_api(impl_record["generics"])], sort_keys=True).encode()).hexdigest()[:24]
                    impls.append(impl_record)
                    trait_item = index.get(str(trait["id"]))
                    if trait_item is None or trait_item["visibility"] == "public":
                        for member in implementation_data["items"]:
                            child(item_id, member, f"<{path} as {trait['path']}>", "trait_impl_member", True, nameable=False)
        elif kind in ("trait", "trait_alias"):
            for member in data.get("items", []):
                child(item_id, member, path, "trait_member", True)
        elif kind == "variant":
            variant = data["kind"]
            if isinstance(variant, dict):
                fields = variant.get("tuple", variant.get("struct", {}).get("fields", []))
                for field in fields:
                    if field is not None:
                        child(item_id, field, path, "variant_field", True)
    return {"schema": 1, "status": "compiled_profile_only_not_full_scope",
            "rustdoc_format": FORMAT, "root": root_id,
            "items": sorted(items, key=lambda i: (i["public_path"], i["rustdoc_id"])),
            "edges": sorted(edges, key=lambda i: (i["path"], i["from"], i["to"])),
            "trait_implementations": impls, "external_exports": external,
            "implicit_trait_summary": dict(sorted(implicit_traits.items())),
            "external_signature_types": sorted({json.dumps(i, sort_keys=True): i for i in signature_types}.values(), key=lambda i: (i["from"], i["to"])),
            "limits": ["Only this compiler/target/feature configuration; absent declarations are not automatically private",
                       "Reexport edges preserve aliases without recursively enumerating infinitely cyclic paths",
                       "Representative paths reached only through signatures can be unnameable; path_is_nameable annotates that route",
                       "External exports need other crate graphs; synthetic/blanket impls are classified separately",
                       "No source declaration is promoted to implemented from compiler reachability alone"]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    raw = args.input.read_bytes()
    result = exports(json.loads(raw), args.source_root)
    result["raw_rustdoc_sha256"] = hashlib.sha256(raw).hexdigest()
    result["extractor_sha256"] = EXTRACTOR_SHA256
    args.output.write_text(json.dumps(result, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(f"{len(result['items'])} reachable local items; {len(result['external_exports'])} external export edges")


if __name__ == "__main__":
    main()

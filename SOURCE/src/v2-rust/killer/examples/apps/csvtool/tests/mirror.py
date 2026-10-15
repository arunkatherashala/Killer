"""Python mirror of `csvtool summary`: recomputes the report with the csv module and compares it
with the JSON file written by the Killer tool.

usage: python mirror.py FILE.csv GROUP VALUE REPORT.json    prints MATCH or the first difference
"""
import csv
import json
import sys


def main(path, group_col, value_col, report_path):
    with open(path, encoding="utf-8", newline="") as f:
        rows = list(csv.DictReader(f))
    groups = {}
    total = 0.0
    for r in rows:
        g = r[group_col] or "(none)"
        x = float(r[value_col])
        e = groups.setdefault(g, {"count": 0, "sum": 0.0, "min": x, "max": x})
        e["count"] += 1
        e["sum"] += x
        e["min"] = min(e["min"], x)
        e["max"] = max(e["max"], x)
        total += x
    expected = sorted(groups.items(), key=lambda kv: (-kv[1]["sum"], kv[0]))
    with open(report_path, encoding="utf-8") as f:
        report = json.load(f)

    def close(a, b):
        return abs(a - b) <= 0.0101

    if report["rows"] != len(rows):
        return f"rows differ: {report['rows']} vs {len(rows)}"
    if not close(report["total"], total):
        return f"total differs: {report['total']} vs {total}"
    if [g["group"] for g in report["groups"]] != [k for k, _ in expected]:
        return f"group order differs: {[g['group'] for g in report['groups']]} vs {[k for k, _ in expected]}"
    for g, (name, e) in zip(report["groups"], expected):
        for field, want in (("count", e["count"]), ("sum", e["sum"]), ("avg", e["sum"] / e["count"]), ("min", e["min"]), ("max", e["max"])):
            if not close(g[field], want):
                return f"{name}.{field}: {g[field]} vs {want}"
    return "MATCH"


if __name__ == "__main__":
    print(main(*sys.argv[1:5]))
